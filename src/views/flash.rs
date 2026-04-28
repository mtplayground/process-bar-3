use std::convert::Infallible;

use axum::{
    extract::FromRequestParts,
    http::{HeaderValue, header::COOKIE, request::Parts},
};
use process_bar_3::flash::{FlashMessage, take_flash_cookie};
use tracing::error;

use crate::AppState;

#[derive(Debug, Default, Clone)]
pub struct IncomingFlash {
    pub flash: Option<FlashMessage>,
    pub clear_cookie: Option<HeaderValue>,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for IncomingFlash {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookie_header = parts.headers.get(COOKIE);

        let Some(cookie_header) = cookie_header else {
            return Ok(Self::default());
        };

        match take_flash_cookie(&state.session_secret, Some(cookie_header)) {
            Ok(read) => Ok(Self {
                flash: read.flash,
                clear_cookie: Some(read.clear_cookie),
            }),
            Err(source) => {
                error!(error = ?source, "failed to read flash cookie");
                Ok(Self::default())
            }
        }
    }
}
