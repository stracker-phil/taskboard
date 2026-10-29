//! Atlassian Document Format (ADF) → HTML for the page. Pure: no database, no network.
//!
//! Jira's own rendering (`renderedFields`) goes through wiki markup and loses formatting: inline code comes back
//! as `<tt>`, and `->text->` turns into strike-through. Rendering the ADF here keeps what the author wrote. Every
//! text is escaped and only tags and classes the page's sanitizer allows are written; the page still sanitizes
//! the result (ADR 0027). Nothing remote is referenced: an image attached to the issue becomes a placeholder
//! the page fills (ADR 0039), other media become text chips (ADR 0020).

mod blocks;
mod inline;
mod media;

pub use media::{File, Media, is_inline_image};

use serde_json::Value;

/// The document's HTML; `None` when `adf` isn't an ADF document (the caller falls back to Jira's HTML). `media`
/// names the issue's attachments, so images among them show inline.
pub fn to_html(adf: &Value, media: &Media) -> Option<String> {
    if adf["type"] != "doc" || !adf["content"].is_array() {
        return None;
    }
    let mut out = String::new();
    children(adf, media, &mut out);
    Some(out)
}

/// The HTML for a stored ADF document, else `fallback` (Jira's rendered HTML). A document that renders to
/// nothing is no content.
pub fn html_or(adf: Option<&str>, fallback: Option<String>, media: &Media) -> Option<String> {
    match adf
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
        .and_then(|doc| to_html(&doc, media))
    {
        Some(html) => Some(html).filter(|html| !html.is_empty()),
        None => fallback,
    }
}

/// One node of any kind; an unknown one renders its text or its children.
fn node(node: &Value, media: &Media, out: &mut String) {
    let Some(kind) = node["type"].as_str() else {
        return;
    };
    if !blocks::render(kind, node, media, out) && !inline::render(kind, node, media, out) {
        match node["text"].as_str() {
            Some(text) => inline::text(text, &node["marks"], out),
            None => children(node, media, out),
        }
    }
}

fn children(parent: &Value, media: &Media, out: &mut String) {
    for child in parent["content"].as_array().into_iter().flatten() {
        node(child, media, out);
    }
}

fn wrap(tag: &str, parent: &Value, media: &Media, out: &mut String) {
    out.push_str(&format!("<{tag}>"));
    children(parent, media, out);
    out.push_str(&format!("</{tag}>"));
}

fn escape(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

/// An address the page may link to: `https`, or a path on the Jira site (the page resolves it).
fn web_address(href: Option<&str>) -> Option<&str> {
    let href = href?.trim();
    let https = href
        .get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
    let site_path = href.starts_with('/') && !href.starts_with("//");
    (https || site_path).then_some(href)
}

/// `value` when it is one of `allowed`, else `fallback`: only fixed class names reach the page.
fn one_of<'a>(value: Option<&str>, allowed: &[&'a str], fallback: &'a str) -> &'a str {
    value
        .and_then(|v| allowed.iter().find(|a| a.eq_ignore_ascii_case(v)))
        .copied()
        .unwrap_or(fallback)
}

#[cfg(test)]
mod media_tests;
#[cfg(test)]
mod tests;
