//! Dragging an issue onto another group of a view: the drop changes the field the view groups by, when Taskboard
//! can write it and Jira allows it.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::transition::{TransitionOption, transitions};
use super::{Change, WriteOutcome, apply, issue_project};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::capability::Capability;
use crate::status_style::Style;
use crate::store::settings::Settings;
use crate::sync::source::Source;
use crate::team::Me;
use crate::views::GroupBy;
use crate::views::groups::GroupMark;
use crate::views::model::to_text;
use crate::views::rows::priority;

/// Where the dragged issue may go, each place with what a drop there does.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DropTargets {
    pub targets: Vec<DropTarget>,
    /// Why there are no targets, when that isn't Jira's choice.
    pub note: Option<String>,
}

/// A group of the view the drag starts in, in the view's order.
#[derive(Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct ViewGroup {
    pub key: String,
    pub label: String,
}

/// A group the issue may be dropped on, keyed like the view's groups (`views::groups`).
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DropTarget {
    pub key: String,
    pub label: String,
    /// "Set status to Code Review", "Assign to Jamie Doe".
    pub action: String,
    pub mark: Option<GroupMark>,
}

/// What a drop did, or the moves to choose from when several statuses of the dropped-on type can follow.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DropOutcome {
    Done { outcome: WriteOutcome },
    Choose { moves: Vec<TransitionOption> },
}

/// Status groups come from the moves Jira offers now, without the ones that need its screen; assignees from the
/// Space's active team; priorities from the issue's edit screen.
pub async fn drop_targets(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
    group_by: GroupBy,
) -> AppResult<DropTargets> {
    if let Some(capability) = drop_capability(group_by)
        && let Err(AppError::InvalidInput(why)) = super::ensure_allowed(settings, capability)
    {
        return Ok(DropTargets::noted(&why));
    }
    let targets = match group_by {
        GroupBy::Status => status_targets(&free_moves(db, source, issue_id).await?),
        GroupBy::StatusType => status_type_targets(&free_moves(db, source, issue_id).await?),
        GroupBy::Assignee => {
            assignee_targets(db, issue_id, crate::team::Me::of(&settings.connection)).await?
        }
        GroupBy::Priority => source
            .priorities(&issue_id)
            .await?
            .into_iter()
            .map(priority_target)
            .collect(),
        _ => {
            return Ok(DropTargets::noted(
                "Dragging changes the status, the assignee or the priority; group the view by one of them.",
            ));
        }
    };
    Ok(DropTargets {
        targets,
        note: None,
    })
}

/// What a drop changes when the view groups by `group_by`; none for groupings Taskboard can't change.
fn drop_capability(group_by: GroupBy) -> Option<Capability> {
    match group_by {
        GroupBy::Status | GroupBy::StatusType => Some(Capability::ChangeStatus),
        GroupBy::Assignee => Some(Capability::Assign),
        GroupBy::Priority => Some(Capability::ChangePriority),
        _ => None,
    }
}

impl DropTargets {
    fn noted(note: &str) -> Self {
        Self {
            targets: Vec::new(),
            note: Some(note.into()),
        }
    }
}

/// The places of a drag from the group `from_key`, never that group: the view's groups in its order, then places
/// the view has no group for (a teammate without issues here), which the page lists along the view's edge.
pub fn arrange(mut found: DropTargets, from_key: &str, groups: &[ViewGroup]) -> DropTargets {
    let rank = |key: &str| {
        groups
            .iter()
            .position(|g| g.key == key)
            .unwrap_or(groups.len())
    };
    found.targets.retain(|t| t.key != from_key);
    found.targets.sort_by_key(|t| rank(&t.key));
    found
}

fn status_targets(moves: &[TransitionOption]) -> Vec<DropTarget> {
    let mut targets: Vec<DropTarget> = Vec::new();
    for m in moves {
        if targets.iter().all(|t| t.key != m.status_id) {
            targets.push(DropTarget {
                key: m.status_id.clone(),
                label: m.status_name.clone(),
                action: format!("Set status to {}", m.status_name),
                mark: Some(GroupMark::Status { style: m.style }),
            });
        }
    }
    targets
}

/// One target per style the moves lead into, in the styles' order.
fn status_type_targets(moves: &[TransitionOption]) -> Vec<DropTarget> {
    let mut styles: Vec<Style> = moves.iter().map(|m| m.style).collect();
    styles.sort_by_key(|style| *style as i64);
    styles.dedup();
    styles
        .into_iter()
        .map(|style| {
            let names: Vec<&str> = moves
                .iter()
                .filter(|m| m.style == style)
                .map(|m| m.status_name.as_str())
                .collect();
            DropTarget {
                key: status_type_key(style),
                label: style.name().into(),
                action: format!("Set status to {}", either(&names)),
                mark: Some(GroupMark::Status { style }),
            }
        })
        .collect()
}

