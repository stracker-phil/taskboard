-- The Me rules a view has on, as a JSON list of `MeMatch` (ADR 0051); none follows the app's (Settings → Me Mode).
ALTER TABLE view ADD COLUMN me_rules TEXT;
