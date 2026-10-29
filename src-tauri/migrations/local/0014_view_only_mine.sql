-- The toolbar's Me chip, saved with the view (ADR 0038): 1 shows only the connected account's issues.
ALTER TABLE view ADD COLUMN only_mine INTEGER NOT NULL DEFAULT 0;
