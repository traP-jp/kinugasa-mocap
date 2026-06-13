use std::{env, net::SocketAddr};

use axum::{Router, routing::get};
use kinugasa_core::usecase::health::handlers;

const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() {
    let app = Router::new().route("/healthz", get(handlers::get_health));
    let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| DEFAULT_LISTEN_ADDR.to_owned());
    let listener = tokio::net::TcpListener::bind(parse_listen_addr(&listen_addr))
        .await
        .expect("bind recording console HTTP listener");

    axum::serve(listener, app)
        .await
        .expect("serve recording console HTTP API");
}

fn parse_listen_addr(value: &str) -> SocketAddr {
    value
        .parse()
        .expect("LISTEN_ADDR must be a socket address like 0.0.0.0:8080")
}
