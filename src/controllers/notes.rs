use axum::{
    Form,
    extract::{Path, State},
    response::Redirect,
    response::{IntoResponse, Response},
};
use process_bar_3::{
    error::{AppError, AppResult},
    flash::{FlashKind, set_flash_cookie},
    forms::note_input::NoteInput,
    models::note::Note,
    templates::{EditNoteTemplate, NewNoteTemplate, NoteShowTemplate, NotesIndexTemplate},
};
use tracing::error;
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
                &state.session_secret,
                FlashKind::Success,
                "Note created successfully.",
            ))
        }
        Err(errors) => Ok(NewNoteTemplate::new(&input, &errors).into_response()),
    }
}

pub async fn edit(
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
) -> AppResult<Response> {
    let note = Note::find(&state.pool, note_id)
        .await?
        .ok_or_else(|| AppError::not_found("The requested note does not exist."))?;
    let input = NoteInput::from_note(&note);
    let errors = Default::default();

    Ok(EditNoteTemplate::new(note_id, &input, &errors).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
    Form(input): Form<NoteInput>,
) -> AppResult<Response> {
    match input.validate() {
        Ok(validated) => {
            let updated = Note::update(
                &state.pool,
                note_id,
                &validated.title,
                &validated.content,
                &validated.tags,
            )
            .await?
            .ok_or_else(|| AppError::not_found("The requested note does not exist."))?;

            Ok(redirect_with_flash(
                &format!("/notes/{}", updated.id),
                &state.session_secret,
                FlashKind::Success,
                "Note updated successfully.",
            ))
        }
        Err(errors) => Ok(EditNoteTemplate::new(note_id, &input, &errors).into_response()),
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

pub async fn delete(
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
) -> AppResult<Response> {
    let deleted = Note::delete(&state.pool, note_id).await?;

    if deleted {
        Ok(redirect_with_flash(
            "/notes",
            &state.session_secret,
            FlashKind::Success,
            "Note deleted successfully.",
        ))
    } else {
        Err(AppError::not_found("The requested note does not exist."))
    }
}

fn redirect_with_flash(
    redirect_to: &str,
    session_secret: &str,
    kind: FlashKind,
    message: &str,
) -> Response {
    let mut response = Redirect::to(redirect_to).into_response();
    match set_flash_cookie(session_secret, kind, message) {
        Ok(value) => {
            response
                .headers_mut()
                .insert(axum::http::header::SET_COOKIE, value);
        }
        Err(source) => {
            error!(error = ?source, "failed to encode flash cookie");
        }
    }

    response
}
