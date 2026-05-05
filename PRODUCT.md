# process-bar-3

## What It Is

`process-bar-3` is a small server-rendered notes application built in Rust with Axum,
Askama, SQLx, and PostgreSQL. It is a single-service web app for creating, viewing,
editing, and deleting notes through HTML pages.

## What It Does

- Creates notes with `title`, `content`, and comma-separated `tags`
- Lists active notes newest-first
- Shows individual notes
- Updates existing notes
- Soft-deletes notes instead of removing rows permanently
- Displays one-time success and error flash messages

## Current Product Contract

- The user-facing app only works with active notes.
- Deleting a note sets `deleted_at` and hides the note from normal reads.
- External read-only consumers should use the `active_notes` database view, not the
  base `notes` table.
- `active_notes` exposes: `id`, `title`, `content`, `tags`, `created_at`,
  `updated_at`.

## Architecture Snapshot

- `src/main.rs`: bootstraps config, logging, DB pool, migrations, and HTTP server
- `src/routes.rs`: Axum routes, request IDs, tracing, static files, fallback
- `src/controllers/notes.rs`: note CRUD request handlers
- `src/models/note.rs`: note queries, tag normalization, soft-delete behavior
- `src/forms/note_input.rs`: form parsing and validation
- `src/templates/mod.rs` plus `templates/`: Askama-backed HTML rendering
- `src/flash.rs` and `src/views/flash.rs`: signed flash cookie contract
- `src/middleware/method_override.rs`: HTML form support for `PUT` and `DELETE`

## Key Decisions

- Server-rendered HTML instead of a JSON API or SPA
- PostgreSQL is the only persistence layer
- Soft delete is the canonical delete behavior
- Startup runs migrations before serving traffic
- Flash messages are signed cookies keyed by `SESSION_SECRET`
- The codebase follows a lightweight MVC-style split across controllers, models,
  forms, views, and templates

## Conventions

- Note tags are normalized to lowercase, deduplicated, and stored as `TEXT[]`
- Normal application reads exclude soft-deleted rows
- Request flow returns HTML pages and redirects, not API responses
- Static assets live under `static/` and are served at `/static`

## Operational Notes

- Required env vars: `DATABASE_URL`, `BIND_ADDR`, `RUST_LOG`, `SESSION_SECRET`
- Docker Compose starts the app and PostgreSQL together for local development
- A seed binary exists at `src/bin/seed.rs` for demo data when the table is empty

## Not Implemented

- Authentication or multi-user ownership
- Restore UI for soft-deleted notes
- Public JSON API
- Search, pagination, or tag filtering
