//! The toolbar's narrowing of a view and the Filter popover's drafts, as the page sends them, with what Rust adds.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::filter::Condition;
use crate::me_mode::{self, MeMatch, MeRule};

/// The toolbar's narrowing on top of the level's filter, and the Filter popover's unsaved conditions (ADR 0038).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Narrowing {
    /// Only my issues (Me mode).
    pub me: bool,
    /// Key or summary contains it, ignoring case and accents.
    pub text: String,
    /// Only children of this epic.
    pub epic: Option<String>,
    /// Only issues assigned to this account (the person chip).
    pub person: Option<String>,
    /// Unsaved conditions in place of the filter list's own ones; only a filter list has its own.
    pub list_draft: Option<Vec<Condition>>,
    /// Unsaved conditions in place of the view's own ones.
    pub view_draft: Option<Vec<Condition>>,
    /// Only unread issues, and the ones read since the owner came here (`read_here`): Unread only is on.
    #[serde(skip)]
    #[ts(skip)]
    pub unread: bool,
    /// Unread only keeps these read issues (read since the owner came here); set by Rust, never by the page.
    #[serde(skip)]
    #[ts(skip)]
    pub read_here: Vec<String>,
    /// The Me rules this view has on, drafted or saved (ADR 0051); `None` follows the app's.
    pub me_on: Option<Vec<MeMatch>>,
    /// What Me mode counts as mine besides my assigned issues (`resolve_me_rules`).
    #[serde(skip)]
    #[ts(skip)]
    pub me_rules: Vec<MeRule>,
}

impl Narrowing {
    /// Me mode's rules for these rows: the view's own while the view itself is in Me mode, else the app's, which
    /// "For all views" uses.
    pub fn resolve_me_rules(&self, app: &[MeRule]) -> Vec<MeRule> {
        let own = self.me_on.as_deref().filter(|_| self.me);
        me_mode::rules_for(app, own)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_rules() -> Vec<MeRule> {
        vec![
            MeRule {
                rule: MeMatch::Mentioned,
                on: false,
            },
            MeRule {
                rule: MeMatch::Subtasks,
                on: true,
            },
        ]
    }

    fn switched(mentioned: bool, subtasks: bool) -> Vec<MeRule> {
        vec![
            MeRule {
                rule: MeMatch::Mentioned,
                on: mentioned,
            },
            MeRule {
                rule: MeMatch::Subtasks,
                on: subtasks,
            },
        ]
    }

    #[test]
    fn a_view_in_me_mode_with_its_own_rules_uses_them_instead_of_the_apps() {
        let narrowing = Narrowing {
            me: true,
            me_on: Some(vec![MeMatch::Mentioned]),
            ..Narrowing::default()
        };

        assert_eq!(
            narrowing.resolve_me_rules(&app_rules()),
            switched(true, false)
        );
    }

    #[test]
    fn a_view_in_me_mode_with_an_empty_own_list_has_every_rule_off() {
        let narrowing = Narrowing {
            me: true,
            me_on: Some(vec![]),
            ..Narrowing::default()
        };

        assert_eq!(
            narrowing.resolve_me_rules(&app_rules()),
            switched(false, false)
        );
    }

    #[test]
    fn a_view_in_me_mode_without_own_rules_follows_the_apps() {
        let narrowing = Narrowing {
            me: true,
            ..Narrowing::default()
        };

        assert_eq!(narrowing.resolve_me_rules(&app_rules()), app_rules());
    }

    #[test]
    fn a_view_not_in_me_mode_follows_the_apps_rules_whatever_it_saved() {
        let narrowing = Narrowing {
            me: false,
            me_on: Some(vec![MeMatch::Mentioned]),
            ..Narrowing::default()
        };

        assert_eq!(narrowing.resolve_me_rules(&app_rules()), app_rules());
    }

    #[test]
    fn the_pages_own_rules_come_from_the_me_on_field_in_camel_case() {
        let narrowing: Narrowing = serde_json::from_value(
            serde_json::json!({"me": true, "meOn": [{"kind": "mentioned"}]}),
        )
        .unwrap();

        assert_eq!(narrowing.me_on, Some(vec![MeMatch::Mentioned]));
        assert_eq!(
            serde_json::from_value::<Narrowing>(serde_json::json!({}))
                .unwrap()
                .me_on,
            None
        );
    }
}
