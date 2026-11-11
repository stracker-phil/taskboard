//! Markdown (CommonMark with strike-through) → an ADF document, for comments typed in Taskboard. A line break
//! inside a paragraph stays a line break, bare `http(s)://` addresses become links, `@Full Name` of someone who
//! may be mentioned becomes a mention and raw HTML stays text.
//! Blocks ADF doesn't allow where they were typed (a heading in a list, a quote in a quote) become what it
//! allows there.

mod links;
mod mentions;
#[cfg(test)]
mod tests;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde_json::{Map, Value, json};

use links::Piece;
use mentions::Part;

/// Someone a comment may mention.
#[derive(Clone, Debug)]
pub struct Mentionable {
    pub account_id: String,
    pub name: String,
}

pub fn from_markdown(text: &str, people: &[Mentionable]) -> Value {
    let mut doc = Builder {
        people,
        open: Vec::new(),
        doc: Vec::new(),
        marks: Vec::new(),
        text: String::new(),
        link: None,
    };
    for event in Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH) {
        doc.event(event);
    }
    doc.finish()
}

struct Builder<'p> {
    people: &'p [Mentionable],
    /// Open blocks below the document, innermost last.
    open: Vec<Block>,
    doc: Vec<Value>,
    marks: Vec<Value>,
    /// Text not yet written, all under the current marks; Markdown hands it over in pieces.
    text: String,
    link: Option<OpenLink>,
}

struct Block {
    kind: &'static str,
    attrs: Option<Value>,
    content: Vec<Value>,
    /// Opened for inline content that came without a paragraph (a tight list item).
    implicit: bool,
}

struct OpenLink {
    href: Option<String>,
    wrote_text: bool,
}

impl Builder<'_> {
    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text.push_str(&text),
            Event::Code(code) => {
                self.flush();
                let mut marks = self.open_link_mark().into_iter().collect::<Vec<_>>();
                marks.push(json!({ "type": "code" }));
                self.inline(text_node(&code, &marks));
            }
            Event::Html(html) | Event::InlineHtml(html) => self.html(&html),
            Event::SoftBreak | Event::HardBreak => {
                self.flush();
                self.inline(json!({ "type": "hardBreak" }));
            }
            Event::Rule => self.block(json!({ "type": "rule" })),
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        self.flush();
        match tag {
            Tag::Paragraph => self.open("paragraph", None),
            Tag::Heading { level, .. } => {
                self.open("heading", Some(json!({ "level": level as u8 })))
            }
            Tag::BlockQuote(_) => self.open("blockquote", None),
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().map(str::to_string)
                    }
                    CodeBlockKind::Indented => None,
                };
                self.open("codeBlock", language.map(|l| json!({ "language": l })));
            }
            Tag::List(None) => self.open("bulletList", None),
            Tag::List(Some(start)) => {
                self.open(
                    "orderedList",
                    (start != 1).then(|| json!({ "order": start })),
                );
            }
            Tag::Item => self.open("listItem", None),
            Tag::Emphasis => self.marks.push(json!({ "type": "em" })),
            Tag::Strong => self.marks.push(json!({ "type": "strong" })),
            Tag::Strikethrough => self.marks.push(json!({ "type": "strike" })),
            Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                self.link = Some(OpenLink {
                    href: web_address(&dest_url).map(str::to_string),
                    wrote_text: false,
                });
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        self.flush();
        match tag {
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Item => self.close(),
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.marks.pop();
            }
            TagEnd::Link | TagEnd::Image => {
                if let Some(OpenLink {
                    href: Some(href),
                    wrote_text: false,
                }) = &self.link
                {
                    // `[](https://…)` or an image without a description: the address is the text.
                    let href = href.clone();
                    self.inline(text_node(&href, &[link_mark(&href)]));
                }
                self.link = None;
            }
            _ => {}
        }
    }

    fn open(&mut self, kind: &'static str, attrs: Option<Value>) {
        self.close_implicit();
        self.open.push(Block {
            kind,
            attrs,
            content: Vec::new(),
            implicit: false,
        });
    }

    fn close(&mut self) {
        self.close_implicit();
        self.pop_block();
    }

    fn close_implicit(&mut self) {
        if self.open.last().is_some_and(|b| b.implicit) {
            self.pop_block();
        }
    }

    fn pop_block(&mut self) {
        if let Some(block) = self.open.pop() {
            let node = block.into_node();
            self.parent().push(node);
        }
    }

    fn parent(&mut self) -> &mut Vec<Value> {
        match self.open.last_mut() {
            Some(block) => &mut block.content,
            None => &mut self.doc,
        }
    }

    fn block(&mut self, node: Value) {
        self.flush();
        self.close_implicit();
        self.parent().push(node);
    }

    /// An inline node, in a paragraph opened for it when the open block takes no inline content.
    fn inline(&mut self, node: Value) {
        let takes_inline = self
            .open
            .last()
            .is_some_and(|b| matches!(b.kind, "paragraph" | "heading" | "codeBlock"));
        if !takes_inline {
            self.open.push(Block {
                kind: "paragraph",
                attrs: None,
                content: Vec::new(),
                implicit: true,
            });
        }
        if let Some(link) = &mut self.link {
            link.wrote_text = true;
        }
        self.parent().push(node);
    }

    fn html(&mut self, html: &str) {
        for (index, line) in html.trim_end_matches('\n').split('\n').enumerate() {
            if index > 0 {
                self.flush();
                self.inline(json!({ "type": "hardBreak" }));
            }
            self.text.push_str(line);
        }
    }

    fn flush(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text);
        if self.open.last().is_some_and(|b| b.kind == "codeBlock") {
            self.inline(text_node(&text, &[]));
            return;
        }
        let mut marks = self.marks.clone();
        if let Some(mark) = self.open_link_mark() {
            marks.push(mark);
            self.inline(text_node(&text, &marks));
            return;
        }
        for piece in links::split(&text) {
            match piece {
                Piece::Text(text) => {
                    for part in mentions::split(text, self.people) {
                        let node = match part {
                            Part::Text(text) => text_node(text, &marks),
                            Part::Mention(person) => mention_node(person),
                        };
                        self.inline(node);
                    }
                }
                Piece::Link(address) => {
                    let mut marks = marks.clone();
                    marks.push(link_mark(address));
                    self.inline(text_node(address, &marks));
                }
            }
        }
    }

    fn open_link_mark(&self) -> Option<Value> {
        let href = self.link.as_ref()?.href.as_deref()?;
        Some(link_mark(href))
    }

    fn finish(mut self) -> Value {
        self.flush();
        while !self.open.is_empty() {
            self.close();
        }
        json!({ "type": "doc", "version": 1, "content": self.doc })
    }
}

