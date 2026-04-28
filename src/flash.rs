use axum::http::HeaderValue;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

const FLASH_COOKIE_NAME: &str = "flash";
const FLASH_COOKIE_MAX_AGE: u64 = 60;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlashKind {
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashMessage {
    pub kind: FlashKind,
    pub message: String,
}

#[derive(Debug)]
pub struct FlashRead {
    pub flash: Option<FlashMessage>,
    pub clear_cookie: HeaderValue,
}

#[derive(Debug, Error)]
pub enum FlashError {
    #[error("flash cookie value could not be serialized: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("flash cookie header value is invalid: {0}")]
    Header(#[from] axum::http::header::InvalidHeaderValue),
    #[error("flash cookie signer could not be initialized")]
    SignerInitialization,
}

pub fn set_flash_cookie(
    secret: &str,
    kind: FlashKind,
    message: &str,
) -> Result<HeaderValue, FlashError> {
    let payload = FlashMessage {
        kind,
        message: message.to_owned(),
    };
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload)?);
    let signature = sign(secret, &payload)?;
    let cookie = format!(
        "{FLASH_COOKIE_NAME}={payload}.{signature}; Max-Age={FLASH_COOKIE_MAX_AGE}; Path=/; HttpOnly; SameSite=Lax"
    );

    HeaderValue::from_str(&cookie).map_err(FlashError::Header)
}

pub fn take_flash_cookie(
    secret: &str,
    cookie_header: Option<&HeaderValue>,
) -> Result<FlashRead, FlashError> {
    let clear_cookie = clear_flash_cookie()?;
    let flash = cookie_header
        .and_then(|header| header.to_str().ok())
        .and_then(find_flash_cookie_value)
        .and_then(|value| verify_and_decode(secret, value));

    Ok(FlashRead { flash, clear_cookie })
}

pub fn clear_flash_cookie() -> Result<HeaderValue, FlashError> {
    let cookie = format!("{FLASH_COOKIE_NAME}=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax");
    HeaderValue::from_str(&cookie).map_err(FlashError::Header)
}

fn sign(secret: &str, payload: &str) -> Result<String, FlashError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| FlashError::SignerInitialization)?;
    mac.update(payload.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn verify_and_decode(secret: &str, cookie_value: &str) -> Option<FlashMessage> {
    let (payload, signature) = cookie_value.rsplit_once('.')?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature).ok()?;

    let payload = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&payload).ok()
}

fn find_flash_cookie_value(cookie_header: &str) -> Option<&str> {
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{FLASH_COOKIE_NAME}=")))
}

#[cfg(test)]
mod tests {
    use super::{FlashKind, FlashMessage, clear_flash_cookie, set_flash_cookie, take_flash_cookie};
    use axum::http::HeaderValue;

    const SECRET: &str = "test-session-secret";

    #[test]
    fn set_and_take_flash_cookie_round_trip() {
        let set_cookie = match set_flash_cookie(SECRET, FlashKind::Success, "Saved note") {
            Ok(value) => value,
            Err(error) => panic!("expected flash cookie to be created: {error}"),
        };
        let cookie = set_cookie
            .to_str()
            .ok()
            .and_then(|value| value.split(';').next())
            .and_then(|value| HeaderValue::from_str(value).ok());

        let read = match take_flash_cookie(SECRET, cookie.as_ref()) {
            Ok(read) => read,
            Err(error) => panic!("expected flash cookie to be readable: {error}"),
        };

        assert_eq!(
            read.flash,
            Some(FlashMessage {
                kind: FlashKind::Success,
                message: String::from("Saved note"),
            })
        );
        assert_eq!(
            read.clear_cookie.to_str().ok(),
            Some("flash=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax")
        );
    }

    #[test]
    fn take_flash_cookie_ignores_invalid_signature_and_still_clears_cookie() {
        let cookie = HeaderValue::from_static("flash=invalid.signature");

        let read = match take_flash_cookie(SECRET, Some(&cookie)) {
            Ok(read) => read,
            Err(error) => panic!("expected invalid flash cookies to be ignored: {error}"),
        };

        assert!(read.flash.is_none());
        assert_eq!(
            read.clear_cookie.to_str().ok(),
            Some("flash=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax")
        );
    }

    #[test]
    fn clear_flash_cookie_expires_the_cookie() {
        let header = match clear_flash_cookie() {
            Ok(header) => header,
            Err(error) => panic!("expected clear cookie header: {error}"),
        };

        assert_eq!(
            header.to_str().ok(),
            Some("flash=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax")
        );
    }
}
