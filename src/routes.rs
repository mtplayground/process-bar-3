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
        .route("/notes/:id", get(notes::show).put(notes::update).delete(notes::delete))
        .nest_service("/static", ServeDir::new("static"))
        .fallback(notes::not_found)
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

#[cfg(test)]
mod tests {
    use super::build_router;
    use crate::AppState;
    use axum::{
        body::{Body, to_bytes},
        http::{
            Request, StatusCode,
            header::{CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE},
        },
    };
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use tower::ServiceExt;

    #[tokio::test]
    async fn unknown_routes_render_not_found_status() {
        let pool = match PgPoolOptions::new().connect_lazy("postgres://postgres:postgres@localhost/app") {
            Ok(pool) => pool,
            Err(error) => panic!("expected lazy pool to initialize: {error}"),
        };
        let app = build_router(AppState {
            pool,
            session_secret: String::from("test-session-secret"),
        });
        let request = match Request::builder().uri("/missing").body(axum::body::Body::empty()) {
            Ok(request) => request,
            Err(error) => panic!("expected request to build: {error}"),
        };

        let response = match app.oneshot(request).await {
            Ok(response) => response,
            Err(error) => panic!("expected fallback response: {error}"),
        };

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[sqlx::test(migrations = "./migrations")]
    #[ignore = "requires a valid Postgres DATABASE_URL for sqlx::test"]
    async fn notes_crud_flow_works_against_router(pool: PgPool) {
        let app = build_router(AppState {
            pool,
            session_secret: String::from("test-session-secret"),
        });

        let create_response = request(
            app.clone(),
            build_request(
                Request::builder()
                    .method("POST")
                    .uri("/notes")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("title=First+Note&content=Original+body&tags_raw=rust%2Csqlx")),
            ),
        )
        .await;
        assert_eq!(create_response.status(), StatusCode::SEE_OTHER);

        let location = header_str(&create_response, LOCATION);
        let created_path = match location {
            Some(path) => path.to_owned(),
            None => panic!("expected create redirect location"),
        };
        let create_cookie = cookie_pair(&create_response);
        assert!(create_cookie.is_some());

        let show_response = request(
            app.clone(),
            build_request(
                Request::builder()
                    .uri(&created_path)
                    .header(COOKIE, create_cookie.unwrap_or_default())
                    .body(Body::empty()),
            ),
        )
        .await;
        assert_eq!(show_response.status(), StatusCode::OK);
        let show_body = body_text(show_response).await;
        assert!(show_body.contains("First Note"));
        assert!(show_body.contains("Original body"));
        assert!(show_body.contains("Note created successfully."));

        let list_response = request(
            app.clone(),
            build_request(Request::builder().uri("/notes").body(Body::empty())),
        )
        .await;
        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body = body_text(list_response).await;
        assert!(list_body.contains("First Note"));

        let edit_response = request(
            app.clone(),
            build_request(
                Request::builder()
                    .uri(format!("{created_path}/edit"))
                    .body(Body::empty()),
            ),
        )
        .await;
        assert_eq!(edit_response.status(), StatusCode::OK);
        let edit_body = body_text(edit_response).await;
        assert!(edit_body.contains("Original body"));
        assert!(edit_body.contains("rust, sqlx"));

        let update_response = request(
            app.clone(),
            build_request(
                Request::builder()
                    .method("PUT")
                    .uri(&created_path)
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(
                        "title=Updated+Note&content=Updated+body&tags_raw=rust%2Caxum",
                    )),
            ),
        )
        .await;
        assert_eq!(update_response.status(), StatusCode::SEE_OTHER);
        let update_cookie = cookie_pair(&update_response);
        assert!(update_cookie.is_some());

        let updated_show_response = request(
            app.clone(),
            build_request(
                Request::builder()
                    .uri(&created_path)
                    .header(COOKIE, update_cookie.unwrap_or_default())
                    .body(Body::empty()),
            ),
        )
        .await;
        assert_eq!(updated_show_response.status(), StatusCode::OK);
        let updated_show_body = body_text(updated_show_response).await;
        assert!(updated_show_body.contains("Updated Note"));
        assert!(updated_show_body.contains("Updated body"));
        assert!(updated_show_body.contains("Note updated successfully."));

        let delete_response = request(
            app.clone(),
            build_request(
                Request::builder()
                    .method("DELETE")
                    .uri(&created_path)
                    .body(Body::empty()),
            ),
        )
        .await;
        assert_eq!(delete_response.status(), StatusCode::SEE_OTHER);
        let delete_cookie = cookie_pair(&delete_response);
        assert!(delete_cookie.is_some());

        let deleted_show_response = request(
            app.clone(),
            build_request(
                Request::builder()
                    .uri(&created_path)
                    .body(Body::empty()),
            ),
        )
        .await;
        assert_eq!(deleted_show_response.status(), StatusCode::NOT_FOUND);
        let deleted_show_body = body_text(deleted_show_response).await;
        assert!(deleted_show_body.contains("The requested note does not exist."));

        let deleted_list_response = request(
            app,
            build_request(
                Request::builder()
                    .uri("/notes")
                    .header(COOKIE, delete_cookie.unwrap_or_default())
                    .body(Body::empty()),
            ),
        )
        .await;
        assert_eq!(deleted_list_response.status(), StatusCode::OK);
        let deleted_list_body = body_text(deleted_list_response).await;
        assert!(deleted_list_body.contains("No notes exist yet."));
        assert!(deleted_list_body.contains("Note deleted successfully."));
        assert!(!deleted_list_body.contains("Updated Note"));
    }

    async fn request(app: axum::Router, request: Request<Body>) -> axum::response::Response {
        match app.oneshot(request).await {
            Ok(response) => response,
            Err(error) => panic!("expected request to succeed: {error}"),
        }
    }

    fn build_request(request: Result<Request<Body>, axum::http::Error>) -> Request<Body> {
        match request {
            Ok(request) => request,
            Err(error) => panic!("expected request to build: {error}"),
        }
    }

    async fn body_text(response: axum::response::Response) -> String {
        let body = match to_bytes(response.into_body(), usize::MAX).await {
            Ok(body) => body,
            Err(error) => panic!("expected response body to be readable: {error}"),
        };

        match String::from_utf8(body.to_vec()) {
            Ok(body) => body,
            Err(error) => panic!("expected UTF-8 body: {error}"),
        }
    }

    fn cookie_pair(response: &axum::response::Response) -> Option<String> {
        response
            .headers()
            .get(SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::to_owned)
    }

    fn header_str<'a>(
        response: &'a axum::response::Response,
        header: axum::http::header::HeaderName,
    ) -> Option<&'a str> {
        response.headers().get(header).and_then(|value| value.to_str().ok())
    }
}
