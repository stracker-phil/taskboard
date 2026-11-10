//! A Space's settings as the page and the views see them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::views::{GroupBy, Layout};

pub const ALL_LAYOUTS: [Layout; 3] = [Layout::List, Layout::Board, Layout::Table];

/// Where a custom field opted in app-wide shows in a Space; `Visible` (as app-wide) is the default.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum FieldVisibility {
    /// A column choice in the list and shown in the Issue view when filled.
    #[default]
    Visible,
    /// Shown in the Issue view, never offered as a column.
    HiddenInList,
    /// A column choice, left out of the Issue view.
    HiddenInDetail,
    Hidden,
}

impl FieldVisibility {
    pub fn in_list(self) -> bool {
        matches!(self, Self::Visible | Self::HiddenInDetail)
    }

    pub fn in_detail(self) -> bool {
        matches!(self, Self::Visible | Self::HiddenInList)
    }

    pub fn from_places(in_list: bool, in_detail: bool) -> Self {
        match (in_list, in_detail) {
            (true, true) => Self::Visible,
            (false, true) => Self::HiddenInList,
            (true, false) => Self::HiddenInDetail,
            (false, false) => Self::Hidden,
        }
    }
}

/// The layouts a Space's views offer, and how its new views start.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SpaceLayouts {
    /// The layouts its views can show, in List, Board, Table order; never empty.
    pub layouts: Vec<Layout>,
    /// One of `layouts`: what a new view gets, and what a view in a switched-off layout shows.
    pub default_layout: Layout,
    /// How new views group.
    pub default_group: GroupBy,
}

impl Default for SpaceLayouts {
    fn default() -> Self {
        Self {
            layouts: ALL_LAYOUTS.to_vec(),
            default_layout: Layout::List,
            default_group: GroupBy::Status,
        }
    }
}

impl SpaceLayouts {
    pub fn shown(&self, layout: Layout) -> Layout {
        if self.layouts.contains(&layout) {
            layout
        } else {
            self.default_layout
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct FieldSetting {
    pub id: String,
    pub name: String,
    /// Opted in app-wide for the list; `in_list` is the Space's own say, which only narrows it.
    pub app_list: bool,
    pub app_detail: bool,
    pub in_list: bool,
    pub in_detail: bool,
}

/// A Space's settings page, apart from its name and look (`SpaceUpdate`) and its statuses.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SpaceConfig {
    pub project_id: String,
    pub layouts: SpaceLayouts,
    /// The custom fields opted in app-wide, by name.
    pub fields: Vec<FieldSetting>,
}

/// Where a custom field shows: among the list's columns or in the Issue view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    List,
    Detail,
}
