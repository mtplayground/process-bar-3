mod config;
mod controllers;
mod db;
mod error;
mod models;
mod routes;
mod views;

use std::{error::Error, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::info;

use config::Config;
use routes::build_router;

pub(crate) async fn healthcheck() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let address: SocketAddr = config.bind_addr.parse()?;

    let app = build_router();

    info!("starting server on {}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
