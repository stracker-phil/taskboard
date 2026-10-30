-- When the owner last looked at an issue and whether they marked it unread, moved here from the cache so a
-- cache reset keeps it. No foreign key: the issues live in the other file (`sync::store::forget_issue` deletes
-- the row with its issue). `connection::carry_over_seen` copies the cache's rows once.
CREATE TABLE issue_seen (
  issue_id TEXT PRIMARY KEY,
  seen_at  TEXT NOT NULL,
  unread   INTEGER NOT NULL DEFAULT 0
);
