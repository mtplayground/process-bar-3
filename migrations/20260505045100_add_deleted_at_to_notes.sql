ALTER TABLE notes
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

CREATE INDEX IF NOT EXISTS idx_notes_active_created_at
    ON notes (created_at DESC)
    WHERE deleted_at IS NULL;
