//! Inline nodes: text with its marks, breaks, mentions, emoji, dates, status lozenges, cards and media.

use chrono::DateTime;
use serde_json::Value;

use super::{escape, one_of, web_address};

const STATUS_COLORS: &[&str] = &["neutral", "purple", "blue", "red", "yellow", "green"];

/// Writes `node` when `kind` is an inline node; `false` for any other kind.
pub(super) fn render(kind: &str, node: &Value, out: &mut String) -> bool {
    let attrs = &node["attrs"];
    match kind {
        "text" => text(
            node["text"].as_str().unwrap_or_default(),
            &node["marks"],
            out,
        ),
        "hardBreak" => out.push_str("<br>"),
        "mention" => {
            let name = attrs["text"].as_str().unwrap_or_default();
            out.push_str("<span class=\"adf-mention\">@");
            escape(name.strip_prefix('@').unwrap_or(name), out);
            out.push_str("</span>");
        }
        "emoji" => escape(
            attrs["text"]
                .as_str()
                .or(attrs["shortName"].as_str())
                .unwrap_or_default(),
            out,
        ),
        "date" => {
            let day = attrs["timestamp"]
                .as_str()
                .and_then(|ms| ms.parse().ok())
                .or(attrs["timestamp"].as_i64())
                .and_then(DateTime::from_timestamp_millis);
            if let Some(day) = day {
                out.push_str(&day.format("%b %-d, %Y").to_string());
            }
        }
        "status" => {
            let color = one_of(attrs["color"].as_str(), STATUS_COLORS, "neutral");
            out.push_str(&format!("<span class=\"adf-status adf-status--{color}\">"));
            escape(attrs["text"].as_str().unwrap_or_default(), out);
            out.push_str("</span>");
        }
        "inlineCard" => card(attrs, out),
        "media" | "mediaInline" => {
            out.push_str("<span class=\"jira-chip\">Attachment");
            let name = ["alt", "name", "__fileName"]
                .iter()
                .find_map(|key| attrs[*key].as_str().filter(|n| !n.trim().is_empty()));
            if let Some(name) = name {
                out.push_str(" · ");
                escape(name, out);
            }
            out.push_str("</span> ");
        }
        _ => return false,
    }
    true
}

/// Text inside its marks: a link outermost, code innermost. Colors are left out.
pub(super) fn text(text: &str, marks: &Value, out: &mut String) {
    let marks = marks.as_array().map(Vec::as_slice).unwrap_or_default();
    let has = |kind: &str| marks.iter().find(|m| m["type"] == kind);
    let mut close = Vec::new();
    if let Some(link) = has("link") {
        match web_address(link["attrs"]["href"].as_str()) {
            Some(href) => {
                out.push_str("<a href=\"");
                escape(href, out);
                out.push_str("\">");
            }
            None => out.push_str("<a>"),
        }
        close.push("a");
    }
    for (mark, tag) in [
        ("strong", "strong"),
        ("em", "em"),
        ("underline", "u"),
        ("strike", "s"),
    ] {
        if has(mark).is_some() {
            out.push_str(&format!("<{tag}>"));
            close.push(tag);
        }
    }
    if let Some(subsup) = has("subsup") {
        let tag = if subsup["attrs"]["type"] == "sub" {
            "sub"
        } else {
            "sup"
        };
        out.push_str(&format!("<{tag}>"));
        close.push(tag);
    }
    if has("code").is_some() {
        out.push_str("<code>");
        close.push("code");
    }
    escape(text, out);
    for tag in close.iter().rev() {
        out.push_str(&format!("</{tag}>"));
    }
}

/// A smart link: its address as a link when it is one the page may open, else as text.
pub(super) fn card(attrs: &Value, out: &mut String) {
    let url = attrs["url"].as_str().unwrap_or_default();
    match web_address(Some(url)) {
        Some(href) => {
            out.push_str("<a href=\"");
            escape(href, out);
            out.push_str("\">");
            escape(href, out);
            out.push_str("</a>");
        }
        None => escape(url, out),
    }
}
