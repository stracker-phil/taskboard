-- Status styles are chosen app-wide; `space_status_style` only overrides them in one Space.
CREATE TABLE status_style (
  status_id TEXT PRIMARY KEY,
  style     TEXT NOT NULL                           -- progress | review | blocked | todo | backlog | done
);

-- A status every overriding Space styled alike gets that style app-wide; conflicting overrides stay per Space.
INSERT INTO status_style (status_id, style)
  SELECT status_id, MIN(style) FROM space_status_style GROUP BY status_id HAVING COUNT(DISTINCT style) = 1;
DELETE FROM space_status_style WHERE status_id IN (SELECT status_id FROM status_style);
