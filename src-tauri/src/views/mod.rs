//! The view tabs of a sidebar level (ADR 0024): layout, grouping, sort and columns, never a filter.

pub mod columns;
pub mod commands;
mod edit;
pub mod groups;
pub mod model;
pub mod query;
pub mod rows;
mod sorting;

pub use columns::{Column, columns_catalogue};
pub use edit::{
    clean_name, columns_for_new_view, copy_views, insert_view, list_views, read_view,
    space_default_columns, view_create, view_delete, view_duplicate, view_move, view_rename,
    view_set_columns, view_set_group, view_set_layout, view_set_sort, view_set_widths,
    view_set_wrap, views_of,
};
pub use model::{DEFAULT_COLUMNS, GroupBy, Layout, Sort, SortBy, View};
pub use query::{Narrowing, ViewData, view_rows};
