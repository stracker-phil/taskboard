-- The Jira mirror (docs/data.md). Disposable: Reset cache deletes this file and the next full sync refills it.

CREATE TABLE project (
  id          TEXT PRIMARY KEY,           -- Jira project id
  key         TEXT NOT NULL,
  name        TEXT NOT NULL,
  avatar_url  TEXT,
  raw         TEXT NOT NULL
);

CREATE TABLE status (
  id        TEXT PRIMARY KEY,
  name      TEXT NOT NULL,
  category  TEXT NOT NULL                 -- new | indeterminate | done
);

CREATE TABLE issue (
  id                 TEXT PRIMARY KEY,    -- numeric Jira id as text; never the key
  key                TEXT NOT NULL,
  project_id         TEXT NOT NULL REFERENCES project(id),
  parent_id          TEXT,                -- epic, or the parent of a subtask
  fix_versions       TEXT,                -- JSON array of version ids
  epic_color         TEXT,                -- epics only, from Jira
  summary            TEXT NOT NULL,
  issue_type         TEXT NOT NULL,
  type_is_subtask    INTEGER NOT NULL DEFAULT 0,
  is_epic            INTEGER NOT NULL DEFAULT 0,
  status_id          TEXT NOT NULL,
  status_category    TEXT NOT NULL,       -- copy of status.category for filters without a join
  priority           TEXT,
  assignee_id        TEXT,
  reporter_id        TEXT,
  sprint             TEXT,
  sprint_id          TEXT,
  sprint_state       TEXT,                -- active | future | closed
  labels             TEXT,                -- JSON array
  due_date           TEXT,
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL,       -- Jira's updated timestamp, UTC
  resolved_at        TEXT,
  description_html   TEXT,
  description_adf    TEXT,
  comment_count      INTEGER NOT NULL DEFAULT 0,
  comments_synced_at TEXT,                -- when the comments were last fetched in full
  raw                TEXT NOT NULL,       -- the issue JSON as fetched
  synced_at          TEXT NOT NULL,
  gone               INTEGER NOT NULL DEFAULT 0  -- deleted or no longer visible
);
CREATE INDEX issue_updated ON issue(updated_at);
CREATE INDEX issue_project_status ON issue(project_id, status_id);
CREATE INDEX issue_project_gone ON issue(project_id, gone);
CREATE INDEX issue_parent ON issue(parent_id);
CREATE INDEX issue_assignee ON issue(assignee_id);

CREATE TABLE issue_change (               -- written by the sync when a tracked field differs
  id         INTEGER PRIMARY KEY,
  issue_id   TEXT NOT NULL REFERENCES issue(id) ON DELETE CASCADE,
  field      TEXT NOT NULL,               -- status | assignee | due_date | priority | comment_added | created
  old_value  TEXT,
  new_value  TEXT,
  changed_at TEXT NOT NULL                -- Jira's updated timestamp of that version
);
CREATE INDEX issue_change_issue ON issue_change(issue_id, changed_at);

CREATE TABLE issue_seen (
  issue_id   TEXT PRIMARY KEY REFERENCES issue(id) ON DELETE CASCADE,
  seen_at    TEXT NOT NULL
);

CREATE TABLE comment (
  id         TEXT PRIMARY KEY,
  issue_id   TEXT NOT NULL REFERENCES issue(id) ON DELETE CASCADE,
  author_id  TEXT,
  body_html  TEXT,
  body_text  TEXT,                        -- plain text for search
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX comment_issue ON comment(issue_id, created_at);

CREATE TABLE version (
  id           TEXT PRIMARY KEY,
  project_id   TEXT NOT NULL REFERENCES project(id),
  name         TEXT NOT NULL,
  release_date TEXT,
  released     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE user (
  account_id TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  avatar_url TEXT
);

CREATE TABLE field (
  id      TEXT PRIMARY KEY,               -- customfield_10042
  name    TEXT NOT NULL,
  type    TEXT,
  shown   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE sync_state (
  scope_id          TEXT PRIMARY KEY,     -- one row per sync scope, or per project for full syncs
  project_id        TEXT,
  jql               TEXT NOT NULL,
  last_delta_at     TEXT,
  last_full_at      TEXT,
  last_reconcile_at TEXT,
  approx_total      INTEGER,
  last_error        TEXT
);

-- rowid = the numeric issue id. Keeps its text so search can show snippets (docs/data.md).
CREATE VIRTUAL TABLE issue_fts USING fts5(
  key, summary, description, comments,
  tokenize = 'unicode61 remove_diacritics 2'
);
