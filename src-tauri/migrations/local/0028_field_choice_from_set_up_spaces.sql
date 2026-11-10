-- `local/0027`'s seed counted a Space that never hid a field as showing every field, so most fields got boxes.
-- Only Spaces whose fields the owner set up count (`db/field_seed.rs` CHOOSING_SPACES); a field a view or filter
-- uses keeps its list box. A database that seeds after this migration gets the same rule from the seed.
UPDATE field_choice SET
  in_list = (in_list AND EXISTS (
      SELECT 1 FROM space s WHERE s.tracked = 1
        AND EXISTS (SELECT 1 FROM space_field x WHERE x.project_id = s.project_id)
        AND NOT EXISTS (SELECT 1 FROM space_field sf WHERE sf.project_id = s.project_id
                          AND sf.field_id = field_choice.field_id AND sf.visibility IN ('hiddenInList', 'hidden'))))
    OR field_id IN (
      SELECT c.value FROM view v, json_each(v.columns) c
      UNION SELECT json_extract(c.value, '$.field') FROM filter_list l, json_each(l.filter, '$.conditions') c
      UNION SELECT json_extract(c.value, '$.field') FROM view v, json_each(v.filter, '$.conditions') c),
  in_detail = in_detail AND EXISTS (
      SELECT 1 FROM space s WHERE s.tracked = 1
        AND EXISTS (SELECT 1 FROM space_field x WHERE x.project_id = s.project_id)
        AND NOT EXISTS (SELECT 1 FROM space_field sf WHERE sf.project_id = s.project_id
                          AND sf.field_id = field_choice.field_id AND sf.visibility IN ('hiddenInDetail', 'hidden')))
WHERE EXISTS (SELECT 1 FROM space_field x JOIN space t ON t.project_id = x.project_id WHERE t.tracked = 1);
DELETE FROM field_choice WHERE NOT in_list AND NOT in_detail;
DELETE FROM space_field WHERE NOT EXISTS (
  SELECT 1 FROM field_choice c WHERE c.field_id = space_field.field_id
    AND ((c.in_list AND visibility IN ('hiddenInList', 'hidden'))
      OR (c.in_detail AND visibility IN ('hiddenInDetail', 'hidden'))));
