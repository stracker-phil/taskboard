//! How the sync runs: the scope and its intervals.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

const FOCUSED_MINUTES: (u32, u32) = (1, 60);
const BACKGROUND_MINUTES: (u32, u32) = (5, 120);
const KEEP_DONE_DAYS: (u32, u32) = (7, 365);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct SyncSettings {
    /// The owner's own scope JQL; `None` uses the default built from the tracked projects.
    pub scope_override: Option<String>,
    /// Delta sync interval while the window has focus.
    pub focused_minutes: u32,
    pub background_minutes: u32,
    /// How long finished issues stay in the default scope.
    pub keep_done_days: u32,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            scope_override: None,
            focused_minutes: 5,
            background_minutes: 15,
            keep_done_days: 90,
        }
    }
}

impl SyncSettings {
    pub fn validate(&self) -> AppResult<()> {
        check("sync.focusedMinutes", self.focused_minutes, FOCUSED_MINUTES)?;
        check(
            "sync.backgroundMinutes",
            self.background_minutes,
            BACKGROUND_MINUTES,
        )?;
        check("sync.keepDoneDays", self.keep_done_days, KEEP_DONE_DAYS)
    }

    /// Values a hand-edited file put out of bounds, moved to the nearest bound.
    pub fn clamped(self) -> Self {
        let clamp = |value: u32, (min, max): (u32, u32)| value.clamp(min, max);
        Self {
            focused_minutes: clamp(self.focused_minutes, FOCUSED_MINUTES),
            background_minutes: clamp(self.background_minutes, BACKGROUND_MINUTES),
            keep_done_days: clamp(self.keep_done_days, KEEP_DONE_DAYS),
            ..self
        }
    }
}

fn check(field: &str, value: u32, (min, max): (u32, u32)) -> AppResult<()> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(AppError::InvalidField {
            field: field.into(),
            message: format!("must be between {min} and {max}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sync(focused: u32, background: u32, keep_done: u32) -> SyncSettings {
        SyncSettings {
            focused_minutes: focused,
            background_minutes: background,
            keep_done_days: keep_done,
            ..SyncSettings::default()
        }
    }

    fn invalid_path(settings: &SyncSettings) -> String {
        match settings.validate() {
            Err(AppError::InvalidField { field, .. }) => field,
            other => panic!("expected an invalid field, got {other:?}"),
        }
    }

    #[test]
    fn the_defaults_are_valid() {
        assert_eq!(SyncSettings::default().validate(), Ok(()));
    }

    #[test]
    fn every_bound_itself_is_valid() {
        for (focused, background, keep_done) in [(1, 5, 7), (60, 120, 365)] {
            assert_eq!(
                sync(focused, background, keep_done).validate(),
                Ok(()),
                "{focused}/{background}/{keep_done}"
            );
        }
    }

    #[test]
    fn one_past_a_bound_names_the_field_in_error() {
        for (settings, path) in [
            (sync(0, 15, 90), "sync.focusedMinutes"),
            (sync(61, 15, 90), "sync.focusedMinutes"),
            (sync(5, 4, 90), "sync.backgroundMinutes"),
            (sync(5, 121, 90), "sync.backgroundMinutes"),
            (sync(5, 15, 6), "sync.keepDoneDays"),
            (sync(5, 15, 366), "sync.keepDoneDays"),
        ] {
            assert_eq!(invalid_path(&settings), path, "{settings:?}");
        }
    }

    #[test]
    fn the_message_of_an_invalid_field_gives_the_allowed_range() {
        assert_eq!(
            sync(0, 15, 90).validate(),
            Err(AppError::InvalidField {
                field: "sync.focusedMinutes".into(),
                message: "must be between 1 and 60".into(),
            })
        );
    }

    #[test]
    fn clamping_moves_out_of_range_values_to_the_nearest_bound() {
        assert_eq!(sync(0, 0, 0).clamped(), sync(1, 5, 7));
        assert_eq!(sync(1000, 1000, 1000).clamped(), sync(60, 120, 365));
    }

    #[test]
    fn clamping_keeps_valid_values_and_the_scope_override() {
        let settings = SyncSettings {
            scope_override: Some("project = NW".into()),
            ..sync(10, 30, 30)
        };
        assert_eq!(settings.clone().clamped(), settings);
    }

    #[test]
    fn sync_settings_use_camel_case_names_for_the_page() {
        assert_eq!(
            serde_json::to_value(SyncSettings::default()).unwrap(),
            json!({
                "scopeOverride": null,
                "focusedMinutes": 5,
                "backgroundMinutes": 15,
                "keepDoneDays": 90,
            })
        );
    }
}
