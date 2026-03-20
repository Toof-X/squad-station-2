ALTER TABLE agents ADD COLUMN status TEXT NOT NULL DEFAULT 'idle';
ALTER TABLE agents ADD COLUMN status_updated_at TEXT NOT NULL DEFAULT (datetime('now'));
