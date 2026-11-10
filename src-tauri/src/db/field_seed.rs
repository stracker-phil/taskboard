//! One-time seed of `field_choice` from what the tracked Spaces showed before custom fields became opt-in
//! (`local/0027`).

use std::path::Path;

use rusqlite::Connection;

use super::connection::{attach_cache, cache_path, detach_cache};
use crate::error::AppResult;
use crate::filter::FIELDS_IN_USE;
use crate::views::columns::SHOWABLE;

pub fn seed_field_choices(local: &Connection, dir: &Path) -> AppResult<()> {
    let pending: bool = local.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'field_choice_seed')",
        [],
        |row| row.get(0),
    )?;
    if !pending {
        return Ok(());
    }
    if cache_path(dir).exists() {
        attach_cache(local, dir)?;
        let seeded = seed_choices(local).and_then(|()| prune_space_fields(local));
        detach_cache(local)?;
        seeded?;
    }
    local.execute_batch("DROP TABLE field_choice_seed")?;
    Ok(())
}

/// The tracked Spaces whose fields the owner set up (one hidden field is enough): every field showed in a Space
/// by default, so one left alone says nothing. Without any, every tracked Space.
const CHOOSING_SPACES: &str = "SELECT s.project_id FROM main.space s WHERE s.tracked = 1
    AND (EXISTS (SELECT 1 FROM main.space_field x WHERE x.project_id = s.project_id)
      OR NOT EXISTS (SELECT 1 FROM main.space_field x JOIN main.space t ON t.project_id = x.project_id
                     WHERE t.tracked = 1))";

/// A field the tracked projects fill, the owner shows or uses gets the places a Space that set up its fields
/// shows; one a view or filter uses is in the list.
fn seed_choices(conn: &Connection) -> AppResult<()> {
    let has = |column: &str| -> AppResult<bool> {
        Ok(conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM pragma_table_info('field', 'cache') WHERE name = ?1)",
            [column],
            |row| row.get(0),
        )?)
    };
    // Before the first usage sample `used` is 1 for every field of the site.
    let used = if has("used")? {
        "(used = 1 AND EXISTS (SELECT 1 FROM cache.field WHERE used = 0))"
    } else {
        "0"
    };
    let shown = if has("shown")? { "shown = 1" } else { "0" };
    let shows_in = |hidden: &str| {
        format!(
            "EXISTS (SELECT 1 FROM ({CHOOSING_SPACES}) s WHERE NOT EXISTS (
               SELECT 1 FROM main.space_field sf
               WHERE sf.project_id = s.project_id AND sf.field_id = cache.field.id
                 AND sf.visibility IN ('{hidden}', 'hidden')))"
        )
    };
    conn.execute_batch(&format!(
        "INSERT INTO main.field_choice (field_id, in_list, in_detail)
         SELECT id, in_list, in_detail FROM (
           SELECT id, {} OR id IN ({FIELDS_IN_USE}) AS in_list, {} AS in_detail FROM cache.field
           WHERE {SHOWABLE} AND ({used} OR {shown} OR id IN ({FIELDS_IN_USE}))
         ) WHERE in_list OR in_detail;",
        shows_in("hiddenInList"),
        shows_in("hiddenInDetail"),
    ))?;
    Ok(())
}

