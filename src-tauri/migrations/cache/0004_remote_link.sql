-- An issue's web links (Jira's remote links: pull requests, documents), fetched when the issue is opened and
-- replaced as a whole each time. Every row says which tracker it comes from (ADR 0029).
CREATE TABLE remote_link (
  issue_id     TEXT NOT NULL REFERENCES issue(id) ON DELETE CASCADE,
  id           TEXT NOT NULL,             -- the tracker's link id
  position     INTEGER NOT NULL,          -- the tracker's order
  url          TEXT NOT NULL,             -- untrusted: opened only when https (`issues::web_links`)
  title        TEXT,
  application  TEXT,                      -- the tool it points into: application name, else type
  relationship TEXT,
  source       TEXT NOT NULL DEFAULT 'jira',
  PRIMARY KEY (issue_id, id)
);
