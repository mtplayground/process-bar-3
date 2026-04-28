//! Route wiring.

use axum::{routing::get, Router};
use tower_http::services::ServeDir;

pub fn build_router() -> Router {
    Router::new()
        .route("/", get(crate::healthcheck))
        .nest_service("/static", ServeDir::new("static"))
}
