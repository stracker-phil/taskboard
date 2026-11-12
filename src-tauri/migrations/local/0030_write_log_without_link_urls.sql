-- The record of changes keeps no typed text in full (ADR 0066): a web link's URL shrinks to its first 10
-- characters and its length, as `writes::sent::typed` records new ones.
UPDATE write_log SET change = 'Web link: "' || substr(substr(change, 11), 1, 10)
    || CASE WHEN length(substr(change, 11)) > 10 THEN '…' ELSE '' END
    || '" (' || length(substr(change, 11)) || ' characters)'
  WHERE change LIKE 'Web link: %' AND change NOT LIKE 'Web link: "%';
