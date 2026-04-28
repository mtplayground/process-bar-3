use askama::Template;
use axum::{
    http::{
        header::SET_COOKIE,
        HeaderValue, StatusCode,
    },
    response::{Html, IntoResponse, Redirect, Response},
};
use thiserror::Error;
use tracing::error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    NotFound { message: String },
    #[error("{message}")]
    Validation { message: String, redirect_to: String },
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("template error")]
    Template(#[from] askama::Error),
}

impl AppError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>, redirect_to: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            redirect_to: redirect_to.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound { message } => render_template_response(
                StatusCode::NOT_FOUND,
                &NotFoundTemplate {
                    message: message.as_str(),
                },
            ),
            Self::Validation {
                message,
                redirect_to,
            } => redirect_with_flash(&redirect_to, &message),
            Self::Database(source) => {
                error!(error = ?source, "database error");
                render_template_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ServerErrorTemplate {
                        message: "The database request could not be completed.",
                    },
                )
            }
            Self::Template(source) => {
                error!(error = ?source, "template rendering error");
                render_template_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ServerErrorTemplate {
                        message: "The page could not be rendered.",
                    },
                )
            }
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "errors/404.html")]
struct NotFoundTemplate<'a> {
    message: &'a str,
}

#[derive(Debug, Template)]
#[template(path = "errors/500.html")]
struct ServerErrorTemplate<'a> {
    message: &'a str,
}

fn render_template_response<T>(status: StatusCode, template: &T) -> Response
where
    T: Template,
{
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(render_error) => {
            error!(error = ?render_error, "failed to render error template");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(String::from(
                    "<h1>Internal Server Error</h1><p>The request could not be completed.</p>",
                )),
            )
                .into_response()
        }
    }
}

fn redirect_with_flash(redirect_to: &str, message: &str) -> Response {
    let encoded_message = encode_cookie_component(message);
    let cookie_value = format!("flash=error.{encoded_message}; Max-Age=60; Path=/; HttpOnly; SameSite=Lax");

    let mut response = Redirect::to(redirect_to).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie_value) {
        response.headers_mut().insert(SET_COOKIE, value);
    } else {
        error!("failed to encode flash cookie");
    }
    response
}

fn encode_cookie_component(input: &str) -> String {
    let mut encoded = String::new();

    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use axum::{
        http::{header::SET_COOKIE, StatusCode},
        response::IntoResponse,
    };

    #[tokio::test]
    async fn validation_error_redirects_with_flash_cookie() {
        let response = AppError::validation("Title is required.", "/notes/new").into_response();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(SET_COOKIE).unwrap(),
            "flash=error.Title%20is%20required.; Max-Age=60; Path=/; HttpOnly; SameSite=Lax"
        );
    }

    #[tokio::test]
    async fn not_found_renders_not_found_status() {
        let response = AppError::not_found("Missing note").into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
