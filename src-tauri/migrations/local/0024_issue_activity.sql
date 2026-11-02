-- When the owner last opened an issue in the Issue view and last did anything with it in the app (opened,
-- previewed, bookmarked, changed in Jira). Local like `issue_seen`, so a cache reset keeps it;
-- `sync::store::forget_issue` deletes the row with its issue.
CREATE TABLE issue_activity (
  issue_id      TEXT PRIMARY KEY,
  viewed_at     TEXT,
  interacted_at TEXT
);
