-- src/db/migrations/0003_v11.sql
-- Phase 4: Align DB schema with solution design

-- ============================================================
-- AGENTS TABLE
-- ============================================================

-- AGNT-03: rename provider → tool
-- SQLite >= 3.25 (2018) supports RENAME COLUMN; sqlx 0.8 bundles a recent SQLite
ALTER TABLE agents RENAME COLUMN provider TO tool;

-- AGNT-01: add model and description (optional fields, nullable)
ALTER TABLE agents ADD COLUMN model       TEXT DEFAULT NULL;
ALTER TABLE agents ADD COLUMN description TEXT DEFAULT NULL;

-- AGNT-02: add current_task FK (nullable; FK constraint is decorative — SQLite FK
-- enforcement is disabled by default and we do not enable it)
ALTER TABLE agents ADD COLUMN current_task TEXT DEFAULT NULL
    REFERENCES messages(id);

-- ============================================================
-- MESSAGES TABLE
-- ============================================================

-- MSGS-01: add directional routing fields
-- Use NULL default (not NOT NULL DEFAULT '') — old rows have no known sender
ALTER TABLE messages ADD COLUMN from_agent  TEXT DEFAULT NULL;
ALTER TABLE messages ADD COLUMN to_agent    TEXT DEFAULT NULL;

-- MSGS-02: add message type
ALTER TABLE messages ADD COLUMN type TEXT NOT NULL DEFAULT 'task_request';

-- MSGS-04: add completion timestamp
ALTER TABLE messages ADD COLUMN completed_at TEXT DEFAULT NULL;

-- Backfill to_agent from legacy agent_name column for existing rows
UPDATE messages SET to_agent = agent_name WHERE to_agent IS NULL;

-- Note: changing default status 'pending' → 'processing' is handled in Rust INSERT
-- (ALTER TABLE cannot change DEFAULT on existing column in SQLite)

-- Index for directional message queries
CREATE INDEX IF NOT EXISTS idx_messages_direction ON messages(from_agent, to_agent);
