-- Every view reads every issue row, so the big texts leave it (docs/data.md): the description and the issue
-- JSON move to `issue_body`, and the custom fields' values that views and filters read become a small
-- `custom` object. `raw` keeps only what no other column or table holds (`sync::raw`); the next fetch of an
-- issue writes it in exactly that shape.
CREATE TABLE issue_body (
  issue_id         TEXT PRIMARY KEY REFERENCES issue(id) ON DELETE CASCADE,
  description_html TEXT,
  description_adf  TEXT,
  raw              TEXT NOT NULL
);

INSERT INTO issue_body (issue_id, description_html, description_adf, raw)
SELECT id, description_html, description_adf, json_object(
  'id', json_extract(raw, '$.id'),
  'key', json_extract(raw, '$.key'),
  'fields', (SELECT json_group_object(key, value) FROM json_each(issue.raw, '$.fields')
             WHERE key NOT LIKE 'customfield\_%' ESCAPE '\' AND key NOT IN ('description', 'comment')),
  'renderedFields', (SELECT json_group_object(key, value) FROM json_each(issue.raw, '$.renderedFields')
                     WHERE key LIKE 'customfield\_%' ESCAPE '\' AND type = 'text'))
FROM issue;

ALTER TABLE issue ADD COLUMN custom TEXT;
UPDATE issue SET custom = (
  SELECT json_group_object(key, value) FROM json_each(issue.raw, '$.fields')
  WHERE key LIKE 'customfield\_%' ESCAPE '\' AND type <> 'null'
  HAVING COUNT(*) > 0);

ALTER TABLE issue DROP COLUMN description_html;
ALTER TABLE issue DROP COLUMN description_adf;
ALTER TABLE issue DROP COLUMN raw;

-- The palette, links and `taskboard open` find issues by key, or by the number after the dash.
CREATE INDEX issue_key ON issue(key COLLATE NOCASE);
CREATE INDEX issue_key_number ON issue(CAST(substr(key, instr(key, '-') + 1) AS INTEGER));