/// Space rows that no longer hide a place the app shows.
fn prune_space_fields(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "DELETE FROM main.space_field WHERE NOT EXISTS (
           SELECT 1 FROM main.field_choice c WHERE c.field_id = space_field.field_id
             AND ((c.in_list AND visibility IN ('hiddenInList', 'hidden'))
               OR (c.in_detail AND visibility IN ('hiddenInDetail', 'hidden'))))",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::db::connection::{local_path, prepare};
    use crate::db::migrate::{CACHE, LOCAL, migrate};
    use crate::test_support::temp_config_dir;

    const A: &str = "10001";
    const B: &str = "10002";
    const ZONE: &str = "customfield_10001";
    const ALPHA: &str = "customfield_10002";
    const BETA: &str = "customfield_10003";
    const DELTA: &str = "customfield_10004";
    const EPSILON: &str = "customfield_10005";
    const RANK: &str = "customfield_10019";

    type Choice = (String, bool, bool);

    fn choice(id: &str, in_list: bool, in_detail: bool) -> Choice {
        (id.to_string(), in_list, in_detail)
    }

    struct Lab {
        dir: TempDir,
        local: Connection,
    }

    /// A folder with the local file migrated and no cache file; `field_choice` still to be seeded.
    fn local_only() -> Lab {
        let dir = temp_config_dir();
        let mut local = Connection::open(local_path(dir.path())).unwrap();
        migrate(&mut local, "local", LOCAL).unwrap();
        Lab { dir, local }
    }

    /// A folder as an older Taskboard left it: the cache still has `field.used` and `field.shown`, and
    /// `field_choice` is still to be seeded.
    fn lab() -> Lab {
        let lab = local_only();
        let mut cache = Connection::open(cache_path(lab.dir.path())).unwrap();
        migrate(&mut cache, "cache", &CACHE[..14]).unwrap();
        lab
    }

    impl Lab {
        fn cache(&self, sql: &str) {
            Connection::open(cache_path(self.dir.path()))
                .unwrap()
                .execute_batch(sql)
                .unwrap();
        }

        fn field(&self, id: &str, kind: &str, used: i64, shown: i64) {
            self.cache(&format!(
                "INSERT INTO field (id, name, type, used, shown) VALUES ('{id}', 'Field {id}', '{kind}', {used}, {shown})"
            ));
        }

        fn space(&self, project: &str, tracked: i64) {
            self.local
                .execute(
                    "INSERT INTO space (project_id, tracked) VALUES (?1, ?2)",
                    rusqlite::params![project, tracked],
                )
                .unwrap();
        }

        fn hide(&self, project: &str, field: &str, visibility: &str) {
            self.local
                .execute(
                    "INSERT INTO space_field (project_id, field_id, visibility) VALUES (?1, ?2, ?3)",
                    [project, field, visibility],
                )
                .unwrap();
        }

        fn seed(&self) {
            seed_field_choices(&self.local, self.dir.path()).unwrap();
        }

        fn triples<A, B, C>(&self, sql: &str) -> Vec<(A, B, C)>
        where
            A: rusqlite::types::FromSql,
            B: rusqlite::types::FromSql,
            C: rusqlite::types::FromSql,
        {
            let mut query = self.local.prepare(sql).unwrap();
            query
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        }

        fn choices(&self) -> Vec<Choice> {
            self.triples("SELECT field_id, in_list, in_detail FROM field_choice ORDER BY field_id")
        }

        fn space_fields(&self) -> Vec<(String, String, String)> {
            self.triples("SELECT project_id, field_id, visibility FROM space_field ORDER BY 1, 2")
        }

        fn marker_exists(&self) -> bool {
            self.local
                .query_row(
                    "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE name = 'field_choice_seed')",
                    [],
                    |r| r.get(0),
                )
                .unwrap()
        }
    }

    fn row(project: &str, field: &str, visibility: &str) -> (String, String, String) {
        (project.into(), field.into(), visibility.into())
    }

    #[test]
    fn a_field_a_space_that_set_up_its_fields_kept_is_opted_in_for_both_places() {
        let lab = lab();
        lab.space(A, 1);
        lab.space(B, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.field(ALPHA, "string", 0, 1);
        lab.hide(A, ALPHA, "hidden");

        lab.seed();

        assert_eq!(lab.choices(), [choice(ZONE, true, true)]);
        assert_eq!(lab.space_fields(), []);
    }

    #[test]
    fn a_space_that_never_hid_a_field_says_nothing_once_another_space_set_up_its_fields() {
        let lab = lab();
        lab.space(A, 1);
        lab.space(B, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.hide(A, ZONE, "hidden");

        lab.seed();

        assert_eq!(lab.choices(), []);
    }

    #[test]
    fn without_any_set_up_space_every_tracked_space_counts() {
        let lab = lab();
        lab.space(A, 1);
        lab.space(B, 1);
        lab.field(ZONE, "string", 0, 1);

        lab.seed();

        assert_eq!(lab.choices(), [choice(ZONE, true, true)]);
    }

    #[test]
    fn a_field_hidden_in_the_set_up_space_but_used_by_a_view_column_keeps_its_list_box_only() {
        let lab = lab();
        lab.space(A, 1);
        lab.space(B, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.hide(A, ZONE, "hidden");
        lab.local
            .execute_batch(&format!(
                r#"INSERT INTO filter_list (id, kind, name) VALUES (900, 'list', 'Open work');
                   INSERT INTO view (list_id, name, layout, columns, group_by, sort_by, sort_order) VALUES
                     (900, 'Table', 'table', '["status","{ZONE}"]', 'none', 'updated', 0)"#
            ))
            .unwrap();

        lab.seed();

        assert_eq!(lab.choices(), [choice(ZONE, true, false)]);
        assert_eq!(lab.space_fields(), [row(A, ZONE, "hidden")]);
    }

    #[test]
    fn a_field_is_opted_in_only_for_the_places_a_tracked_space_showed_it() {
        let lab = lab();
        lab.space(A, 1);
        lab.space(B, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.field(ALPHA, "string", 0, 1);
        lab.field(BETA, "string", 0, 1);
        for project in [A, B] {
            lab.hide(project, ZONE, "hiddenInList");
            lab.hide(project, ALPHA, "hiddenInDetail");
        }
        lab.hide(A, BETA, "hiddenInList");
        lab.hide(B, BETA, "hiddenInDetail");

        lab.seed();

        assert_eq!(
            lab.choices(),
            [
                choice(ZONE, false, true),
                choice(ALPHA, true, false),
                choice(BETA, true, true),
            ]
        );
    }

    #[test]
    fn a_field_hidden_everywhere_gets_no_choice() {
        let lab = lab();
        lab.space(A, 1);
        lab.space(B, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.hide(A, ZONE, "hidden");
        lab.hide(B, ZONE, "hidden");

        lab.seed();

        assert_eq!(lab.choices(), []);
    }

    #[test]
    fn untracked_spaces_do_not_count_as_showing_a_field() {
        let lab = lab();
        lab.space(A, 0);
        lab.field(ZONE, "string", 0, 1);

        lab.seed();

        assert_eq!(lab.choices(), []);
    }

    #[test]
    fn a_field_nobody_uses_shows_or_references_gets_no_choice() {
        let lab = lab();
        lab.space(A, 1);
        lab.field(ZONE, "string", 0, 0);
        lab.field(ALPHA, "string", 0, 0);
        lab.field(RANK, "any", 0, 1);

        lab.seed();

        assert_eq!(lab.choices(), []);
    }

    #[test]
    fn before_the_first_usage_sample_a_field_counts_as_used_only_with_another_reason() {
        let lab = lab();
        lab.space(A, 1);
        lab.field(ZONE, "string", 1, 0);
        lab.field(ALPHA, "string", 1, 0);

        lab.seed();

        assert_eq!(lab.choices(), []);
    }

    #[test]
    fn fields_the_sample_a_column_or_a_filter_use_are_opted_in() {
        let lab = lab();
        lab.space(A, 1);
        lab.field(ZONE, "string", 1, 0);
        lab.field(ALPHA, "string", 0, 0);
        lab.field(BETA, "string", 0, 0);
        lab.field(DELTA, "string", 0, 0);
        lab.field(EPSILON, "string", 0, 0);
        lab.local
            .execute_batch(&format!(
                r#"INSERT INTO filter_list (id, kind, name, filter) VALUES
                     (900, 'list', 'Open work', '{{"conditions":[{{"field":"{BETA}","op":"is","value":"x"}}]}}');
                   INSERT INTO view (list_id, name, layout, columns, group_by, sort_by, sort_order, filter) VALUES
                     (900, 'Table', 'table', '["status","{ALPHA}"]', 'none', 'updated', 0,
                      '{{"conditions":[{{"field":"{DELTA}","op":"is","value":"x"}}]}}')"#
            ))
            .unwrap();

        lab.seed();

        assert_eq!(
            lab.choices(),
            [
                choice(ZONE, true, true),
                choice(ALPHA, true, true),
                choice(BETA, true, true),
                choice(DELTA, true, true),
            ]
        );
    }

    #[test]
    fn space_rows_that_no_longer_narrow_anything_go_and_the_ones_that_do_stay() {
        let lab = lab();
        lab.space(A, 1);
        lab.space(B, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.field(ALPHA, "string", 0, 1);
        lab.field(BETA, "string", 0, 0);
        lab.hide(A, ZONE, "hiddenInList");
        for project in [A, B] {
            lab.hide(project, ALPHA, "hiddenInList");
        }
        lab.hide(A, BETA, "hidden");

        lab.seed();

        assert_eq!(
            lab.choices(),
            [choice(ZONE, true, true), choice(ALPHA, false, true)]
        );
        assert_eq!(lab.space_fields(), [row(A, ZONE, "hiddenInList")]);
    }

    #[test]
    fn without_a_cache_file_nothing_is_seeded_and_the_marker_goes() {
        let lab = local_only();
        lab.space(A, 1);
        lab.hide(A, ZONE, "hidden");
        assert!(lab.marker_exists());

        lab.seed();

        assert_eq!(lab.choices(), []);
        assert_eq!(lab.space_fields(), [row(A, ZONE, "hidden")]);
        assert!(!lab.marker_exists());
    }

    #[test]
    fn the_seed_runs_once_and_a_cleared_choice_stays_cleared() {
        let lab = lab();
        lab.space(A, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.seed();
        assert_eq!(lab.choices(), [choice(ZONE, true, true)]);
        assert!(!lab.marker_exists());
        lab.local.execute("DELETE FROM field_choice", []).unwrap();

        lab.seed();

        assert_eq!(lab.choices(), []);
    }

    #[test]
    fn preparing_a_folder_seeds_it_once_and_opening_it_again_keeps_the_choices() {
        let lab = lab();
        lab.space(A, 1);
        lab.field(ZONE, "string", 0, 1);
        lab.field(ALPHA, "string", 0, 1);

        prepare(lab.dir.path()).unwrap();

        assert_eq!(
            lab.choices(),
            [choice(ZONE, true, true), choice(ALPHA, true, true)]
        );
        lab.local
            .execute("DELETE FROM field_choice WHERE field_id = ?1", [ALPHA])
            .unwrap();

        prepare(lab.dir.path()).unwrap();

        assert_eq!(lab.choices(), [choice(ZONE, true, true)]);
    }

    #[test]
    fn a_cache_from_before_usage_sampling_seeds_from_the_shown_fields() {
        let lab = local_only();
        let mut cache = Connection::open(cache_path(lab.dir.path())).unwrap();
        migrate(&mut cache, "cache", &CACHE[..1]).unwrap();
        cache
            .execute_batch(&format!(
                "INSERT INTO field (id, name, type, shown) VALUES
                   ('{ZONE}', 'Zone', 'string', 1),
                   ('{ALPHA}', 'Alpha', 'string', 0)"
            ))
            .unwrap();
        drop(cache);
        lab.space(A, 1);

        lab.seed();

        assert_eq!(lab.choices(), [choice(ZONE, true, true)]);
    }

    #[test]
    fn a_cache_without_the_usage_marks_seeds_only_the_fields_a_view_or_filter_uses() {
        let lab = local_only();
        let mut cache = Connection::open(cache_path(lab.dir.path())).unwrap();
        migrate(&mut cache, "cache", CACHE).unwrap();
        cache
            .execute_batch(&format!(
                "INSERT INTO field (id, name, type) VALUES
                   ('{ZONE}', 'Zone', 'string'), ('{ALPHA}', 'Alpha', 'string')"
            ))
            .unwrap();
        drop(cache);
        lab.space(A, 1);
        lab.local
            .execute_batch(&format!(
                r#"INSERT INTO filter_list (id, kind, name, filter) VALUES
                     (900, 'list', 'Open work', '{{"conditions":[{{"field":"{ALPHA}","op":"is","value":"x"}}]}}')"#
            ))
            .unwrap();

        lab.seed();

        assert_eq!(lab.choices(), [choice(ALPHA, true, true)]);
    }
}
