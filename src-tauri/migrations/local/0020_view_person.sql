-- A view saves its person chip, like its Me chip: only the issues assigned to that account.
ALTER TABLE view ADD COLUMN person TEXT;
