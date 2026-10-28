-- Whether a List view wraps long titles onto more lines (1, the default) or truncates them (0).

ALTER TABLE view ADD COLUMN wrap_titles INTEGER NOT NULL DEFAULT 1;
