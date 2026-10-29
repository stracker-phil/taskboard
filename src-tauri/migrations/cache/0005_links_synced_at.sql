-- When an issue's web links were last fetched: opening it again within a few minutes shows the cached ones
-- without asking the tracker (`issues::web_links::FRESH_FOR`).
ALTER TABLE issue ADD COLUMN links_synced_at TEXT;
