-- Space settings: which layouts a Space offers, the layout and grouping of its new views, its status styles and how its
-- custom fields show.
ALTER TABLE space ADD COLUMN layouts TEXT;          -- JSON array of layouts (list | board | table); null = all
ALTER TABLE space ADD COLUMN default_layout TEXT;   -- the layout new views get; null = list
ALTER TABLE space ADD COLUMN default_group TEXT;    -- how new views group; null = status

CREATE TABLE space_status_style (                   -- overrides the detected status style in one Space
  project_id TEXT NOT NULL,
  status_id  TEXT NOT NULL,
  style      TEXT NOT NULL,                         -- todo | progress | review | blocked | done
  PRIMARY KEY (project_id, status_id)
);

-- The app-wide choices become every Space's; a Space without that status ignores its row.
INSERT INTO space_status_style (project_id, status_id, style)
  SELECT s.project_id, o.status_id, o.style FROM space s CROSS JOIN status_style o;
DROP TABLE status_style;

CREATE TABLE space_field (                          -- how a custom field shows in one Space; no row = visible
  project_id TEXT NOT NULL,
  field_id   TEXT NOT NULL,
  visibility TEXT NOT NULL,                         -- hiddenInList | hiddenInDetail | hidden
  PRIMARY KEY (project_id, field_id)
);
