CREATE OR REPLACE VIEW active_notes AS
SELECT id, title, content, tags, created_at, updated_at
FROM notes
WHERE deleted_at IS NULL;
