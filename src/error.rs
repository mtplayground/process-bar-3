use askama::Template;
use axum::{
    http::{
        header::SET_COOKIE,
        StatusCode,
    },
    response::{Html, IntoResponse, Redirect, Response},
};
use thiserror::Error;
use tracing::error;

use crate::flash::{FlashKind, set_flash_cookie};
use crate::templates::LayoutFlash;

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
                    flash: LayoutFlash::default(),
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
                        flash: LayoutFlash::default(),
                        message: "The database request could not be completed.",
                    },
                )
            }
            Self::Template(source) => {
                error!(error = ?source, "template rendering error");
                render_template_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &ServerErrorTemplate {
                        flash: LayoutFlash::default(),
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
    flash: LayoutFlash,
    message: &'a str,
}

#[derive(Debug, Template)]
#[template(path = "errors/500.html")]
struct ServerErrorTemplate<'a> {
    flash: LayoutFlash,
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
    let mut response = Redirect::to(redirect_to).into_response();
    match std::env::var("SESSION_SECRET") {
        Ok(secret) => match set_flash_cookie(&secret, FlashKind::Error, message) {
            Ok(value) => {
                response.headers_mut().insert(SET_COOKIE, value);
            }
            Err(source) => {
                error!(error = ?source, "failed to encode flash cookie");
            }
        },
        Err(source) => {
            error!(error = ?source, "session secret was unavailable for flash cookie");
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use axum::{
        http::{HeaderValue, header::SET_COOKIE, StatusCode},
        response::IntoResponse,
    };
    use crate::flash::{FlashKind, take_flash_cookie};

    fn set_test_session_secret() {
        unsafe {
            std::env::set_var("SESSION_SECRET", "test-session-secret");
        }
    }

    #[tokio::test]
    async fn validation_error_redirects_with_flash_cookie() {
        set_test_session_secret();
        let response = AppError::validation("Title is required.", "/notes/new").into_response();
        let set_cookie = response.headers().get(SET_COOKIE).and_then(|value| value.to_str().ok());
        let cookie = set_cookie
            .and_then(|value| value.split(';').next())
            .and_then(|value| HeaderValue::from_str(value).ok());
        let read = match take_flash_cookie("test-session-secret", cookie.as_ref()) {
            Ok(read) => read,
            Err(error) => panic!("expected flash cookie to be readable: {error}"),
        };

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            read.flash.map(|flash| (flash.kind, flash.message)),
            Some((FlashKind::Error, String::from("Title is required.")))
        );
    }

    #[tokio::test]
    async fn not_found_renders_not_found_status() {
        let response = AppError::not_found("Missing note").into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
