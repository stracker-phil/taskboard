-- Whether a tracked project's recent issues fill the field; 1 until the first sync has looked.
ALTER TABLE field ADD COLUMN used INTEGER NOT NULL DEFAULT 1;
