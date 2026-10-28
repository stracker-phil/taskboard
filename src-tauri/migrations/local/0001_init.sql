-- The owner's own setup (docs/data.md): kept when the cache is reset. Project ids point into the cache file,
-- which SQLite can't enforce across files; code removes what a vanished project leaves behind.

CREATE TABLE space (                      -- a tracked Jira project as the sidebar shows it
  project_id TEXT PRIMARY KEY,
  tracked    INTEGER NOT NULL DEFAULT 1,
  name       TEXT,                        -- display name; null = the project's name
  icon       TEXT,                        -- one letter
  color      TEXT,                        -- a Space tile token number
  client     TEXT,                        -- the owner's label, e.g. a client; null = internal
  position   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE filter_list (                -- WHICH issues: one node of the sidebar tree
  id         INTEGER PRIMARY KEY,
  parent_id  INTEGER REFERENCES filter_list(id) ON DELETE CASCADE,
  kind       TEXT NOT NULL,               -- everything | space | list | recent | pinned
  project_id TEXT,                        -- set for kind = space
  name       TEXT NOT NULL,
  filter     TEXT,                        -- JSON filter, ANDed with all ancestors
  position   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE view (                       -- HOW to show a filter list: one tab; never filters
  id         INTEGER PRIMARY KEY,
  list_id    INTEGER NOT NULL REFERENCES filter_list(id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  icon_color TEXT,
  layout     TEXT NOT NULL,               -- list | board | table
  columns    TEXT,                        -- JSON: columns or board card fields
  position   INTEGER NOT NULL DEFAULT 0,
  is_default INTEGER NOT NULL DEFAULT 0,
  group_by   TEXT NOT NULL,
  sort_by    TEXT NOT NULL,
  sort_order INTEGER NOT NULL
);

CREATE TABLE pin (                        -- local pins, never synced to Jira
  id          INTEGER PRIMARY KEY,
  target_kind TEXT NOT NULL,              -- issue | comment | filter_list
  target_id   TEXT NOT NULL,
  reason      TEXT,
  expires_at  TEXT,                       -- null = until unpinned
  created_at  TEXT NOT NULL,
  unpinned_at TEXT
);

CREATE TABLE status_style (               -- overrides the detected status style
  status_id TEXT PRIMARY KEY,
  style     TEXT NOT NULL                 -- todo | progress | review | blocked | done
);
