//! Plain text from markup.

/// Jira's rendered HTML as plain text for search: every tag becomes a space, common entities are decoded.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
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
}
