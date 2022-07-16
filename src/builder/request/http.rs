use std::{collections::HashMap};

use hyper::body::Bytes;
use reqwest::Error;

use crate::{configuration::RouteMethod, builder::builder::ResourceData};

use super::util::{hash_map_to_header_map, header_map_to_hash_map};

/// Load data from external http source
pub async fn fetch_http(
    method: &RouteMethod,
    url: String,
    body: Option<String>,
    header: HashMap<String, String>,
) -> Option<ResourceData> {
    let client = reqwest::Client::new();
    let mut req = client.request(method.to_owned().into(), url);

    req = req.headers(hash_map_to_header_map(header));

    if let Some(body) = body {
        req = req.body(body);
    }

    let response = req.send().await;

    if let Ok(response) = response {
        return Some(ResourceData {
            method: method.clone(),
            headers: header_map_to_hash_map(response.headers()),
            code: response.status().as_u16(),
            payload: get_payload(response.bytes().await),
        });
    }

    None
}

/// Convert request body to Vec<u8>
fn get_payload(bytes: Result<Bytes, Error>) -> Option<Vec<u8>> {
    match bytes {
        Ok(bytes) => Some(bytes.to_vec()),
        Err(_) => None,
    }
}
