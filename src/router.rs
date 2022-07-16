//! Returns a respomse to a given request.
use futures_util::StreamExt;
use futures_util::{sink::SinkExt, stream::FuturesUnordered};
use hyper::upgrade::Upgraded;
use hyper_tungstenite::WebSocketStream;
use rayon::prelude::*;
use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};
use tokio::time;

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use hyper_tungstenite::{tungstenite::Message, HyperWebsocket};
use tokio::sync::Mutex;

use crate::configuration::Route;
use crate::route_handler;
use crate::{
    builder::{self, storage},
    configuration::{self, BuildMode, RouteMethod, WsMessageType},
    data_loader,
};

/// Start webserver using hyper
pub async fn start() {
    let mut config_initial = configuration::get_configuration().await;
    log::trace!("Config: {:?}", config_initial);
    if config_initial.host == None {
        config_initial.host = Some("127.0.0.1:8080".to_string());
    }
    let addr: Result<SocketAddr, _> = config_initial.host.as_ref().unwrap().parse();

    let conf = config_initial.clone();
    let build_mode = conf.build_mode;
    let remote = conf.remote;

    let config = Arc::new(Mutex::new(config_initial.clone()));
    let routes = Arc::new(Mutex::new(config_initial.routes));

    let config_a = Arc::clone(&config);
    let routes_a = Arc::clone(&routes);
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));

        loop {
            interval.tick().await;
            let routes = routes_a.lock().await;
            let mut config = config_a.lock().await;
            let routes = routes.to_vec();

            if config.routes != routes {
                log::debug!("Saving: {} routes", routes.len());
                config.routes = routes;

                match configuration::save_configuration(&config).await {
                    Ok(_) => log::info!("Saved routes"),
                    Err(_) => log::error!("Unable to save routes"),
                }
            }
        }
    });

    if let Ok(addr) = addr {
        let make_service = make_service_fn(move |_| {
            let routes = Arc::clone(&routes);
            let build_mode = build_mode.clone();
            let remote = remote.clone();

            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let routes = Arc::clone(&routes);
                    let build_mode = build_mode.clone();
                    let remote = remote.clone();

                    async move { check_ws(req, routes, build_mode, remote).await }
                }))
            }
        });

        log::info!("Starting http server");
        let server = Server::bind(&addr).serve(make_service);

        // Run this server for... forever!
        if let Err(e) = server.await {
            log::error!("server error: {}", e);
        }
    } else {
        log::error!("Unable to start application with an invalid host");
    }
}

/// Call data_loader or builder depending on if the route exists or not.
async fn endpoint(
    routes_a: Arc<Mutex<Vec<Route>>>,
    uri: &hyper::Uri,
    method: &hyper::Method,
    build_mode: &Option<BuildMode>,
    remote: &Option<String>,
) -> Result<Response<Body>, Infallible> {
    log::info!("{}", uri);
    let routes = Arc::clone(&routes_a);
    let mut routes = routes.lock().await.to_owned();
    let method = RouteMethod::from(method);
    let (route, parameter) = route_handler::get_route(&routes, uri.path(), &method);

    if let Some(route) = route {
        let data = data_loader::load(route, parameter);
        if let Some(data) = data.await {
            let response = Response::builder()
                .status(200)
                .header(
                    "content-type",
                    // unwrap is save here because there will never be data without a resource
                    storage::get_content_type(route.resource.as_ref().unwrap()),
                )
                .body(Body::from(data))
                .unwrap();

            Ok(response)
        } else {
            if let Some(x) = routes.iter().position(|c| c == route) {
                log::info!("Remove route because the file does not exist: {:?}", route);
                routes.remove(x);
            }

            if build_mode == &Some(BuildMode::Write) {
                builder::builder::build_response(uri, &method, &build_mode, &remote, routes_a).await
            } else {
                log::error!("Will build new route for missing file");
                let response = Response::builder().status(404).body(Body::empty()).unwrap();
                Ok(response)
            }
        }
    } else if build_mode == &Some(BuildMode::Write) {
        builder::builder::build_response(uri, &method, &build_mode, &remote, routes_a).await
    } else {
        log::info!("Resource not found and build mode disabled");
        let response = Response::builder().status(404).body(Body::empty()).unwrap();
        Ok(response)
    }
}

