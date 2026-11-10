-- Which custom fields show and download is the owner's choice (`local.field_choice`, ADR 0056), not a sample.
ALTER TABLE field DROP COLUMN used;
ALTER TABLE field DROP COLUMN shown;
