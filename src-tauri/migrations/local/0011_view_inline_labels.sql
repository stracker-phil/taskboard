-- Whether a List or Table view shows an issue's labels as tags after its title (1, the default) while the view
-- has no Labels column.
ALTER TABLE view ADD COLUMN inline_labels INTEGER NOT NULL DEFAULT 1;
