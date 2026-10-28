//! The sidebar tree (ADR 0024): Everything, Home, the Spaces and their filter lists, which decide which
//! issues a level holds.

pub mod commands;
mod conditions;
mod context;
mod defaults;
mod edit;
mod nodes;
mod tree;

pub use conditions::{
    Choice, FilterField, FilterOptions, ListConditions, LockedCondition, ValueKind, filter_options,
    list_conditions,
};
pub use context::Viewer;
pub use defaults::{LIST_VIEW, ensure_space};
pub use edit::{
    MatchCount, SpaceUpdate, ensure_tracked_spaces, list_create, list_delete, list_duplicate,
    list_match_count, list_move, list_rename, list_set_filter, space_move, space_update,
};
pub use nodes::{Kind, all_nodes, node, scope_of};
pub use tree::{ListNode, SidebarTree, SpaceNode, sidebar_tree};
