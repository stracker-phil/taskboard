-- Custom fields are opted in app-wide (ADR 0056); `space_field` only narrows that choice in one Space.
CREATE TABLE field_choice (                         -- no row = hidden everywhere and never downloaded
  field_id  TEXT PRIMARY KEY,
  in_list   INTEGER NOT NULL,                       -- offered as a column and a filter field
  in_detail INTEGER NOT NULL                        -- shown in the Issue view when filled
);

-- Present until `db::field_seed` has filled `field_choice` from the cache once.
CREATE TABLE field_choice_seed (pending INTEGER);
INSERT INTO field_choice_seed VALUES (1);
