-- Bookmarks take an own name and the owner's order; the order starts as it was, newest first.
ALTER TABLE pin ADD COLUMN title TEXT;
ALTER TABLE pin ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
UPDATE pin SET position = (
  SELECT COUNT(*) FROM pin q
  WHERE q.created_at > pin.created_at OR (q.created_at = pin.created_at AND q.id > pin.id)
);
