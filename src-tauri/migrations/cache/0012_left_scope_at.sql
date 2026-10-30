-- When reconciliation last found a live issue outside the sync scope (done longer than the scope keeps, moved to
-- an untracked project). It is asked for again only after a while (`sync::reconcile`), not every hour.
ALTER TABLE issue ADD COLUMN left_scope_at TEXT;
