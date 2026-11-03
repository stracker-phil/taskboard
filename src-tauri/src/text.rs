//! Plain text from markup.

/// Jira's rendered HTML as plain text for search: every tag becomes a space, common entities are decoded.
pub fn html_to_text(html: &str) -> String {
    html_to_paragraphs(html)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Jira's rendered HTML as plain text that keeps its paragraphs and line breaks.
pub fn html_to_paragraphs(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut tag = None::<String>;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, '<') => tag = Some(String::new()),
            (Some(name), '>') => {
                let name = name.trim().to_ascii_lowercase();
                let name = name.split_whitespace().next().unwrap_or_default();
                let name = name.trim_end_matches('/');
                out.push_str(if name == "/p" || is_heading_end(name) {
                    "\n\n"
                } else if ["br", "/li", "/div", "/tr"].contains(&name) {
                    "\n"
                } else {
                    " "
                });
                tag = None;
            }
            (Some(name), _) => name.push(c),
            (None, _) => out.push(c),
        }
    }
    let mut text = String::new();
    for line in decode_entities(&out).lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() && (text.is_empty() || text.ends_with("\n\n")) {
            continue;
        }
        text.push_str(&line);
        text.push('\n');
    }
    text.trim().to_string()
}

fn is_heading_end(name: &str) -> bool {
    ["/h1", "/h2", "/h3", "/h4", "/h5", "/h6"].contains(&name)
}

fn decode_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_becomes_plain_text() {
        for (html, text) in [
            ("", ""),
            ("plain", "plain"),
            ("<p>Hello <b>world</b></p>", "Hello world"),
            ("<ul><li>One</li><li>Two</li></ul>", "One Two"),
            ("<p>a</p>\n\n<p>  b \t c </p>", "a b c"),
            ("Tom &amp; Jerry", "Tom & Jerry"),
            (
                "&lt;b&gt; &quot;x&quot; it&#39;s&nbsp;fine",
                "<b> \"x\" it's fine",
            ),
            ("&amp;lt;", "&lt;"),
        ] {
            assert_eq!(html_to_text(html), text, "{html:?}");
        }
    }

    #[test]
    fn html_keeps_its_paragraphs_and_line_breaks() {
        for (html, text) in [
            ("", ""),
            ("plain", "plain"),
            ("<p>One</p><p>Two</p>", "One\n\nTwo"),
            ("<p>One</p>\n\n\n<p>Two</p>", "One\n\nTwo"),
            ("<p>a<br>b<br/>c</p>", "a\nb\nc"),
            ("<ul><li>One</li><li>Two</li></ul>", "One\nTwo"),
            ("<h2>Title</h2><p>Body</p>", "Title\n\nBody"),
            (
                "<p>  Tom &amp; <b>Jerry</b> &lt;3 &nbsp;</p>",
                "Tom & Jerry <3",
            ),
        ] {
            assert_eq!(html_to_paragraphs(html), text, "{html:?}");
        }
    }
}
