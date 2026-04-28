use askama::Template;

use crate::models::note::Note;

#[derive(Debug, Template)]
#[template(path = "notes/index.html")]
pub struct NotesIndexTemplate<'a> {
    pub notes: &'a [Note],
}

#[derive(Debug, Template)]
#[template(path = "notes/show.html")]
pub struct NoteShowTemplate<'a> {
    pub note: &'a Note,
}
