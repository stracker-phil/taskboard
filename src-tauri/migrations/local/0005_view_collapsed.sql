-- A Board view's columns the owner collapsed or expanded, by status id (JSON object of booleans); a status
-- not in it uses the default (Done collapsed).

ALTER TABLE view ADD COLUMN collapsed TEXT;
