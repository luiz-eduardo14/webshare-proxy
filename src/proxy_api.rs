use std::env;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use crate::PROXIES_API;

const PROXY_API: &str = "https://proxy.webshare.io/api/v2/proxy/list/?mode=direct&page_size=1000000";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ProxyData {
    username: String,
    password: String,
    proxy_address: String,
    port: i32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ResponseData {
    count: i32,
    results: Vec<ProxyData>,
}

pub async fn refresh_proxies_api() {
    let api_token = env::var("API_TOKEN").expect("Missing API_TOKEN");
    let http_client = reqwest::Client::builder()
        .use_native_tls()
        .tls_built_in_root_certs(true)
        .build()
        .unwrap();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Authorization", format!("{}", api_token).parse().unwrap());

    let request = http_client
        .request(Method::GET, PROXY_API)
        .headers(headers)
        .build().unwrap();
    let response = http_client.execute(request).await.unwrap();

    if response.status().is_success() {
        let response_data: ResponseData = response.json().await.unwrap();
        let mut proxies_api = PROXIES_API.lock().await;
        proxies_api.clear();
        for proxy_data in response_data.results {
            let url = format!("https://{}:{}@{}:{}",
                              proxy_data.username,
                              proxy_data.password,
                              proxy_data.proxy_address,
                              proxy_data.port
            );
            proxies_api.push(url)
        }
        drop(proxies_api);
        return;
    }
    let text = response.text().await.unwrap();
    panic!("failed to execute: {}", text)
}