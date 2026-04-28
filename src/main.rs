mod config;

use axum::{routing::get, Router};
use std::{error::Error, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::info;

use config::Config;

async fn healthcheck() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let address: SocketAddr = config.bind_addr.parse()?;

    let app = Router::new().route("/", get(healthcheck));

    info!("starting server on {}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
