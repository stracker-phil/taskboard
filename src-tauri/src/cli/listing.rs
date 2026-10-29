//! The list replies as text: one row per line, `*` marks where the window is, ids and keys first so
//! scripts can cut them.

use super::rows::{IssueRow, LevelRow, PlaceReport, SpaceRow, ViewRow, ViewsReport};
use crate::views::Layout;

fn marker(current: bool) -> &'static str {
    if current { "* " } else { "  " }
}

fn width<T>(rows: &[T], cell: impl Fn(&T) -> usize) -> usize {
    rows.iter().map(cell).max().unwrap_or(0)
}

fn open_text(open: Option<i64>) -> String {
    open.map_or_else(|| "filter can't be used".into(), |n| format!("{n} open"))
}

pub fn spaces(rows: &[SpaceRow]) -> String {
    if rows.is_empty() {
        return "No Spaces are tracked; pick projects in Settings → Projects.".into();
    }
    let id = width(rows, |r| r.id.to_string().len());
    let key = width(rows, |r| r.key.len());
    lines(rows.iter().map(|r| {
        let client = r.client.as_deref().map(|c| format!(" ({c})")).unwrap_or_default();
        format!(
            "{}{:>id$}  {:<key$}  {}{client} · {} open",
            marker(r.current),
            r.id,
            r.key,
            r.name,
            r.open
        )
    }))
}

pub fn levels(rows: &[LevelRow]) -> String {
    if rows.is_empty() {
        return "No filter lists.".into();
    }
    let id = width(rows, |r| r.id.to_string().len());
    lines(rows.iter().map(|r| {
        let name = match &r.space {
            Some(key) => format!("{key}/{}", r.name),
            None => r.name.clone(),
        };
        format!(
            "{}{:>id$}  {name} · {}",
            marker(r.current),
            r.id,
            open_text(r.open)
        )
    }))
}

fn layout_word(layout: Layout) -> &'static str {
    match layout {
        Layout::List => "list",
        Layout::Board => "board",
        Layout::Table => "table",
    }
}

pub fn views(report: &ViewsReport) -> String {
    let rows: &[ViewRow] = &report.views;
    let id = width(rows, |r| r.id.to_string().len());
    let header = std::iter::once(format!("{}:", report.level));
    lines(header.chain(rows.iter().map(|r| {
        let default = if r.is_default { ", default" } else { "" };
        format!(
            "{}{:>id$}  {} ({}{default})",
            marker(r.current),
            r.id,
            r.name,
            layout_word(r.layout)
        )
    })))
}

pub fn issues(rows: &[IssueRow]) -> String {
    if rows.is_empty() {
        return "No cached issue matches.".into();
    }
    let key = width(rows, |r| r.key.len());
    let status = width(rows, |r| r.status.chars().count());
    lines(rows.iter().map(|r| {
        let gone = if r.gone { " (gone)" } else { "" };
        let pad = status - r.status.chars().count();
        format!(
            "{:<key$}  {}{}  {}{gone}",
            r.key,
            r.status,
            " ".repeat(pad),
            r.summary
        )
    }))
}

/// `NW Website relaunch › Board`.
pub fn place(place: &PlaceReport) -> String {
    format!("{} › {}", place.level, place.view)
}

fn lines(lines: impl Iterator<Item = String>) -> String {
    lines.collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests;
