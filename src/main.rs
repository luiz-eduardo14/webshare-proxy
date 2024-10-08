use hyper::{Body, Client, Request, Response, Server, Uri};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use hyper::client::HttpConnector;
use hyper_proxy::{Intercept, Proxy, ProxyConnector};
use lazy_static::lazy_static;
use rand::prelude::SliceRandom;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::proxy_api::refresh_proxies_api;

lazy_static! {
    pub static ref PROXIES_API: Mutex<Vec<String>> = {
        Mutex::new(Vec::new())
    };
}

mod proxy_api;

async fn proxy(mut req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let proxy_connector = {
        let proxies_mutex = PROXIES_API.lock().await;
        let proxies = proxies_mutex.clone();
        drop(proxies_mutex);
        let proxy_element = proxies.choose(&mut rand::thread_rng()).unwrap().clone();
        let proxy_uri: Uri = proxy_element.parse().unwrap();
        let proxy = Proxy::new(Intercept::All, proxy_uri);
        let connector = HttpConnector::new();
        let proxy_connector = ProxyConnector::from_proxy(connector, proxy).unwrap();
        proxy_connector
    };
    let client = Client::builder().build(proxy_connector);
    // Build the request to forward
    let uri_string = format!("https://{}{}", req.uri().host().unwrap(), req.uri().path());
    let uri: Uri = uri_string.parse().unwrap();

    // Create a new request
    let mut new_req = Request::builder()
        .method(req.method())
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    // Forward headers
    *new_req.headers_mut() = req.headers().clone();

    // Send the request
    match client.request(new_req).await {
        Ok(response) => Ok(response),
        Err(_) => Ok(Response::builder()
            .status(500)
            .body(Body::from("Internal Server Error"))
            .unwrap()),
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    // Define the address to listen on
    let addr = ([127, 0, 0, 1], 3000).into();

    // Create a service
    let make_svc = make_service_fn(|_| async { Ok::<_, Infallible>(service_fn(proxy)) });

    // Create and run the server
    let server = Server::bind(&addr).serve(make_svc);

    refresh_proxies_api().await;

    println!("Listening on https://{}", addr);

    tokio::spawn(async {
        let sched = JobScheduler::new().await.unwrap();
        sched.add(
            Job::new_async("0 0 0 * * *", |uuid, _l| {
                Box::pin(async move {
                    refresh_proxies_api().await
                })
            }).unwrap()
        ).await.unwrap();
        sched.start().await.unwrap();
    });

    // Run the server
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}