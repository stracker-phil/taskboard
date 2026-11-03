//! ADF → Markdown, for an issue copied to the clipboard. Blocks are separated by one blank line, lists are tight
//! and nest by indenting, headings sit two levels lower so they fall below the section's `##`. Text is not
//! escaped: the result is for reading and pasting, not for a renderer that must reproduce the document exactly.

mod inline;
mod table;

use serde_json::Value;

#[cfg(test)]
mod tests;

/// The document as Markdown; `None` when `adf` isn't an ADF document. A document without content gives "".
pub fn to_markdown(adf: &Value) -> Option<String> {
    if adf["type"] != "doc" || !adf["content"].is_array() {
        return None;
    }
    let text = blocks_text(adf);
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    Some(lines.join("\n").trim().to_string())
}

/// The node's children as blocks. Inline nodes in a row form a paragraph; an unknown node without text gives
/// its children's blocks.
fn blocks(parent: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let mut paragraph = String::new();
    for child in parent["content"].as_array().into_iter().flatten() {
        let Some(kind) = child["type"].as_str() else {
            continue;
        };
        let Some(text) = child_block(kind, child) else {
            inline::node(child, &mut paragraph);
            continue;
        };
        flush(&mut paragraph, &mut out);
        if !text.trim().is_empty() {
            out.push(text);
        }
    }
    flush(&mut paragraph, &mut out);
    out
}

fn blocks_text(parent: &Value) -> String {
    blocks(parent).join("\n\n")
}

/// A child as a block: rendered, or its children's blocks when it is unknown; `None` when it is inline.
fn child_block(kind: &str, child: &Value) -> Option<String> {
    match block(kind, child) {
        Some(text) => Some(text),
        None if inline::is_inline(kind) || child["text"].is_string() => None,
        None => Some(blocks_text(child)),
    }
}

fn flush(paragraph: &mut String, out: &mut Vec<String>) {
    let text = paragraph.trim();
    if !text.is_empty() {
        out.push(text.to_string());
    }
    paragraph.clear();
}

/// A block node as Markdown; `None` when `kind` isn't a block this renderer knows.
fn block(kind: &str, node: &Value) -> Option<String> {
    let attrs = &node["attrs"];
    Some(match kind {
        "paragraph" => inline::children(node),
        "heading" => {
            let level = (attrs["level"].as_u64().unwrap_or(1) + 2).clamp(3, 6) as usize;
            let text = inline::children(node);
            if text.is_empty() {
                text
            } else {
                format!("{} {text}", "#".repeat(level))
            }
        }
        "bulletList" | "decisionList" => list(node, |_| "- ".to_string()),
        "orderedList" => {
            let start = attrs["order"].as_u64().unwrap_or(1);
            list(node, |i| format!("{}. ", start + i as u64))
        }
        "taskList" => list(node, |_| String::new()),
        "blockquote" | "panel" => quote(&blocks_text(node)),
        "codeBlock" => code_block(node),
        "rule" => "---".to_string(),
        "table" => table::render(node),
        "expand" | "nestedExpand" => {
            let mut parts = Vec::new();
            let title = attrs["title"].as_str().unwrap_or_default().trim();
            if !title.is_empty() {
                parts.push(format!("**{title}**"));
            }
            parts.extend(blocks(node));
            parts.join("\n\n")
        }
        "mediaSingle" | "mediaGroup" => {
            let mut out = String::new();
            for child in node["content"].as_array().into_iter().flatten() {
                inline::node(child, &mut out);
            }
            out.trim().to_string()
        }
        "blockCard" | "embedCard" => inline::card(attrs),
        _ => return None,
    })
}

/// Continuation lines indent at most this much: deeper indents read as code in many Markdown renderers.
const MAX_CONTINUATION_INDENT: usize = 4;

/// A list's items, one per line; `marker` gives the i-th item's marker. Each item's further lines are indented
/// under its text, and a list nested directly in the list (as Jira nests task lists) is indented as a whole.
fn list(node: &Value, marker: impl Fn(usize) -> String) -> String {
    let mut lines = Vec::new();
    let mut index = 0;
    for child in node["content"].as_array().into_iter().flatten() {
        let (marker, body) = match child["type"].as_str() {
            Some("listItem") => (marker(index), item(child)),
            Some("taskItem") => {
                let done = child["attrs"]["state"] == "DONE";
                let checkbox = if done { "- [x] " } else { "- [ ] " };
                (checkbox.to_string(), inline::children(child))
            }
            Some("decisionItem") => (marker(index), inline::children(child)),
            _ => {
                let nested = block(child["type"].as_str().unwrap_or_default(), child)
                    .unwrap_or_else(|| blocks_text(child));
                if !nested.trim().is_empty() {
                    lines.push(format!("  {}", indent(nested.trim(), 2)));
                }
                continue;
            }
        };
        index += 1;
        let width = marker.chars().count().min(MAX_CONTINUATION_INDENT);
        let body = indent(body.trim(), width);
        lines.push(format!("{marker}{body}"));
    }
    lines.join("\n")
}

/// A list item's blocks: a nested list follows its text on the next line, other blocks after a blank line.
fn item(node: &Value) -> String {
    let mut out = String::new();
    let mut paragraph = String::new();
    for child in node["content"].as_array().into_iter().flatten() {
        let kind = child["type"].as_str().unwrap_or_default();
        let Some(text) = child_block(kind, child) else {
            inline::node(child, &mut paragraph);
            continue;
        };
        push_item_part(&mut out, std::mem::take(&mut paragraph).trim(), false);
        push_item_part(&mut out, text.trim(), is_list_kind(kind));
    }
    push_item_part(&mut out, paragraph.trim(), false);
    out
}

fn is_list_kind(kind: &str) -> bool {
    matches!(
        kind,
        "bulletList" | "orderedList" | "taskList" | "decisionList"
    )
}

fn push_item_part(out: &mut String, text: &str, is_list: bool) {
    if text.is_empty() {
        return;
    }
    if !out.is_empty() {
        out.push_str(if is_list { "\n" } else { "\n\n" });
    }
    out.push_str(text);
}

/// Every line after the first indented by `width` spaces; blank lines stay empty.
fn indent(text: &str, width: usize) -> String {
    let pad = " ".repeat(width);
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 || line.is_empty() {
                line.to_string()
            } else {
                format!("{pad}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn quote(text: &str) -> String {
    text.lines()
        .map(|line| match line {
            "" => ">".to_string(),
            _ => format!("> {line}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A fenced block with its language, the fence longer than any run of backticks inside.
fn code_block(node: &Value) -> String {
    let mut code = String::new();
    for child in node["content"].as_array().into_iter().flatten() {
        match child["type"].as_str() {
            Some("hardBreak") => code.push('\n'),
            _ => code.push_str(child["text"].as_str().unwrap_or_default()),
        }
    }
    let fence = "`".repeat((inline::longest_run(&code, '`') + 1).max(3));
    let language = node["attrs"]["language"]
        .as_str()
        .unwrap_or_default()
        .trim();
    format!(
        "{fence}{language}\n{}\n{fence}",
        code.trim_end_matches('\n')
    )
}
