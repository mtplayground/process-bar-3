use axum::{Router, routing::get};
use dotenvy::dotenv;
use std::{env, error::Error, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::info;

async fn healthcheck() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| String::from("0.0.0.0"));
    let port = env::var("PORT").unwrap_or_else(|_| String::from("8080"));
    let address: SocketAddr = format!("{host}:{port}").parse()?;

    let app = Router::new().route("/", get(healthcheck));

    info!("starting server on {}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
