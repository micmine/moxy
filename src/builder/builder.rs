use std::{collections::HashMap, convert::Infallible, sync::Arc};

use hyper::{Body, Response};
use tokio::sync::Mutex;

use crate::configuration::{BuildMode, Route, RouteMethod};

use super::{request, storage};

/// The data structure that will contain all relevant data. To easily convert a request to a response
/// without doing a huge workaround.
pub struct ResourceData {
    /// HTTP method
    pub method: RouteMethod,
    /// HTTP heades
    pub headers: HashMap<String, String>,
    /// HTTP status code
    pub code: u16,
    /// HTTP body
    pub payload: Option<Vec<u8>>,
}

/// Handles unknown routes. It accomplishes that with creating HTTP request and saving the response
/// into a file. It also modifies the configuration in order to not call this function with the
/// same URL again.
pub async fn build_response(
    uri: &hyper::Uri,
    method: &RouteMethod,
    build_mode: &Option<BuildMode>,
    remote: &Option<String>,
    routes_a: Arc<Mutex<Vec<Route>>>,
) -> Result<Response<Body>, Infallible> {
    if let Some(build_mode) = build_mode {
        if let Some(remote) = remote {
            let response = request::http::fetch_http(
                &method,
                request::util::get_url(uri, &remote),
                None,
                HashMap::new(),
            )
            .await;

            match response {
                Some(response) => {
                    if let Some(body) = response.payload {
                            if response.code != 404 && build_mode == &BuildMode::Write {
                            storage::save(
                                &response.method,
                                uri.path(),
                                body.clone(),
                                &response.headers,
                                routes_a,
                            )
                            .await
                            .unwrap()
                        }

                        get_response(response.headers, response.code, Body::from(body))
                    } else {
                        get_response(response.headers, response.code, Body::empty())
                    }
                }
                None => {
                    log::error!("No response from endpoint");
                    let response = Response::builder().status(404).body(Body::empty()).unwrap();
                    Ok(response)
                }
            }
        } else {
            log::error!("Resource not found and no remove specified");
            let response = Response::builder().status(404).body(Body::empty()).unwrap();
            Ok(response)
        }
    } else {
        log::info!("Resource not found and build mode disabled");
        let response = Response::builder().status(404).body(Body::empty()).unwrap();
        Ok(response)
    }
}

/// Returns a respinse with headers and a code
pub fn get_response(
    headers: HashMap<String, String>,
    code: u16,
    body: Body,
) -> Result<Response<Body>, Infallible> {
    let mut response = Response::builder().status(code);

    for (key, value) in headers.into_iter() {
        response = response.header(key, value);
    }

    Ok(response.body(body).unwrap())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{builder::request, configuration::RouteMethod};

    #[tokio::test]
    async fn requrest_no_body() {
        let _reponse = request::http::fetch_http(
            RouteMethod::GET,
            "http://example.com".to_string(),
            None,
            HashMap::new(),
        )
        .await
        .unwrap();
    }
}
