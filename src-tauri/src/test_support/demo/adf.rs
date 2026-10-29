//! Demo text as Atlassian Document Format and as the HTML Jira renders from it.
//!
//! Each line is a paragraph; consecutive lines starting with `- ` form a bullet list, and lines between
//! ```` ```lang ```` and ```` ``` ```` a code block. Inside a line, `` `code` ``, `~~struck~~`, `@{Name}` (a
//! mention) and `{status:TEXT}` (a lozenge) are marked up. A line `![name.png]` shows the issue's attachment of
//! that name, as Jira's editor embeds it. The HTML is as lossy as Jira's: code comes back as `<tt>`.

use serde_json::{Value, json};

use super::attachment_files::{PNG_HEIGHT, PNG_WIDTH};

enum Block<'a> {
    Paragraph(&'a str),
    Bullets(Vec<&'a str>),
    Code(&'a str, Vec<&'a str>),
    Image(&'a str),
}

enum Inline<'a> {
    Text(&'a str),
    Code(&'a str),
    Strike(&'a str),
    Mention(&'a str),
    Status(&'a str),
}

fn blocks(text: &str) -> Vec<Block<'_>> {
    let mut blocks = Vec::new();
    let mut code: Option<(&str, Vec<&str>)> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(fence) = trimmed.strip_prefix("```") {
            match code.take() {
                Some((language, lines)) => blocks.push(Block::Code(language, lines)),
                None => code = Some((fence, Vec::new())),
            }
            continue;
        }
        if let Some((_, lines)) = code.as_mut() {
            lines.push(line);
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        if let Some(name) = trimmed
            .strip_prefix("![")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            blocks.push(Block::Image(name));
            continue;
        }
        match (trimmed.strip_prefix("- "), blocks.last_mut()) {
            (Some(item), Some(Block::Bullets(items))) => items.push(item),
            (Some(item), _) => blocks.push(Block::Bullets(vec![item])),
            (None, _) => blocks.push(Block::Paragraph(trimmed)),
        }
    }
    blocks
}

const MARKUP: [(&str, &str); 4] = [("`", "`"), ("~~", "~~"), ("@{", "}"), ("{status:", "}")];

fn inlines(mut line: &str) -> Vec<Inline<'_>> {
    let mut out = Vec::new();
    while !line.is_empty() {
        let next = MARKUP
            .iter()
            .enumerate()
            .filter_map(|(i, (open, close))| {
                let start = line.find(open)?;
                let body = start + open.len();
                let end = body + line[body..].find(close)?;
                Some((start, i, body, end, end + close.len()))
            })
            .min();
        let Some((start, kind, body, end, after)) = next else {
            out.push(Inline::Text(line));
            break;
        };
        if start > 0 {
            out.push(Inline::Text(&line[..start]));
        }
        let inner = &line[body..end];
        out.push(match kind {
            0 => Inline::Code(inner),
            1 => Inline::Strike(inner),
            2 => Inline::Mention(inner),
            _ => Inline::Status(inner),
        });
        line = &line[after..];
    }
    out
}

fn text_node(text: &str, mark: Option<&str>) -> Value {
    match mark {
        Some(mark) => json!({ "type": "text", "text": text, "marks": [{ "type": mark }] }),
        None => json!({ "type": "text", "text": text }),
    }
}

fn paragraph(line: &str) -> Value {
    let content: Vec<Value> = inlines(line)
        .into_iter()
        .map(|inline| match inline {
            Inline::Text(text) => text_node(text, None),
            Inline::Code(text) => text_node(text, Some("code")),
            Inline::Strike(text) => text_node(text, Some("strike")),
            Inline::Mention(name) => json!({
                "type": "mention",
                "attrs": { "id": format!("demo-{}", name.to_lowercase().replace(' ', "-")), "text": format!("@{name}") },
            }),
            Inline::Status(text) => json!({
                "type": "status",
                "attrs": { "text": text, "color": "blue", "localId": "demo-status" },
            }),
        })
        .collect();
    json!({ "type": "paragraph", "content": content })
}

pub fn doc(text: &str) -> Value {
    let content: Vec<Value> = blocks(text)
        .into_iter()
        .map(|block| match block {
            Block::Paragraph(line) => paragraph(line),
            Block::Bullets(items) => json!({
                "type": "bulletList",
                "content": items
                    .into_iter()
                    .map(|item| json!({ "type": "listItem", "content": [paragraph(item)] }))
                    .collect::<Vec<_>>(),
            }),
            Block::Code(language, lines) => json!({
                "type": "codeBlock",
                "attrs": { "language": language },
                "content": [{ "type": "text", "text": lines.join("\n") }],
            }),
            Block::Image(name) => json!({
                "type": "mediaSingle",
                "attrs": { "layout": "center" },
                "content": [{
                    "type": "media",
                    "attrs": {
                        "type": "file",
                        "id": format!("demo-media-{name}"),
                        "collection": "",
                        "alt": name,
                        "width": PNG_WIDTH,
                        "height": PNG_HEIGHT,
                    },
                }],
            }),
        })
        .collect();
    json!({ "type": "doc", "version": 1, "content": content })
}

pub fn html(text: &str) -> String {
    blocks(text)
        .into_iter()
        .map(|block| match block {
            Block::Paragraph(line) => format!("<p>{}</p>", inline_html(line)),
            Block::Bullets(items) => {
                let items: String = items
                    .into_iter()
                    .map(|item| format!("<li>{}</li>", inline_html(item)))
                    .collect();
                format!("<ul>{items}</ul>")
            }
            Block::Code(_, lines) => format!(
                "<div class=\"code panel\"><pre>{}</pre></div>",
                escape(&lines.join("\n"))
            ),
            Block::Image(name) => format!("<p><img alt=\"{}\"></p>", escape(name)),
        })
        .collect()
}

fn inline_html(line: &str) -> String {
    inlines(line)
        .into_iter()
        .map(|inline| match inline {
            Inline::Text(text) => escape(text),
            Inline::Code(text) => format!("<tt>{}</tt>", escape(text)),
            Inline::Strike(text) => format!("<del>{}</del>", escape(text)),
            Inline::Mention(name) => format!("<a class=\"user-hover\">{}</a>", escape(name)),
            Inline::Status(text) => {
                format!("<span class=\"aui-lozenge\">{}</span>", escape(text))
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
