//! This contains the configuraton datastructures and the logic how to read and wirte it.

use hyper::Method;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{convert::TryInto, fmt::Display, io::ErrorKind, str::FromStr, time::Duration};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};

/// This represents one route that can be navigated to
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Route {
    /// HTTP method
    pub method: RouteMethod,
    /// HTTP uri
    pub path: String,
    /// File storeage location
    #[serde(default)]
    pub resource: Option<String>,
    /// Data for WS
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<WsMessage>,
}

/// A WS message with controll when it has to be sent
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WsMessage {
    /// Type of time
    pub kind: WsMessageType,
    /// This will be contveted to WsMessageTime
    #[serde(default)]
    pub time: Option<String>,
    /// File storeage location
    pub location: String,
}

impl WsMessage {
    /// get parsed WsMessageTime
    pub fn get_time(&self) -> Option<Duration> {
        if let Some(time) = &self.time {
            if let Ok(time) = time.parse::<WsMessageTime>() {
                return Some(Duration::from(time));
            }
        }

        None
    }
}

/// Time units
#[derive(Debug, PartialEq)]
pub enum WsMessageTime {
    /// 5s
    Second(usize),
    /// 5m
    Minute(usize),
    /// 5h
    Hour(usize),
    /// 5sent ;After x messages sent messages
    Sent(usize),
    /// 5recived ;After x messages recived messages
    Recived(usize),
}

impl FromStr for WsMessageTime {
    type Err = u8;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with('s') {
            let padding = 1;
            if let Ok(number) = parse_time_number(s, padding) {
                return Ok(Self::Second(number));
            }
        } else if s.ends_with('m') {
            let padding = 1;
            if let Ok(number) = parse_time_number(s, padding) {
                return Ok(Self::Minute(number));
            }
        } else if s.ends_with('h') {
            let padding = 1;
            if let Ok(number) = parse_time_number(s, padding) {
                return Ok(Self::Hour(number));
            }
        } else if s.ends_with("sent") {
            let padding = 4;
            if let Ok(number) = parse_time_number(s, padding) {
                return Ok(Self::Sent(number));
            }
        } else if s.ends_with("recived") {
            let padding = 7;
            if let Ok(number) = parse_time_number(s, padding) {
                return Ok(Self::Recived(number));
            }
        }

        Err(1)
    }
}

impl Display for WsMessageTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsMessageTime::Second(t) => write!(f, "{}s", t),
            WsMessageTime::Minute(t) => write!(f, "{}m", t),
            WsMessageTime::Hour(t) => write!(f, "{}h", t),
            WsMessageTime::Sent(t) => write!(f, "{}sent", t),
            WsMessageTime::Recived(t) => write!(f, "{}recived", t),
        }
    }
}

impl From<WsMessageTime> for Duration {
    fn from(wt: WsMessageTime) -> Self {
        let seconds = match wt {
            WsMessageTime::Second(s) => s,
            WsMessageTime::Minute(m) => 60 * m,
            WsMessageTime::Hour(h) => 60 * 60 * h,
            WsMessageTime::Sent(_) => 1,
            WsMessageTime::Recived(_) => 1,
        };

        Duration::from_secs(seconds.try_into().unwrap())
    }
}

/// This will take the number infront of a string
///
/// # Exaples
/// ```
/// let input = "3sent";
///
/// assert_eq!(parse_time_number(input, 3), 3);
/// ```
fn parse_time_number(number: &str, padding: usize) -> Result<usize, u8> {
    let padding = number.len() - padding;
    if let Some(number) = number.get(0..padding) {
        if let Ok(number) = number.parse::<usize>() {
            Ok(number)
        } else {
            log::error!("Time is invalid format. Unable to parse number: {number}");
            Err(2)
        }
    } else {
        log::error!("Time is invalid format. (To short)");
        Err(1)
    }
}

/// Type of time
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WsMessageType {
    /// When the ws connects
    Startup,
    /// Time after the ws connects
    After,
    /// Repeat
    Every,
}

