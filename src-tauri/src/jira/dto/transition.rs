use serde::Deserialize;

use super::Status;

/// `GET issue/{id}/transitions`: the moves the workflow allows from the issue's status.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Transitions {
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub id: String,
    /// The workflow's name for the move ("Start progress"), often the target status's.
    pub name: String,
    pub to: Status,
    /// The move asks for more fields on a screen, which Taskboard doesn't show.
    #[serde(default)]
    pub has_screen: bool,
}
