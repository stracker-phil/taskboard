-- Whether a recorded change counts in app (Settings → Notifications): 1 never makes its issue unread; NULL is
-- not decided yet and counts (`notify::rules::settle`).
ALTER TABLE issue_change ADD COLUMN quiet INTEGER;
CREATE INDEX issue_change_unsettled ON issue_change(id) WHERE quiet IS NULL;
-- A list field's item type (`user` for a multi user picker), so people fields can be told apart.
ALTER TABLE field ADD COLUMN items TEXT;
