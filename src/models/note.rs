use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, FromRow)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Note {
    const RETURNING_COLUMNS: &str = "id, title, content, tags, created_at, updated_at, deleted_at";

    pub fn parse_tags_csv(tags_raw: &str) -> Vec<String> {
        let mut tags = BTreeSet::new();

        for tag in tags_raw.split(',') {
            let normalized = tag.trim().to_lowercase();

            if !normalized.is_empty() {
                tags.insert(normalized);
            }
        }

        tags.into_iter().collect()
    }

    pub fn tags_csv(&self) -> String {
        Self::serialize_tags_csv(&self.tags)
    }

    pub fn serialize_tags_csv(tags: &[String]) -> String {
        tags.join(", ")
    }

    pub async fn create(
        pool: &PgPool,
        title: &str,
        content: &str,
        tags: &[String],
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            &format!(
                r#"
                INSERT INTO notes (title, content, tags)
                VALUES ($1, $2, $3)
                RETURNING {}
                "#,
                Self::RETURNING_COLUMNS
            ),
        )
        .bind(title)
        .bind(content)
        .bind(tags)
        .fetch_one(pool)
        .await
    }

    pub async fn list(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            &format!(
                r#"
                SELECT {}
                FROM notes
                WHERE deleted_at IS NULL
                ORDER BY created_at DESC, id DESC
                "#,
                Self::RETURNING_COLUMNS
            ),
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            &format!(
                r#"
                SELECT {}
                FROM notes
                WHERE id = $1
                  AND deleted_at IS NULL
                LIMIT 1
                "#,
                Self::RETURNING_COLUMNS
            ),
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_including_deleted(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            &format!(
                r#"
                SELECT {}
                FROM notes
                WHERE id = $1
                LIMIT 1
                "#,
                Self::RETURNING_COLUMNS
            ),
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        title: &str,
        content: &str,
        tags: &[String],
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            &format!(
                r#"
                UPDATE notes
                SET title = $2,
                    content = $3,
                    tags = $4,
                    updated_at = NOW()
                WHERE id = $1
                  AND deleted_at IS NULL
                RETURNING {}
                "#,
                Self::RETURNING_COLUMNS
            ),
        )
        .bind(id)
        .bind(title)
        .bind(content)
        .bind(tags)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE notes
            SET deleted_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn restore(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            &format!(
                r#"
                UPDATE notes
                SET deleted_at = NULL,
                    updated_at = NOW()
                WHERE id = $1
                  AND deleted_at IS NOT NULL
                RETURNING {}
                "#,
                Self::RETURNING_COLUMNS
            ),
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::Note;
    use sqlx::PgPool;

    #[test]
    fn parse_tags_csv_normalizes_and_deduplicates_tags() {
        let tags = Note::parse_tags_csv(" Rust, axum, rust , SQLX , , AxUm ");

        assert_eq!(
            tags,
            vec![
                String::from("axum"),
                String::from("rust"),
                String::from("sqlx"),
            ]
        );
    }

    #[test]
    fn serialize_tags_csv_joins_tags_for_forms() {
        let tags = vec![
            String::from("axum"),
            String::from("rust"),
            String::from("sqlx"),
        ];

        assert_eq!(Note::serialize_tags_csv(&tags), "axum, rust, sqlx");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn list_excludes_soft_deleted_notes(pool: PgPool) {
        let active = create_note(&pool, "Active note").await;
        let deleted = create_note(&pool, "Deleted note").await;

        assert!(delete_note(&pool, deleted.id).await);

        let notes = match Note::list(&pool).await {
            Ok(notes) => notes,
            Err(error) => panic!("expected list query to succeed: {error}"),
        };

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, active.id);
        assert!(notes.iter().all(|note| note.deleted_at.is_none()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_returns_none_for_soft_deleted_notes(pool: PgPool) {
        let note = create_note(&pool, "Deleted note").await;

        assert!(delete_note(&pool, note.id).await);

        let found = match Note::find(&pool, note.id).await {
            Ok(found) => found,
            Err(error) => panic!("expected find query to succeed: {error}"),
        };

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn delete_sets_deleted_at_instead_of_removing_row(pool: PgPool) {
        let note = create_note(&pool, "Soft delete target").await;

        let deleted = delete_note(&pool, note.id).await;
        let found = match Note::find_including_deleted(&pool, note.id).await {
            Ok(found) => found,
            Err(error) => panic!("expected deleted note lookup to succeed: {error}"),
        };

        assert!(deleted);
        assert!(found.is_some());
        assert!(found.and_then(|note| note.deleted_at).is_some());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn delete_returns_false_when_note_is_already_deleted(pool: PgPool) {
        let note = create_note(&pool, "Already deleted").await;

        assert!(delete_note(&pool, note.id).await);

        let deleted_again = match Note::delete(&pool, note.id).await {
            Ok(deleted) => deleted,
            Err(error) => panic!("expected second delete query to succeed: {error}"),
        };

        assert!(!deleted_again);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_including_deleted_returns_soft_deleted_notes(pool: PgPool) {
        let note = create_note(&pool, "Deleted but queryable").await;

        assert!(delete_note(&pool, note.id).await);

        let found = match Note::find_including_deleted(&pool, note.id).await {
            Ok(found) => found,
            Err(error) => panic!("expected deleted note lookup to succeed: {error}"),
        };

        assert!(found.is_some());
        assert_eq!(found.map(|note| note.id), Some(note.id));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn restore_makes_soft_deleted_note_visible_to_find(pool: PgPool) {
        let note = create_note(&pool, "Restorable note").await;

        assert!(delete_note(&pool, note.id).await);
        assert!(find_note(&pool, note.id).await.is_none());

        let restored = match Note::restore(&pool, note.id).await {
            Ok(restored) => restored,
            Err(error) => panic!("expected restore query to succeed: {error}"),
        };
        let found = find_note(&pool, note.id).await;

        assert!(restored.is_some());
        assert!(restored.as_ref().and_then(|note| note.deleted_at).is_none());
        assert_eq!(found.map(|note| note.id), Some(note.id));
    }

    async fn create_note(pool: &PgPool, title: &str) -> Note {
        let tags = vec![String::from("test")];

        match Note::create(pool, title, "Body", &tags).await {
            Ok(note) => note,
            Err(error) => panic!("expected note to be created: {error}"),
        }
    }

    async fn delete_note(pool: &PgPool, id: uuid::Uuid) -> bool {
        match Note::delete(pool, id).await {
            Ok(deleted) => deleted,
            Err(error) => panic!("expected delete query to succeed: {error}"),
        }
    }

    async fn find_note(pool: &PgPool, id: uuid::Uuid) -> Option<Note> {
        match Note::find(pool, id).await {
            Ok(found) => found,
            Err(error) => panic!("expected find query to succeed: {error}"),
        }
    }
}
