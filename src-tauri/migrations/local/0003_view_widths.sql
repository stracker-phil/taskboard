-- The Table layout's column widths per view (docs/data.md).

ALTER TABLE view ADD COLUMN widths TEXT;  -- JSON: column id → width in points; a missing id uses the default
