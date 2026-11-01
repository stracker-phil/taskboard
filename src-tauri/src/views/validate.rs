//! Cleaning what a view change asks for before it is stored.

use std::collections::BTreeMap;

use crate::error::{AppError, AppResult};

/// Narrowest and widest a Table column can be dragged.
const MIN_WIDTH: i64 = 48;
const MAX_WIDTH: i64 = 800;

pub fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput("The name can't be empty".into()));
    }
    Ok(name.to_string())
}

pub(super) fn clean_columns(columns: Vec<String>) -> AppResult<Vec<String>> {
    let mut clean: Vec<String> = Vec::with_capacity(columns.len());
    for column in columns {
        let column = column.trim().to_string();
        if column.is_empty() {
            return Err(AppError::InvalidInput("A column id can't be empty".into()));
        }
        if !clean.contains(&column) {
            clean.push(column);
        }
    }
    Ok(clean)
}

/// Clamps each width to the Table column bounds.
pub(super) fn clean_widths(widths: BTreeMap<String, i64>) -> BTreeMap<String, i64> {
    widths
        .into_iter()
        .map(|(column, width)| (column, width.clamp(MIN_WIDTH, MAX_WIDTH)))
        .collect()
}