/// "A", "A or B", "A, B or C".
fn either(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, [])) => (*last).into(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

async fn assignee_targets(db: &Db, issue_id: String, me: Option<Me>) -> AppResult<Vec<DropTarget>> {
    db.read(move |conn| {
        let project = issue_project(conn, &issue_id)?;
        let people = crate::team::active(conn, &[project], me.as_ref())?;
        let mut targets: Vec<DropTarget> = people
            .into_iter()
            .map(|person| DropTarget {
                key: person.account_id.clone(),
                label: person.name.clone(),
                action: if Some(&person.account_id) == me.as_ref().map(|m| &m.account_id) {
                    "Assign to me".into()
                } else {
                    format!("Assign to {}", person.name)
                },
                mark: Some(GroupMark::Person { person }),
            })
            .collect();
        targets.push(DropTarget {
            key: String::new(),
            label: "Unassigned".into(),
            action: "Unassign".into(),
            mark: None,
        });
        Ok(targets)
    })
    .await
}

fn priority_target(name: String) -> DropTarget {
    DropTarget {
        key: name.clone(),
        label: name.clone(),
        action: format!("Set priority to {name}"),
        mark: Some(GroupMark::Priority {
            level: priority(&name).level,
        }),
    }
}

/// The key of a status-type group (`views::groups`).
fn status_type_key(style: Style) -> String {
    to_text(style)
}

/// Status and status-type drops take a move that doesn't need Jira's screen; an empty assignee key unassigns.
pub async fn drop_on(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
    group_by: GroupBy,
    key: String,
) -> AppResult<DropOutcome> {
    let change = match group_by {
        GroupBy::Assignee => Change::Assign {
            account_id: Some(key).filter(|k| !k.is_empty()),
        },
        GroupBy::Priority => Change::Priority { name: key },
        GroupBy::Status => {
            let moves = free_moves(db, source, issue_id.clone()).await?;
            let Some(chosen) = moves.into_iter().find(|m| m.status_id == key) else {
                return Err(not_allowed());
            };
            Change::Transition {
                transition_id: chosen.id,
            }
        }
        GroupBy::StatusType => {
            let mut moves: Vec<TransitionOption> = free_moves(db, source, issue_id.clone())
                .await?
                .into_iter()
                .filter(|m| status_type_key(m.style) == key)
                .collect();
            match moves.len() {
                0 => return Err(not_allowed()),
                1 => Change::Transition {
                    transition_id: moves.remove(0).id,
                },
                _ => return Ok(DropOutcome::Choose { moves }),
            }
        }
        _ => {
            return Err(AppError::InvalidInput(
                "Issues can't be moved between these groups".into(),
            ));
        }
    };
    let outcome = apply(db, source, settings, issue_id, change).await?;
    Ok(DropOutcome::Done { outcome })
}

/// The moves that don't need Jira's own screen.
async fn free_moves(
    db: &Db,
    source: &dyn Source,
    issue_id: String,
) -> AppResult<Vec<TransitionOption>> {
    Ok(transitions(db, source, issue_id)
        .await?
        .into_iter()
        .filter(|m| !m.opens_jira)
        .collect())
}

fn not_allowed() -> AppError {
    AppError::InvalidInput("Jira's workflow doesn't allow this move for the issue".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(key: &str) -> DropTarget {
        DropTarget {
            key: key.into(),
            label: key.to_uppercase(),
            action: format!("Set status to {key}"),
            mark: None,
        }
    }

    fn offering(keys: &[&str]) -> DropTargets {
        DropTargets {
            targets: keys.iter().map(|k| target(k)).collect(),
            note: None,
        }
    }

    fn groups(keys: &[&str]) -> Vec<ViewGroup> {
        keys.iter()
            .map(|k| ViewGroup {
                key: (*k).into(),
                label: format!("Group {k}"),
            })
            .collect()
    }

    fn keys(found: &DropTargets) -> Vec<&str> {
        found.targets.iter().map(|t| t.key.as_str()).collect()
    }

    #[test]
    fn the_issues_own_group_is_never_a_target() {
        let found = arrange(offering(&["a", "b", "c"]), "b", &groups(&["a", "b", "c"]));

        assert_eq!(keys(&found), ["a", "c"]);
    }

    #[test]
    fn the_own_group_is_dropped_even_when_the_view_has_no_such_group() {
        let found = arrange(offering(&["a", "b"]), "b", &groups(&["a"]));

        assert_eq!(keys(&found), ["a"]);
    }

    #[test]
    fn targets_follow_the_views_group_order() {
        let found = arrange(offering(&["a", "b", "c"]), "x", &groups(&["c", "a", "b"]));

        assert_eq!(keys(&found), ["c", "a", "b"]);
    }

    #[test]
    fn targets_without_a_view_group_come_last_in_their_original_order() {
        let found = arrange(
            offering(&["z", "b", "y", "a"]),
            "x",
            &groups(&["a", "b", "c"]),
        );

        assert_eq!(keys(&found), ["a", "b", "z", "y"]);
    }

    #[test]
    fn view_groups_that_are_no_targets_add_nothing() {
        let found = arrange(
            offering(&["b", "d"]),
            "a",
            &groups(&["e", "a", "d", "c", "b"]),
        );

        assert_eq!(keys(&found), ["d", "b"]);
    }

    #[test]
    fn with_a_note_there_are_no_targets() {
        let noted = DropTargets {
            targets: Vec::new(),
            note: Some("Group the view by status".into()),
        };

        let found = arrange(noted.clone(), "a", &groups(&["a", "b", "c"]));

        assert_eq!(found, noted);
    }

    #[test]
    fn the_actions_and_marks_of_the_targets_are_kept() {
        let found = arrange(offering(&["a", "b"]), "x", &groups(&["b", "a"]));

        assert_eq!(found.targets, [target("b"), target("a")]);
    }
}
