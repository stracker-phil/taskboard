-- Where a click on an issue opens it in this view: 'preview' or 'full'; NULL follows Settings → General.
ALTER TABLE view ADD COLUMN open_with TEXT;
