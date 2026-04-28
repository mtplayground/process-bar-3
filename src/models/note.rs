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
}

impl Note {
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
            r#"
            INSERT INTO notes (title, content, tags)
            VALUES ($1, $2, $3)
            RETURNING id, title, content, tags, created_at, updated_at
            "#,
        )
        .bind(title)
        .bind(content)
        .bind(tags)
        .fetch_one(pool)
        .await
    }

    pub async fn list(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT id, title, content, tags, created_at, updated_at
            FROM notes
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT id, title, content, tags, created_at, updated_at
            FROM notes
            WHERE id = $1
            LIMIT 1
            "#,
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
            r#"
            UPDATE notes
            SET title = $2,
                content = $3,
                tags = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, title, content, tags, created_at, updated_at
            "#,
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
            DELETE FROM notes
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::Note;

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
}
