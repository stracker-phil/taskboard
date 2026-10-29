//! The view tabs of a sidebar level (ADR 0024): layout, grouping, sort, columns and their own conditions
//! (ADR 0033).

mod board;
pub mod columns;
pub mod commands;
mod copy;
mod display;
mod edit;
mod fetch;
mod filter;
pub mod groups;
mod lookup;
pub mod model;
pub mod query;
pub mod rows;
mod sorting;
mod subtasks;

pub use board::{BoardColumn, BoardData, view_board};
pub use columns::{Column, columns_catalogue};
pub use copy::{copy_views, view_duplicate};
pub use display::{
    view_set_inline_labels, view_set_look, view_set_open_with, view_set_subtasks, view_set_wrap,
};
pub use edit::{
    clean_name, columns_for_new_view, insert_view, list_views, read_view, space_default_columns,
    view_create, view_delete, view_move, view_rename, view_set_column_collapsed, view_set_columns,
    view_set_group, view_set_layout, view_set_sort, view_set_widths, views_of,
};
pub use filter::{view_conditions, view_set_filter};
pub use model::{DEFAULT_COLUMNS, GroupBy, IssueOpening, Layout, Sort, SortBy, SubtaskMode, View};
pub use query::{Narrowing, ViewData, unseen_ids, view_rows};
pub use sorting::fold;
