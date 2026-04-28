//! Database wiring.

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::Config;

pub async fn init_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(config.database_url.as_str())
        .await
}
