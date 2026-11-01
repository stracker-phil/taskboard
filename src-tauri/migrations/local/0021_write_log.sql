-- Every change sent to Jira, with Jira's answer: Settings → Status lists it and exports it (docs/writes.md).
CREATE TABLE write_log (
    id INTEGER PRIMARY KEY,
    at TEXT NOT NULL,
    issue_id TEXT NOT NULL,
    issue_key TEXT,
    change TEXT NOT NULL,
    error TEXT
);
CREATE INDEX write_log_at ON write_log (at);
