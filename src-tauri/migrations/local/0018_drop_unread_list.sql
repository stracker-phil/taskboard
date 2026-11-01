-- Unread is a mode of every view now (modes.json), not a list of its own.
DELETE FROM pin WHERE target_kind = 'filter_list'
  AND target_id IN (SELECT CAST(id AS TEXT) FROM filter_list WHERE kind = 'recent');
DELETE FROM pin WHERE target_kind = 'view'
  AND target_id IN (SELECT CAST(v.id AS TEXT) FROM view v JOIN filter_list l ON l.id = v.list_id WHERE l.kind = 'recent');
DELETE FROM view WHERE list_id IN (SELECT id FROM filter_list WHERE kind = 'recent');
DELETE FROM filter_list WHERE kind = 'recent';
