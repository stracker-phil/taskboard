# 0056. Custom fields are opted in app-wide; a Space opts out

## Status

Accepted (2026-11-10). Owner's request after several weeks of use. Refines
[0030](0030-a-space-carries-its-own-settings.md) (where a field shows) and
[0041](0041-an-issues-big-texts-live-beside-its-row.md) (which custom values the cache holds).

## Context

Every Space offered every custom field the tracked projects fill, and the full sync sampled each project's 100
newest issues with all their fields (`*all`) to find them. A site has hundreds of fields; most are noise, and
downloading them puts company data on disk that the owner never looks at.

## Decision

- `field_choice` (migration `local/0027`) holds where a custom field shows in every Space: in the list (a
  column and filter choice) and in the Issue view. No row means hidden everywhere. Settings → Custom fields edits
  it; a filter on a field puts it in the list.
- `space_field` only narrows that choice in one Space (`space_config::hidden_fields` = not opted in ∪ the
  Space's opt-outs). A Space's Custom fields section shows a box only where the app shows the field.
- The first start with `local/0027` fills `field_choice` once from what the Spaces showed (`db/field_seed.rs`).
- The sync downloads only the fields opted in or in use by a view column or a filter (`sync::fetched_fields`).
  The field-usage sample and its `*all` search are gone, and `cache/0015` drops `field.used` and `field.shown`.
- The Issue view lists the opted-in fields an issue fills, by name; an empty field doesn't show.

## Consequences

- No custom field reaches the disk until the owner picks it; a new field on the site stays hidden.
- Picking a field that wasn't downloaded yet starts a full sync (`field_choice_set`), which fetches its values.
