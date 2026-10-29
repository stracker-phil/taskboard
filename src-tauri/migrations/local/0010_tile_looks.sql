-- Filter lists and views wear a tile like Spaces: an icon (letter, emoji or line icon name) on a tile color.
-- A list without a color shows its plain icon; a view without an icon shows its layout's.
ALTER TABLE filter_list ADD COLUMN color TEXT;
ALTER TABLE view ADD COLUMN icon TEXT;
ALTER TABLE view ADD COLUMN color TEXT;
