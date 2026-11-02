-- Recently viewed (`viewed`): a Home page right after Everything of the issues the owner opened in the Issue
-- view (`issue_activity.viewed_at`), newest first. Its one condition can't be removed; only its time window
-- changes (`lists::edit::list_set_window`). Position -1 keeps it ahead of Bookmarks and the global lists.
INSERT INTO filter_list (kind, name, position, filter) VALUES
  ('viewed', 'Recently viewed', -1,
   '{"conditions":[{"field":"lastView","op":"withinLastHours","value":24}]}');

INSERT INTO view (list_id, name, layout, columns, position, is_default, group_by, sort_by, sort_order, subtasks)
SELECT id, 'All issues', 'list', '["lastView","assignee","status","priority"]', 0, 1, 'none', 'lastView', -1, 'hidden'
FROM filter_list WHERE kind = 'viewed';
