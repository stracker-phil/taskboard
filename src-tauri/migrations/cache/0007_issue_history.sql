-- An issue's changelog (who changed which field when), fetched when the issue is opened, at most every few
-- minutes, and replaced as a whole each time (`issues::history`). Unlike `issue_change`, it never makes an issue
-- unread.
CREATE TABLE issue_history (
  issue_id   TEXT NOT NULL REFERENCES issue(id) ON DELETE CASCADE,
  position   INTEGER NOT NULL,              -- oldest first
  field      TEXT NOT NULL,                 -- the tracker's display name: status, assignee, Fix Version, Sprint
  field_id   TEXT,                          -- the tracker's field id: status, duedate, customfield_10020
  from_value TEXT,                          -- an id where the field has one (status, account, version)
  from_text  TEXT,                          -- the text the tracker shows
  to_value   TEXT,
  to_text    TEXT,
  author_id  TEXT,
  changed_at TEXT NOT NULL,                 -- UTC, like issue_change.changed_at
  source     TEXT NOT NULL DEFAULT 'jira',
  PRIMARY KEY (issue_id, position)
);
-- When an issue's history was last fetched (`issues::fetched::FRESH_FOR`).
ALTER TABLE issue ADD COLUMN history_synced_at TEXT;
