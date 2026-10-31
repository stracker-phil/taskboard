-- People switched off in a Space's team (they left it); no row = active.
CREATE TABLE space_member_off (
  project_id TEXT NOT NULL,
  account_id TEXT NOT NULL,
  PRIMARY KEY (project_id, account_id)
);
