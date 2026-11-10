//! A Space's own settings: the layouts its views offer, the layout and grouping of its new views, and where it
//! hides a custom field opted in app-wide (`space` columns, `space_field`). Status styles live in `status_style`.

pub mod commands;
mod edit;
mod read;
#[cfg(test)]
mod tests;
mod types;

pub use edit::{space_set_field, space_set_layouts};
pub use read::{
    hidden_columns, hidden_fields, hidden_in_every_space, level_layouts, project_of_level,
    space_config, space_layouts,
};
pub use types::{ALL_LAYOUTS, FieldSetting, FieldVisibility, Place, SpaceConfig, SpaceLayouts};
