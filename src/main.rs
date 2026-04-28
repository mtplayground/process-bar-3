mod config;
mod controllers;
mod db;
mod middleware;
mod routes;
mod views;

use std::{error::Error, net::SocketAddr};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::{make::Shared, Layer};
use tracing::info;
use tracing_subscriber::EnvFilter;

use config::Config;
use middleware::method_override::MethodOverrideLayer;
use routes::build_router;

#[derive(Clone)]
pub(crate) struct AppState {
    #[allow(dead_code)]
    pub(crate) pool: PgPool,
    pub(crate) session_secret: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let env_filter = EnvFilter::try_new(config.rust_log.as_str())?;

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();

    let address: SocketAddr = config.bind_addr.parse()?;
    let pool = db::init_pool(&config).await?;
    db::run_migrations(&pool).await?;
    let state = AppState {
        pool,
        session_secret: config.session_secret,
    };

    let app = MethodOverrideLayer::new().layer(build_router(state));

    info!("starting server on {}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, Shared::new(app)).await?;

    Ok(())
}
