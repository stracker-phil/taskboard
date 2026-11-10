//! A table as a GitHub pipe table: the first row is the header, each cell on one line.

use serde_json::Value;

pub(super) fn render(node: &Value) -> String {
    let rows: Vec<Vec<String>> = node["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["type"] == "tableRow")
        .map(|row| {
            row["content"]
                .as_array()
                .into_iter()
                .flatten()
                .map(cell)
                .collect()
        })
        .collect();
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    if width == 0 {
        return String::new();
    }
    let line = |cells: &[String]| {
        let padded = (0..width).map(|i| cells.get(i).map(String::as_str).unwrap_or_default());
        format!("| {} |", padded.collect::<Vec<_>>().join(" | "))
    };
    let mut lines = vec![line(&rows[0]), line(&vec!["---".to_string(); width])];
    lines.extend(rows[1..].iter().map(|row| line(row)));
    lines.join("\n")
}

/// A cell's blocks on one line: line breaks become `<br>`, pipes are escaped.
fn cell(node: &Value) -> String {
    super::blocks(node)
        .join("\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("<br>")
        .replace('|', "\\|")
}
