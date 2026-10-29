-- Marked unread by the owner: the issue counts as unread until it is read again, changes or not.
ALTER TABLE issue_seen ADD COLUMN unread INTEGER NOT NULL DEFAULT 0;
