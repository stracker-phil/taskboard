-- The built-in levels of the sidebar tree with their first views, and the Space's columns for new views
-- (docs/data.md). Spaces and their filter lists are made when a project is tracked.

ALTER TABLE space ADD COLUMN default_columns TEXT;  -- JSON column ids for new views; null = the app's defaults

INSERT INTO filter_list (kind, name, position) VALUES
  ('everything', 'Everything', 0),
  ('recent', 'Recently changed', 0),
  ('pinned', 'Pinned', 1);

INSERT INTO view (list_id, name, layout, columns, position, is_default, group_by, sort_by, sort_order)
SELECT id, 'All issues', 'list', '["assignee","epic","release","priority"]', 0, 1,
       CASE kind WHEN 'pinned' THEN 'none' ELSE 'project' END, 'updated', -1
FROM filter_list;
