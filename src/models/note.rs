use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
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