/// This represents the http method that is used.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RouteMethod {
    /// HTTP GET
    GET,
    /// HTTP HEAD
    HEAD,
    /// HTTP POST
    POST,
    /// HTTP PUT
    PUT,
    /// HTTP DELETE
    DELETE,
    /// HTTP CONNECT
    CONNECT,
    /// HTTP OPTIONS
    OPTIONS,
    /// HTTP TRACE
    TRACE,
    /// HTTP PATCH
    PATCH,
    /// Websocket
    WS,
}

impl FromStr for RouteMethod {
    type Err = u8;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Self::GET),
            "HEAD" => Ok(Self::HEAD),
            "POST" => Ok(Self::POST),
            "PUT" => Ok(Self::PUT),
            "DELETE" => Ok(Self::DELETE),
            "CONNECT" => Ok(Self::CONNECT),
            "OPTIONS" => Ok(Self::OPTIONS),
            "TRACE" => Ok(Self::TRACE),
            "PATCH" => Ok(Self::PATCH),
            "WS" => Ok(Self::WS),
            _ => Err(1),
        }
    }
}

impl From<Method> for RouteMethod {
    fn from(m: hyper::Method) -> Self {
        RouteMethod::from_str(m.as_str()).unwrap()
    }
}

impl From<&Method> for RouteMethod {
    fn from(m: &hyper::Method) -> Self {
        RouteMethod::from_str(m.as_str()).unwrap()
    }
}

impl From<RouteMethod> for Method {
    fn from(route_method: RouteMethod) -> Self {
        match route_method {
            RouteMethod::GET => Method::GET,
            RouteMethod::HEAD => Method::HEAD,
            RouteMethod::POST => Method::POST,
            RouteMethod::PUT => Method::PUT,
            RouteMethod::DELETE => Method::DELETE,
            RouteMethod::CONNECT => Method::CONNECT,
            RouteMethod::OPTIONS => Method::OPTIONS,
            RouteMethod::TRACE => Method::TRACE,
            RouteMethod::PATCH => Method::PATCH,
            RouteMethod::WS => unreachable!(),
        }
    }
}

/// The configuration setting for `build_mode`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BuildMode {
    /// This specifies to not modifiy the filesystem or configuraion.
    Read,
    /// This enables to modify the filesystem and configuration when Needed.
    Write,
}

/// The datastructure for "moxy.json"
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Configuration {
    /// Specifies the port to run the http server.
    pub host: Option<String>,
    /// This url is called in `BuildMode::Write`
    pub remote: Option<String>,
    /// `BuildMode`
    pub build_mode: Option<BuildMode>,
    /// A list of all available routes.
    pub routes: Vec<Route>,
}

/// Loads the configuration from the filesystem.
pub async fn get_configuration() -> Configuration {
    load_configuration("./moxy.json".to_string()).await
}

async fn load_configuration(loaction: String) -> Configuration {
    log::info!("Load Configuration: {}", loaction);
    match fs::read_to_string(&loaction).await {
        Ok(data) => serde_json::from_str(&data).unwrap_or_else(|error| {
            log::error!("Could not load configuration file: {:?}", error);
            Configuration {
                host: Some(String::from("127.0.0.1:8080")),
                remote: Some(String::from("http://localhost")),
                build_mode: None,
                routes: vec![],
            }
        }),
        Err(e) => {
            let default_configuration = Configuration {
                host: Some(String::from("127.0.0.1:8080")),
                remote: Some(String::from("http://localhost")),
                build_mode: Some(BuildMode::Write),
                routes: vec![],
            };
            if e.kind() == ErrorKind::NotFound {
                save_configuration(&default_configuration).await.unwrap();
            }

            default_configuration
        }
    }
}

/// Save configuration to filesystem
pub async fn save_configuration(configuration: &Configuration) -> Result<(), std::io::Error> {
    let config: String = serde_json::to_string_pretty(configuration)?;
    let mut file = File::create("./moxy.json").await?;

    file.write_all(config.as_bytes()).await?;

    file.flush().await?;

    Ok(())
}
