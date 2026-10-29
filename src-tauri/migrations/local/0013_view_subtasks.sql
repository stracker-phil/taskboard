-- How a view shows subtasks (ADR 0037): 'hidden', 'hierarchy' or 'list'. NULL reads as hidden.
ALTER TABLE view ADD COLUMN subtasks TEXT;
