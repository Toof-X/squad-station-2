ALTER TABLE messages ADD COLUMN thread_id TEXT DEFAULT NULL;
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
