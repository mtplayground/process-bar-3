# process-bar-3

Rust notes app built with Axum, Askama, SQLx, and PostgreSQL.

## Prerequisites

- Rust toolchain
- Docker and Docker Compose for the containerized path
- PostgreSQL 16 for local development if you are not using Docker
- `sqlx-cli` for the local migration workflow:

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

## Quickstart

```bash
git clone https://github.com/mtplayground/process-bar-3.git
cd process-bar-3
cp .env.example .env
docker compose up --build
```

The app listens on `http://127.0.0.1:8080`.

The Compose stack starts:

- `app`: the Axum web server
- `db`: PostgreSQL 16 with a named `postgres_data` volume

Inside Docker, the app uses the Compose database service with:

```text
postgres://postgres:postgres@db:5432/process_bar_3
```

## Local Development

1. Create a local environment file:

```bash
cp .env.example .env
```

2. Update `DATABASE_URL` in `.env` so it points at your local PostgreSQL instance.

3. Run migrations:

```bash
sqlx migrate run
```

4. Start the app:

```bash
cargo run
```

5. Optional: load demo notes into a non-empty database only if the table is empty:

```bash
cargo run --bin seed
```

The app binds to `0.0.0.0:8080` by default.

## Configuration

The app loads configuration from environment variables and optional dotenv files in this order:

- `.env`
- `.env.production`
- process environment

Required variables:

- `DATABASE_URL`
- `BIND_ADDR`
- `RUST_LOG`
- `SESSION_SECRET`

## External Data Access

External read-only consumers should query the `active_notes` database view instead of
reading directly from the `notes` table.

The view is created by `migrations/20260505063000_create_active_notes_view.sql`:

```sql
SELECT id, title, content, tags, created_at, updated_at
FROM active_notes;
```

Contract details:

- `active_notes` exposes only non-deleted notes.
- A note disappears from `active_notes` as soon as the application soft-deletes it by
  setting `notes.deleted_at`.
- The view returns the columns `id`, `title`, `content`, `tags`, `created_at`, and
  `updated_at`.
- External systems that only need current note content should treat `active_notes` as
  the stable read surface.
- Direct reads from `notes` are only appropriate when a consumer explicitly needs
  soft-deleted rows or the internal `deleted_at` lifecycle field.

## Project Layout

The codebase follows a lightweight MVC layout:

- `src/controllers/`: HTTP handlers and route-facing application flow
- `src/models/`: database-backed domain types like `Note`
- `src/views/`: request-driven view helpers such as flash extraction
- `src/templates/`: Askama template structs that render HTML
- `templates/`: HTML templates and shared layout partials
- `src/routes.rs`: Axum route wiring and middleware stack
- `src/db.rs`: PostgreSQL pool initialization and migrations
- `src/error.rs`: application error handling and error page responses
- `src/forms/`: form parsing and validation
- `migrations/`: SQLx migrations for PostgreSQL schema changes

## Notes

- Persistent state uses PostgreSQL only.
- Startup runs pending SQLx migrations before the server begins accepting requests.
- The router includes request tracing, request IDs, flash cookies, and HTML error pages.
