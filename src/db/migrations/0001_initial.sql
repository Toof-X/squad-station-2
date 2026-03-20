CREATE TABLE IF NOT EXISTS agents (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    provider    TEXT NOT NULL DEFAULT '',
    role        TEXT NOT NULL DEFAULT 'worker',
    command     TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id          TEXT PRIMARY KEY,
    agent_name  TEXT NOT NULL,
    task        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    priority    TEXT NOT NULL DEFAULT 'normal',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    FOREIGN KEY (agent_name) REFERENCES agents(name)
);

CREATE INDEX IF NOT EXISTS idx_messages_agent_status ON messages(agent_name, status);
CREATE INDEX IF NOT EXISTS idx_messages_priority ON messages(priority, created_at);
