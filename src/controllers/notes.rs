use axum::{
    Form,
    extract::{Path, State},
    http::header::SET_COOKIE,
    response::Redirect,
    response::{IntoResponse, Response},
};
use process_bar_3::{
    error::{AppError, AppResult},
    forms::note_input::NoteInput,
    models::note::Note,
    templates::{NewNoteTemplate, NoteShowTemplate, NotesIndexTemplate},
};
use uuid::Uuid;

use crate::AppState;

pub async fn root_redirect() -> Redirect {
    Redirect::to("/notes")
}

pub async fn index(State(state): State<AppState>) -> AppResult<NotesIndexTemplate> {
    let notes = Note::list(&state.pool).await?;

    Ok(NotesIndexTemplate { notes })
}

pub async fn new() -> AppResult<Response> {
    let input = NoteInput::default();
    let errors = Default::default();

    Ok(NewNoteTemplate::new(&input, &errors).into_response())
}

pub async fn create(
    State(state): State<AppState>,
    Form(input): Form<NoteInput>,
) -> AppResult<Response> {
    match input.validate() {
        Ok(validated) => {
            let note =
                Note::create(&state.pool, &validated.title, &validated.content, &validated.tags)
                    .await?;

            Ok(redirect_with_flash(
                &format!("/notes/{}", note.id),
                "Note created successfully.",
            ))
        }
        Err(errors) => Ok(NewNoteTemplate::new(&input, &errors).into_response()),
    }
}

pub async fn show(
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
) -> AppResult<NoteShowTemplate> {
    let note = Note::find(&state.pool, note_id)
        .await?
        .ok_or_else(|| AppError::not_found("The requested note does not exist."))?;

    Ok(NoteShowTemplate { note })
}

fn redirect_with_flash(redirect_to: &str, message: &str) -> Response {
    let cookie_value = format!(
        "flash=success.{}; Max-Age=60; Path=/; HttpOnly; SameSite=Lax",
        encode_cookie_component(message)
    );

    let mut response = Redirect::to(redirect_to).into_response();
    if let Ok(value) = cookie_value.parse() {
        response.headers_mut().insert(SET_COOKIE, value);
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
