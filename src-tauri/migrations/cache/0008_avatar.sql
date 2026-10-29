-- People's profile pictures from Atlassian's avatar host (ADR 0032, `avatars`), keyed by account. `url` is the
-- address last asked for; `bytes` the last picture that arrived, kept while a newer one fails.
CREATE TABLE avatar (
  account_id TEXT PRIMARY KEY,
  url        TEXT NOT NULL,
  mime       TEXT,                          -- the picture's checked type (`gateway::ImageKind`)
  bytes      BLOB,
  fetched_at TEXT,                          -- UTC; when `bytes` arrived
  failed_at  TEXT                           -- UTC; the last failed fetch of `url`, cleared by a success
);
