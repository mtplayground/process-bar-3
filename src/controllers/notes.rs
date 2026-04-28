use axum::{
    extract::{Path, State},
    response::Redirect,
};
use process_bar_3::{
    error::{AppError, AppResult},
    models::note::Note,
    templates::{NoteShowTemplate, NotesIndexTemplate},
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

pub async fn show(
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
) -> AppResult<NoteShowTemplate> {
    let note = Note::find(&state.pool, note_id)
        .await?
        .ok_or_else(|| AppError::not_found("The requested note does not exist."))?;

    Ok(NoteShowTemplate { note })
}
