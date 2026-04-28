use askama::Template;
use uuid::Uuid;

use crate::forms::note_input::{NoteInput, NoteInputErrors};
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

#[derive(Debug)]
pub struct NoteFormView<'a> {
    pub action: String,
    pub submit_label: &'static str,
    pub show_method_override: bool,
    pub method_override: &'static str,
    pub input: &'a NoteInput,
    pub errors: &'a NoteInputErrors,
}

impl<'a> NoteFormView<'a> {
    pub fn for_create(input: &'a NoteInput, errors: &'a NoteInputErrors) -> Self {
        Self {
            action: String::from("/notes"),
            submit_label: "Create note",
            show_method_override: false,
            method_override: "",
            input,
            errors,
        }
    }

    pub fn for_update(note_id: Uuid, input: &'a NoteInput, errors: &'a NoteInputErrors) -> Self {
        Self {
            action: format!("/notes/{note_id}"),
            submit_label: "Save changes",
            show_method_override: true,
            method_override: "PUT",
            input,
            errors,
        }
    }

    pub fn has_title_error(&self) -> bool {
        self.errors.title.is_some()
    }

    pub fn title_error(&self) -> &str {
        self.errors.title.as_deref().unwrap_or("")
    }

    pub fn has_content_error(&self) -> bool {
        self.errors.content.is_some()
    }

    pub fn content_error(&self) -> &str {
        self.errors.content.as_deref().unwrap_or("")
    }

    pub fn has_tags_error(&self) -> bool {
        self.errors.tags_raw.is_some()
    }

    pub fn tags_error(&self) -> &str {
        self.errors.tags_raw.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Template)]
#[template(path = "notes/new.html")]
pub struct NewNoteTemplate<'a> {
    pub form: NoteFormView<'a>,
}

impl<'a> NewNoteTemplate<'a> {
    pub fn new(input: &'a NoteInput, errors: &'a NoteInputErrors) -> Self {
        Self {
            form: NoteFormView::for_create(input, errors),
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "notes/edit.html")]
pub struct EditNoteTemplate<'a> {
    pub form: NoteFormView<'a>,
}

impl<'a> EditNoteTemplate<'a> {
    pub fn new(note_id: Uuid, input: &'a NoteInput, errors: &'a NoteInputErrors) -> Self {
        Self {
            form: NoteFormView::for_update(note_id, input, errors),
        }
    }
}
