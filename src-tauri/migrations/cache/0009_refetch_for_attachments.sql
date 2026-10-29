-- The sync now asks for each issue's attachments. Dropping the delta cursor makes the next sync a full one,
-- so issues that didn't change since still get them; projects keep their cursors, so no change is recorded
-- as new.
UPDATE sync_state SET last_delta_at = NULL WHERE scope_id = 'scope';
