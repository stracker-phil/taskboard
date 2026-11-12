-- Who made a recorded change (`docs/notifications.md`, Authors); NULL is unknown and counts.
ALTER TABLE issue_change ADD COLUMN author_id TEXT;
-- Jira's `accountType`: atlassian, app (an automation) or customer; NULL is unknown.
ALTER TABLE user ADD COLUMN account_type TEXT;
