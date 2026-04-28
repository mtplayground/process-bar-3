use std::error::Error;

use process_bar_3::{config::Config, db, models::note::Note};
use sqlx::PgPool;

struct SampleNote {
    title: &'static str,
    content: &'static str,
    tags: &'static [&'static str],
}

const SAMPLE_NOTES: &[SampleNote] = &[
    SampleNote {
        title: "Project Kickoff",
        content: "Capture the initial scope, constraints, and milestones before writing code.",
        tags: &["planning", "team"],
    },
    SampleNote {
        title: "Rust Tips",
        content: "Prefer small modules, explicit error types, and async boundaries that are easy to test.",
        tags: &["rust", "engineering"],
    },
    SampleNote {
        title: "Release Checklist",
        content: "Run migrations, verify logs, smoke-test critical paths, and confirm rollback steps.",
        tags: &["ops", "release"],
    },
    SampleNote {
        title: "Customer Follow-up",
        content: "Summarize the latest feedback and note any action items for the next sprint.",
        tags: &["product", "follow-up"],
    },
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let pool = db::init_pool(&config).await?;
    db::run_migrations(&pool).await?;

    if notes_exist(&pool).await? {
        println!("seed skipped: notes table already contains data");
        return Ok(());
    }

    for note in SAMPLE_NOTES {
        let tags = note
            .tags
            .iter()
            .map(|tag| (*tag).to_owned())
            .collect::<Vec<_>>();

        Note::create(&pool, note.title, note.content, &tags).await?;
    }

    println!("seeded {} sample notes", SAMPLE_NOTES.len());
    Ok(())
}

async fn notes_exist(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notes")
        .fetch_one(pool)
        .await?;

    Ok(count > 0)
}
