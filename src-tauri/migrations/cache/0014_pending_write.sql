-- A change Taskboard sent to the tracker and hasn't settled yet (`writes/`). While it is here, the sync keeps
-- `value` over what it fetches (so nothing flickers back) and keeps the fetched value in `base`, which a failed
-- write puts back.
CREATE TABLE pending_write (
  issue_id   TEXT NOT NULL REFERENCES issue(id) ON DELETE CASCADE,
  field      TEXT NOT NULL,          -- writes::Field
  value      TEXT,                   -- what was sent: an account id; null = none
  base       TEXT,                   -- what the tracker last had
  created_at TEXT NOT NULL,
  PRIMARY KEY (issue_id, field)
);
