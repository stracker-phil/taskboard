//! Plain demo text as Atlassian Document Format and as the HTML Jira renders from it.
//!
//! Each line is a paragraph; consecutive lines starting with `- ` form a bullet list.

use serde_json::{Value, json};

enum Block<'a> {
    Paragraph(&'a str),
    Bullets(Vec<&'a str>),
}

fn blocks(text: &str) -> Vec<Block<'_>> {
    let mut blocks = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match (line.strip_prefix("- "), blocks.last_mut()) {
            (Some(item), Some(Block::Bullets(items))) => items.push(item),
            (Some(item), _) => blocks.push(Block::Bullets(vec![item])),
            (None, _) => blocks.push(Block::Paragraph(line)),
        }
    }
    blocks
}

fn paragraph(text: &str) -> Value {
    json!({ "type": "paragraph", "content": [{ "type": "text", "text": text }] })
}

pub fn doc(text: &str) -> Value {
    let content: Vec<Value> = blocks(text)
        .into_iter()
        .map(|block| match block {
            Block::Paragraph(text) => paragraph(text),
            Block::Bullets(items) => json!({
                "type": "bulletList",
                "content": items
                    .into_iter()
                    .map(|item| json!({ "type": "listItem", "content": [paragraph(item)] }))
                    .collect::<Vec<_>>(),
            }),
        })
        .collect();
    json!({ "type": "doc", "version": 1, "content": content })
}

pub fn html(text: &str) -> String {
    blocks(text)
        .into_iter()
        .map(|block| match block {
            Block::Paragraph(text) => format!("<p>{}</p>", escape(text)),
            Block::Bullets(items) => {
                let items: String = items
                    .into_iter()
                    .map(|item| format!("<li>{}</li>", escape(item)))
                    .collect();
                format!("<ul>{items}</ul>")
            }
        })
        .collect()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
