-- Who is on a Space's team is decided per Space: someone ever assigned one of its issues is on it unless switched
-- off, anyone else only when switched on. A row is the owner's choice; no row = the default.
CREATE TABLE space_member (
  project_id TEXT NOT NULL,
  account_id TEXT NOT NULL,
  active     INTEGER NOT NULL,
  PRIMARY KEY (project_id, account_id)
);
INSERT INTO space_member (project_id, account_id, active) SELECT project_id, account_id, 0 FROM space_member_off;
DROP TABLE space_member_off;
