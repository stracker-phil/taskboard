-- Every mirrored row says which tracker it comes from (ADR 0029); the project table becomes `space`, the app's
-- word for a tracked project or repository.
ALTER TABLE project RENAME TO space;
ALTER TABLE space ADD COLUMN source TEXT NOT NULL DEFAULT 'jira';
ALTER TABLE issue ADD COLUMN source TEXT NOT NULL DEFAULT 'jira';
ALTER TABLE comment ADD COLUMN source TEXT NOT NULL DEFAULT 'jira';