async fn check_ws(
    request: Request<Body>,
    routes: Arc<Mutex<Vec<Route>>>,
    build_mode: Option<BuildMode>,
    remote: Option<String>,
) -> Result<Response<Body>, Infallible> {
    let uri = request.uri().clone();
    let method = &request.method();
    if hyper_tungstenite::is_upgrade_request(&request) {
        if let Ok((response, websocket)) = hyper_tungstenite::upgrade(request, None) {
            // Spawn a task to handle the websocket connection.
            tokio::spawn(async move {
                if let Err(e) = endpoint_ws(&uri, websocket, routes, &build_mode, &remote).await {
                    log::trace!("[WS] Error in websocket connection: {}", e);
                }
            });

            // Return the response so the spawned future can continue.
            Ok(response)
        } else {
            log::error!("Unable to upgrade connection");
            let response = Response::builder().status(404).body(Body::empty()).unwrap();
            Ok(response)
        }
    } else {
        endpoint(routes, &uri, method, &build_mode, &remote).await
    }
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
async fn endpoint_ws(
    uri: &hyper::Uri,
    websocket: HyperWebsocket,
    routes_a: Arc<Mutex<Vec<Route>>>,
    build_mode: &Option<BuildMode>,
    remote: &Option<String>,
) -> Result<(), Error> {
    let routes = routes_a.clone();
    let mut routes = routes.lock().await.to_owned();
    if let (Some(route), _parameter) =
        route_handler::get_route(&routes, uri.path(), &RouteMethod::WS)
    {
        let websocket = websocket.await?;

        let (tx, rx) = tokio::sync::mpsc::channel(32);

        let startup_messges: Vec<Vec<u8>> = route
            .messages
            .par_iter()
            .filter(|c| c.kind == WsMessageType::Startup)
            .map(|c| data_loader::file_sync(c.location.as_str()))
            .filter(|c| c.is_ok())
            .map(|c| c.unwrap())
            .collect();

        for c in startup_messges {
            match std::str::from_utf8(&c) {
                Ok(m) => tx.send(Message::text(m)).await?,
                Err(_) => tx.send(Message::binary(c)).await?,
            };
        }
        let after_messages: Vec<(Duration, Vec<u8>)> = route
            .messages
            .par_iter()
            .filter(|c| c.kind == WsMessageType::After)
            .map(|c| (c.get_time(), data_loader::file_sync(c.location.as_str())))
            .filter(|c| c.0.is_some() && c.1.is_ok())
            .map(|c| (c.0.unwrap(), c.1.unwrap()))
            .collect();

        let mut tasks = FuturesUnordered::new();

        for m in after_messages {
            let tx = tx.clone();
            tasks.push(tokio::task::spawn(async move {
                tokio::time::sleep(m.0).await;

                let msg = m.1;

                match std::str::from_utf8(&msg) {
                    Ok(m) => tx.send(Message::text(m)).await.unwrap(),
                    Err(_) => tx.send(Message::binary(msg.clone())).await.unwrap(),
                }
            }));
        }

        tasks.push(tokio::task::spawn(async move {
            send_ws_messages(rx, websocket).await
        }));

        // execute all tasks
        while (tasks.next().await).is_some() {}
    } else if build_mode == &Some(BuildMode::Write) {
        if let Some(remote) = remote {
            log::trace!("Start ws build");
            let route = builder::ws::build_ws(uri, remote.to_owned(), websocket).await;
            if let Ok(route) = route {
                routes.push(route);
            }
            //configuration::save_configuration(config.to_owned()).await?;
        } else {
            log::info!(
                "There is no configuration for the url: {}, and there is no remote specified",
                &uri.to_string()
            );
        }
    } else {
        log::info!(
            "There is no configuration for the url: {}, and the build_mode is not set to Write",
            &uri.to_string()
        );
    }

    Ok(())
}

async fn send_ws_messages(
    mut rx: tokio::sync::mpsc::Receiver<Message>,
    mut websocket: WebSocketStream<Upgraded>,
) {
    while let Some(message) = rx.recv().await {
        match websocket.send(message).await {
            Ok(_) => log::trace!("Sent message"),
            Err(_) => log::error!("Failed to send message"),
        }
    }
}
