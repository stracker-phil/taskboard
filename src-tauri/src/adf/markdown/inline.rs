//! Inline nodes as Markdown: text with its marks, breaks, mentions, emoji, dates, status lozenges, cards and
//! media as a text placeholder.

use serde_json::Value;

use crate::adf::inline::{date, emoji_text, mention_name};

const INLINE: &[&str] = &[
    "text",
    "hardBreak",
    "mention",
    "emoji",
    "date",
    "status",
    "inlineCard",
    "media",
    "mediaInline",
];

pub(super) fn is_inline(kind: &str) -> bool {
    INLINE.contains(&kind)
}

/// The node's inline children, trimmed.
pub(super) fn children(parent: &Value) -> String {
    let mut out = String::new();
    for child in parent["content"].as_array().into_iter().flatten() {
        node(child, &mut out);
    }
    out.trim().to_string()
}

/// One inline node; an unknown one gives its text or its children.
pub(super) fn node(inline: &Value, out: &mut String) {
    let attrs = &inline["attrs"];
    match inline["type"].as_str().unwrap_or_default() {
        "hardBreak" => out.push_str("\\\n"),
        "mention" => {
            out.push('@');
            out.push_str(mention_name(attrs));
        }
        "emoji" => out.push_str(emoji_text(attrs)),
        "date" => {
            if let Some(day) = date(attrs) {
                out.push_str(&day.format("%Y-%m-%d").to_string());
            }
        }
        "status" => out.push_str(attrs["text"].as_str().unwrap_or_default()),
        "inlineCard" => out.push_str(&card(attrs)),
        "media" | "mediaInline" => media(attrs, out),
        _ => match inline["text"].as_str() {
            Some(text) => marked(text, &inline["marks"], out),
            None => {
                for child in inline["content"].as_array().into_iter().flatten() {
                    node(child, out);
                }
            }
        },
    }
}

/// Text inside its marks: a link outermost, code innermost. Underline, sub/sup and colors have no Markdown and
/// are left out; spaces at the ends stay outside the marks, where Markdown needs them.
fn marked(text: &str, marks: &Value, out: &mut String) {
    let core = text.trim();
    if core.is_empty() {
        out.push_str(text);
        return;
    }
    let marks = marks.as_array().map(Vec::as_slice).unwrap_or_default();
    let mark = |kind: &str| marks.iter().find(|m| m["type"] == kind);
    let mut inner = match mark("code") {
        Some(_) => code_span(core),
        None => core.to_string(),
    };
    for (kind, wrap) in [("strike", "~~"), ("em", "*"), ("strong", "**")] {
        if mark(kind).is_some() {
            inner = format!("{wrap}{inner}{wrap}");
        }
    }
    if let Some(href) = mark("link").and_then(|l| trimmed(&l["attrs"]["href"])) {
        inner = format!("[{inner}]({})", destination(href));
    }
    let lead = &text[..text.len() - text.trim_start().len()];
    let tail = &text[text.trim_end().len()..];
    out.push_str(lead);
    out.push_str(&inner);
    out.push_str(tail);
}

fn code_span(code: &str) -> String {
    let ticks = "`".repeat(longest_run(code, '`') + 1);
    let pad = if code.starts_with('`') || code.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{ticks}{pad}{code}{pad}{ticks}")
}

fn destination(href: &str) -> String {
    if href.contains([' ', '(', ')']) {
        format!("<{href}>")
    } else {
        href.to_string()
    }
}

pub(super) fn card(attrs: &Value) -> String {
    match trimmed(&attrs["url"]) {
        Some(url) => format!("[{url}]({})", destination(url)),
        None => String::new(),
    }
}

/// An attachment as `[attachment: name]`; an image on another server as an image link.
fn media(attrs: &Value, out: &mut String) {
    let name = ["alt", "name", "__fileName"]
        .iter()
        .find_map(|key| trimmed(&attrs[*key]));
    match (attrs["type"] == "external", trimmed(&attrs["url"])) {
        (true, Some(url)) => out.push_str(&format!(
            "![{}]({}) ",
            name.unwrap_or_default(),
            destination(url)
        )),
        _ => match name {
            Some(name) => out.push_str(&format!("[attachment: {name}] ")),
            None => out.push_str("[attachment] "),
        },
    }
}

/// A string value trimmed; `None` when it isn't one or is blank.
fn trimmed(value: &Value) -> Option<&str> {
    value.as_str().map(str::trim).filter(|s| !s.is_empty())
}

pub(super) fn longest_run(text: &str, c: char) -> usize {
    let (mut longest, mut run) = (0, 0);
    for ch in text.chars() {
        run = if ch == c { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
}
