-- Stage registry schema.
-- The full Stage struct is stored as JSONB so Rust drives schema evolution.
-- Two extracted columns (lifecycle, description) allow efficient SQL filtering.

CREATE TABLE IF NOT EXISTS stages (
    id          TEXT        PRIMARY KEY,
    lifecycle   TEXT        NOT NULL DEFAULT 'draft',
    description TEXT        NOT NULL DEFAULT '',
    data        JSONB       NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Fast lookup by lifecycle (list active/draft stages)
CREATE INDEX IF NOT EXISTS stages_lifecycle_idx ON stages (lifecycle);

-- Optional: full-text search on description (supplement to semantic index)
CREATE INDEX IF NOT EXISTS stages_description_fts_idx
    ON stages USING gin (to_tsvector('english', description));
