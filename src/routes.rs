//! Route wiring.

use axum::{routing::get, Router};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{info, info_span, Span};

use crate::controllers::notes;
use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(notes::root_redirect))
        .route("/notes", get(notes::index).post(notes::create))
        .route("/notes/new", get(notes::new))
        .route("/notes/:id/edit", get(notes::edit))
        .route("/notes/:id", get(notes::show).put(notes::update))
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("missing");

            info_span!(
                "http_request",
                method = %request.method(),
                path = %request.uri().path(),
                request_id = %request_id
            )
        })
        .on_request(|request: &axum::http::Request<_>, _span: &Span| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("missing");

            info!(
                method = %request.method(),
                path = %request.uri().path(),
                request_id = %request_id,
                "request started"
            );
        })
        .on_response(|response: &axum::http::Response<_>, latency: std::time::Duration, _span: &Span| {
            let request_id = response
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("missing");

            info!(
                status = %response.status(),
                latency_ms = latency.as_millis(),
                request_id = %request_id,
                "request finished"
            );
        }))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .with_state(state)
}
