//! Changing a Space's settings: its layouts and its fields' visibility.

use rusqlite::params;

use super::read::config;
use super::types::{ALL_LAYOUTS, FieldVisibility, SpaceConfig, SpaceLayouts};
use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::views::Layout;
use crate::views::model::to_text;

pub async fn space_set_layouts(
    db: &Db,
    project_id: String,
    layouts: SpaceLayouts,
) -> AppResult<SpaceConfig> {
    if layouts.layouts.is_empty() {
        return Err(AppError::InvalidInput(
            "A Space needs at least one layout.".into(),
        ));
    }
    if !layouts.layouts.contains(&layouts.default_layout) {
        return Err(AppError::InvalidInput(
            "The default layout must be one the Space offers.".into(),
        ));
    }
    db.write(move |tx, changes| {
        config(tx, &project_id)?;
        let offered: Vec<Layout> = ALL_LAYOUTS
            .into_iter()
            .filter(|l| layouts.layouts.contains(l))
            .collect();
        let stored = (offered.len() < ALL_LAYOUTS.len())
            .then(|| serde_json::to_string(&offered))
            .transpose()?;
        tx.execute(
            "UPDATE space SET layouts = ?2, default_layout = ?3, default_group = ?4 WHERE project_id = ?1",
            params![
                project_id,
                stored,
                to_text(layouts.default_layout),
                to_text(layouts.default_group)
            ],
        )?;
        changes.local_changed(LocalPart::Spaces);
        changes.local_changed(LocalPart::Views);
        config(tx, &project_id)
    })
    .await
}

pub async fn space_set_field(
    db: &Db,
    project_id: String,
    field_id: String,
    visibility: FieldVisibility,
) -> AppResult<SpaceConfig> {
    db.write(move |tx, changes| {
        config(tx, &project_id)?;
        let known: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM cache.field WHERE id = ?1)",
            [&field_id],
            |r| r.get(0),
        )?;
        if !known {
            return Err(AppError::NotFound(format!(
                "Field {field_id} doesn't exist"
            )));
        }
        if visibility == FieldVisibility::Visible {
            tx.execute(
                "DELETE FROM space_field WHERE project_id = ?1 AND field_id = ?2",
                [&project_id, &field_id],
            )?;
        } else {
            tx.execute(
                "INSERT INTO space_field (project_id, field_id, visibility) VALUES (?1, ?2, ?3)
                 ON CONFLICT (project_id, field_id) DO UPDATE SET visibility = excluded.visibility",
                params![project_id, field_id, to_text(visibility)],
            )?;
        }
        changes.local_changed(LocalPart::Views);
        changes.local_changed(LocalPart::Spaces);
        config(tx, &project_id)
    })
    .await
}