impl Block {
    fn into_node(self) -> Value {
        let content = match self.kind {
            "listItem" => list_item_content(self.content),
            "blockquote" => nested_blocks(self.content),
            "codeBlock" => code_content(self.content),
            _ => self.content,
        };
        let mut node = Map::new();
        node.insert("type".into(), json!(self.kind));
        if let Some(attrs) = self.attrs {
            node.insert("attrs".into(), attrs);
        }
        node.insert("content".into(), Value::Array(content));
        Value::Object(node)
    }
}

/// ADF wants a list item to open with a paragraph or code block.
fn list_item_content(content: Vec<Value>) -> Vec<Value> {
    let mut out = nested_blocks(content);
    if !out
        .first()
        .is_some_and(|n| n["type"] == "paragraph" || n["type"] == "codeBlock")
    {
        out.insert(0, json!({ "type": "paragraph", "content": [] }));
    }
    out
}

/// What a list item or quote may hold: no headings, quotes or rules.
fn nested_blocks(content: Vec<Value>) -> Vec<Value> {
    content
        .into_iter()
        .flat_map(|mut node| match node["type"].as_str() {
            Some("heading") => vec![as_paragraph(node)],
            Some("blockquote") => match node["content"].take() {
                Value::Array(children) => children,
                _ => vec![],
            },
            Some("rule") => vec![],
            _ => vec![node],
        })
        .collect()
}

/// The code block's text as one node, without its trailing line break.
fn code_content(content: Vec<Value>) -> Vec<Value> {
    let text: String = content.iter().filter_map(|n| n["text"].as_str()).collect();
    let text = text.strip_suffix('\n').unwrap_or(&text);
    if text.is_empty() {
        vec![]
    } else {
        vec![text_node(text, &[])]
    }
}

fn as_paragraph(heading: Value) -> Value {
    json!({ "type": "paragraph", "content": heading["content"] })
}

fn text_node(text: &str, marks: &[Value]) -> Value {
    if marks.is_empty() {
        json!({ "type": "text", "text": text })
    } else {
        json!({ "type": "text", "text": text, "marks": allowed_marks(marks) })
    }
}

/// Code goes with a link only.
fn allowed_marks(marks: &[Value]) -> Vec<Value> {
    if marks.iter().any(|m| m["type"] == "code") {
        marks
            .iter()
            .filter(|m| m["type"] == "code" || m["type"] == "link")
            .cloned()
            .collect()
    } else {
        marks.to_vec()
    }
}

fn mention_node(person: &Mentionable) -> Value {
    json!({
        "type": "mention",
        "attrs": { "id": person.account_id, "text": format!("@{}", person.name), "accessLevel": "" },
    })
}

fn link_mark(href: &str) -> Value {
    json!({ "type": "link", "attrs": { "href": href } })
}

/// Only web and mail addresses become links; anything else keeps its text without one.
fn web_address(href: &str) -> Option<&str> {
    let href = href.trim();
    let lower = href.to_ascii_lowercase();
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
        .then_some(href)
}
