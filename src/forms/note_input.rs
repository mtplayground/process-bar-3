use crate::models::note::Note;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct NoteInput {
    pub title: String,
    pub content: String,
    pub tags_raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedNoteInput {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct NoteInputErrors {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags_raw: Option<String>,
}

impl NoteInputErrors {
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.content.is_none() && self.tags_raw.is_none()
    }
}

impl NoteInput {
    pub fn from_note(note: &Note) -> Self {
        Self {
            title: note.title.clone(),
            content: note.content.clone(),
            tags_raw: note.tags_csv(),
        }
    }

    pub fn validate(&self) -> Result<ValidatedNoteInput, NoteInputErrors> {
        let title = self.title.trim().to_owned();
        let content = self.content.trim().to_owned();
        let tags = Note::parse_tags_csv(&self.tags_raw);

        let mut errors = NoteInputErrors::default();

        if title.is_empty() {
            errors.title = Some(String::from("Title must be at least 1 character."));
        } else if title.chars().count() > 120 {
            errors.title = Some(String::from("Title must be at most 120 characters."));
        }

        if content.is_empty() {
            errors.content = Some(String::from("Content cannot be empty."));
        }

        if errors.is_empty() {
            Ok(ValidatedNoteInput {
                title,
                content,
                tags,
            })
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NoteInput, NoteInputErrors, ValidatedNoteInput};
    use crate::models::note::Note;

    #[test]
    fn validate_accepts_valid_input_and_normalizes_fields() {
        let input = NoteInput {
            title: String::from("  My Note  "),
            content: String::from("  Important content.  "),
            tags_raw: String::from(" Rust, axum, rust , SQLX , , AxUm "),
        };

        let validated = input.validate();

        assert_eq!(
            validated,
            Ok(ValidatedNoteInput {
                title: String::from("My Note"),
                content: String::from("Important content."),
                tags: vec![
                    String::from("axum"),
                    String::from("rust"),
                    String::from("sqlx"),
                ],
            })
        );
    }

    #[test]
    fn validate_rejects_empty_title_and_content() {
        let input = NoteInput {
            title: String::from("   "),
            content: String::from("\n\t "),
            tags_raw: String::new(),
        };

        let validated = input.validate();

        assert_eq!(
            validated,
            Err(NoteInputErrors {
                title: Some(String::from("Title must be at least 1 character.")),
                content: Some(String::from("Content cannot be empty.")),
                tags_raw: None,
            })
        );
    }

    #[test]
    fn validate_rejects_titles_longer_than_120_characters() {
        let input = NoteInput {
            title: "a".repeat(121),
            content: String::from("Body"),
            tags_raw: String::new(),
        };

        let validated = input.validate();

        assert_eq!(
            validated,
            Err(NoteInputErrors {
                title: Some(String::from("Title must be at most 120 characters.")),
                content: None,
                tags_raw: None,
            })
        );
    }

    #[test]
    fn from_note_prefills_form_fields() {
        let note = Note {
            id: uuid::Uuid::nil(),
            title: String::from("Existing"),
            content: String::from("Stored content"),
            tags: vec![String::from("rust"), String::from("sqlx")],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(
            NoteInput::from_note(&note),
            NoteInput {
                title: String::from("Existing"),
                content: String::from("Stored content"),
                tags_raw: String::from("rust, sqlx"),
            }
        );
    }
}
