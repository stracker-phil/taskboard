-- A comment's body as Atlassian Document Format, rendered by `adf` instead of Jira's lossy HTML. Rows from
-- before this migration get it on the next sync; until then the page shows `body_html`.
ALTER TABLE comment ADD COLUMN body_adf TEXT;
