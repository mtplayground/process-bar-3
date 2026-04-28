use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{header::CONTENT_TYPE, Method, Response},
};
use tower::{Layer, Service};
use tracing::warn;

#[derive(Debug, Clone, Default)]
pub struct MethodOverrideLayer;

impl MethodOverrideLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for MethodOverrideLayer {
    type Service = MethodOverrideService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MethodOverrideService { inner }
    }
}

#[derive(Debug, Clone)]
pub struct MethodOverrideService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for MethodOverrideService<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let request = override_request_method(request).await;
            inner.call(request).await
        })
    }
}

async fn override_request_method(request: Request<Body>) -> Request<Body> {
    if request.method() != Method::POST || !is_form_request(&request) {
        return request;
    }

    let (mut parts, body) = request.into_parts();

    match to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            if let Some(method) = extract_override_method(&bytes) {
                parts.method = method;
            }

            Request::from_parts(parts, Body::from(bytes))
        }
        Err(error) => {
            warn!(error = ?error, "failed to inspect form body for method override");
            Request::from_parts(parts, Body::empty())
        }
    }
}

fn is_form_request(request: &Request<Body>) -> bool {
    request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("application/x-www-form-urlencoded"))
        .unwrap_or(false)
}

fn extract_override_method(body: &[u8]) -> Option<Method> {
    form_urlencoded::parse(body)
        .find_map(|(key, value)| {
            if key == "_method" {
                Some(value.into_owned())
            } else {
                None
            }
        })
        .and_then(|value| match value.trim().to_ascii_uppercase().as_str() {
            "PUT" => Some(Method::PUT),
            "DELETE" => Some(Method::DELETE),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{header::CONTENT_TYPE, Method, Request, StatusCode},
        routing::post,
        Router,
    };
    use tower::{Layer, ServiceExt};

    use super::MethodOverrideLayer;

    async fn post_handler() -> &'static str {
        "post"
    }

    async fn put_handler() -> &'static str {
        "put"
    }

    async fn delete_handler() -> &'static str {
        "delete"
    }

    #[tokio::test]
    async fn promotes_post_to_put() {
        let app = MethodOverrideLayer::new().layer(
            Router::new().route("/notes/1", post(post_handler).put(put_handler)),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/notes/1")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("_method=PUT&title=hello"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, "put");
    }

    #[tokio::test]
    async fn promotes_post_to_delete() {
        let app = MethodOverrideLayer::new().layer(
            Router::new().route("/notes/1", post(post_handler).delete(delete_handler)),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/notes/1")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("title=hello&_method=delete"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, "delete");
    }

    #[tokio::test]
    async fn leaves_plain_post_requests_unchanged() {
        let app = MethodOverrideLayer::new().layer(
            Router::new().route("/notes/1", post(post_handler).put(put_handler)),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/notes/1")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("title=hello"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, "post");
    }
}
