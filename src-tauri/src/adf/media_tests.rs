use serde_json::{Value, json};

use super::*;

fn file(id: &str, filename: &str, mime: Option<&str>, created_at: Option<&str>) -> File {
    File {
        id: id.into(),
        filename: filename.into(),
        mime_type: mime.map(Into::into),
        created_at: created_at.map(Into::into),
    }
}

fn png(id: &str, filename: &str) -> File {
    file(id, filename, Some("image/png"), None)
}

fn render(files: &[File], written_at: Option<&str>, content: Value) -> String {
    let media = Media { files, written_at };
    let doc = json!({ "type": "doc", "version": 1, "content": content });
    to_html(&doc, &media).expect("a document")
}

fn single(attrs: Value) -> Value {
    json!([{ "type": "mediaSingle", "content": [{ "type": "media", "attrs": attrs }] }])
}

fn named(name: &str) -> Value {
    json!({ "type": "file", "id": "media-services-id", "alt": name })
}

#[test]
fn a_media_naming_an_attached_image_becomes_a_placeholder_without_a_source() {
    let files = [png("10001", "wiring.png")];
    assert_eq!(
        render(&files, None, single(named("wiring.png"))),
        "<p><img data-attachment-id=\"10001\" alt=\"wiring.png\"></p>"
    );
}

#[test]
fn a_placeholder_carries_the_media_size_only_when_both_sides_are_positive() {
    let files = [png("10001", "wiring.png")];
    for (width, height, expected) in [
        (json!(240), json!(120), " width=\"240\" height=\"120\""),
        (json!(240), json!(null), ""),
        (json!(null), json!(120), ""),
        (json!(240), json!(0), ""),
        (json!(0), json!(120), ""),
    ] {
        let attrs = json!({ "alt": "wiring.png", "width": width, "height": height });
        assert_eq!(
            render(&files, None, single(attrs)),
            format!("<p><img data-attachment-id=\"10001\" alt=\"wiring.png\"{expected}></p>"),
            "{width} x {height}"
        );
    }
}

#[test]
fn a_pixel_width_on_the_media_single_scales_the_size() {
    let files = [png("10001", "wiring.png")];
    for ((width, height), shown, expected) in [
        ((400, 200), 200, "width=\"200\" height=\"100\""),
        ((300, 101), 100, "width=\"100\" height=\"34\""),
    ] {
        let content = json!([{ "type": "mediaSingle",
            "attrs": { "widthType": "pixel", "width": shown },
            "content": [{ "type": "media",
                "attrs": { "alt": "wiring.png", "width": width, "height": height } }] }]);
        assert_eq!(
            render(&files, None, content),
            format!("<p><img data-attachment-id=\"10001\" alt=\"wiring.png\" {expected}></p>"),
            "{width}x{height} shown at {shown}"
        );
    }
}

#[test]
fn a_percentage_width_on_the_media_single_keeps_the_media_size() {
    let files = [png("10001", "wiring.png")];
    let content = json!([{ "type": "mediaSingle",
        "attrs": { "widthType": "percentage", "width": 50 },
        "content": [{ "type": "media", "attrs": { "alt": "wiring.png", "width": 400, "height": 200 } }] }]);
    assert_eq!(
        render(&files, None, content),
        "<p><img data-attachment-id=\"10001\" alt=\"wiring.png\" width=\"400\" height=\"200\"></p>"
    );
}

#[test]
fn a_media_group_renders_a_placeholder_for_each_attached_image() {
    let files = [png("1", "a.png"), png("2", "b.png")];
    let content = json!([{ "type": "mediaGroup", "content": [
        { "type": "media", "attrs": { "name": "a.png" } },
        { "type": "media", "attrs": { "name": "b.png" } },
    ] }]);
    assert_eq!(
        render(&files, None, content),
        "<p><img data-attachment-id=\"1\" alt=\"a.png\"><img data-attachment-id=\"2\" alt=\"b.png\"></p>"
    );
}

#[test]
fn an_inline_media_naming_an_attached_image_becomes_a_placeholder() {
    let files = [png("10001", "wiring.png")];
    let content = json!([{ "type": "paragraph", "content": [
        { "type": "mediaInline", "attrs": { "name": "wiring.png" } },
    ] }]);
    assert_eq!(
        render(&files, None, content),
        "<p><img data-attachment-id=\"10001\" alt=\"wiring.png\"></p>"
    );
}

#[test]
fn the_placeholder_name_comes_from_alt_then_name_then_file_name() {
    let files = [png("1", "a.png"), png("2", "n.png"), png("3", "f.png")];
    for (attrs, expected) in [
        (
            json!({ "alt": "a.png", "name": "n.png", "__fileName": "f.png" }),
            "<img data-attachment-id=\"1\" alt=\"a.png\">",
        ),
        (
            json!({ "name": "n.png", "__fileName": "f.png" }),
            "<img data-attachment-id=\"2\" alt=\"n.png\">",
        ),
        (
            json!({ "__fileName": "f.png" }),
            "<img data-attachment-id=\"3\" alt=\"f.png\">",
        ),
    ] {
        assert_eq!(
            render(&files, None, single(attrs.clone())),
            format!("<p>{expected}</p>"),
            "{attrs}"
        );
    }
}

#[test]
fn a_placeholder_name_is_attribute_escaped() {
    let files = [png("7", "a\"<b>.png")];
    assert_eq!(
        render(&files, None, single(json!({ "alt": "a\"<b>.png" }))),
        "<p><img data-attachment-id=\"7\" alt=\"a&quot;&lt;b&gt;.png\"></p>"
    );
}

#[test]
fn attachments_that_are_not_inline_images_stay_chips() {
    let files = [
        file("1", "release-notes.pdf", Some("application/pdf"), None),
        file("2", "diagram.svg", Some("image/svg+xml"), None),
        file("3", "drawing.svg", None, None),
        file("4", "fake.png", Some("image/svg+xml"), None),
        file("5", "photo.bmp", Some("image/bmp"), None),
    ];
    for name in [
        "release-notes.pdf",
        "diagram.svg",
        "drawing.svg",
        "fake.png",
        "photo.bmp",
    ] {
        assert_eq!(
            render(&files, None, single(json!({ "alt": name }))),
            format!("<p><span class=\"jira-chip\">Attachment · {name}</span> </p>"),
            "{name}"
        );
    }
}

#[test]
fn a_name_without_a_matching_attachment_stays_a_chip() {
    let files = [png("10001", "wiring.png")];
    assert_eq!(
        render(&files, None, single(named("other.png"))),
        "<p><span class=\"jira-chip\">Attachment · other.png</span> </p>"
    );
}

#[test]
fn an_attachment_id_that_is_not_all_digits_stays_a_chip() {
    for id in ["", "abc", "12a", "-1", "1\" onerror=\"x"] {
        let files = [png(id, "wiring.png")];
        assert_eq!(
            render(&files, None, single(named("wiring.png"))),
            "<p><span class=\"jira-chip\">Attachment · wiring.png</span> </p>",
            "{id:?}"
        );
    }
}

#[test]
fn the_image_type_comes_from_the_extension_when_the_mime_type_is_not_an_image() {
    for (name, mime) in [
        ("shot.png", None),
        ("shot.PNG", None),
        ("shot.jpg", Some("application/octet-stream")),
        ("shot.jpeg", None),
        ("shot.gif", Some("application/octet-stream")),
        ("shot.webp", None),
    ] {
        let files = [file("5", name, mime, None)];
        assert_eq!(
            render(&files, None, single(json!({ "alt": name }))),
            format!("<p><img data-attachment-id=\"5\" alt=\"{name}\"></p>"),
            "{name}"
        );
    }
}

#[test]
fn png_jpeg_gif_and_webp_mime_types_are_inline_images() {
    for mime in [
        "image/png",
        "image/jpeg",
        "image/gif",
        "image/webp",
        "IMAGE/PNG",
    ] {
        assert!(is_inline_image(&file("1", "x", Some(mime), None)), "{mime}");
    }
}

#[test]
fn an_external_https_image_is_a_link_chip_and_never_an_image() {
    let files = [png("10001", "wiring.png")];
    for (attrs, label) in [
        (
            json!({ "type": "external", "url": "https://example.com/a.png" }),
            "https://example.com/a.png",
        ),
        (
            json!({ "type": "external", "url": "https://example.com/a.png", "alt": "wiring.png" }),
            "wiring.png",
        ),
    ] {
        assert_eq!(
            render(&files, None, single(attrs.clone())),
            format!(
                "<p><a class=\"jira-chip\" href=\"https://example.com/a.png\">Image · {label}</a> </p>"
            ),
            "{attrs}"
        );
    }
}

#[test]
fn an_external_image_without_an_https_address_is_a_chip_without_a_link() {
    for (attrs, expected) in [
        (
            json!({ "type": "external", "url": "http://example.com/a.png", "alt": "a.png" }),
            "<span class=\"jira-chip\">Image · a.png</span>",
        ),
        (
            json!({ "type": "external", "url": "javascript:alert(1)" }),
            "<span class=\"jira-chip\">Image</span>",
        ),
        (
            json!({ "type": "external" }),
            "<span class=\"jira-chip\">Image</span>",
        ),
    ] {
        assert_eq!(
            render(&[], None, single(attrs.clone())),
            format!("<p>{expected} </p>"),
            "{attrs}"
        );
    }
}

#[test]
fn an_external_image_address_is_attribute_escaped() {
    assert_eq!(
        render(
            &[],
            None,
            single(json!({ "type": "external", "url": "https://example.com/a\"b" }))
        ),
        "<p><a class=\"jira-chip\" href=\"https://example.com/a&quot;b\">Image · https://example.com/a&quot;b</a> </p>"
    );
}

fn versions() -> Vec<File> {
    vec![
        file(
            "3",
            "a.png",
            Some("image/png"),
            Some("2026-03-03T10:00:00Z"),
        ),
        file(
            "1",
            "a.png",
            Some("image/png"),
            Some("2026-01-01T10:00:00Z"),
        ),
        file(
            "2",
            "a.png",
            Some("image/png"),
            Some("2026-02-02T10:00:00Z"),
        ),
    ]
}

fn picked_id(written_at: Option<&str>) -> String {
    let html = render(&versions(), written_at, single(named("a.png")));
    html.split("data-attachment-id=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a placeholder")
        .to_owned()
}

#[test]
fn among_same_named_attachments_the_newest_before_the_document_was_written_wins() {
    for (written_at, expected) in [
        ("2026-02-15T00:00:00Z", "2"),
        ("2026-02-02T10:00:00Z", "2"),
        ("2026-01-01T10:00:00Z", "1"),
        ("2026-12-31T00:00:00Z", "3"),
    ] {
        assert_eq!(picked_id(Some(written_at)), expected, "{written_at}");
    }
}

#[test]
fn without_a_write_time_the_newest_same_named_attachment_wins() {
    assert_eq!(picked_id(None), "3");
}

#[test]
fn when_every_same_named_attachment_is_newer_than_the_document_the_newest_wins() {
    assert_eq!(picked_id(Some("2025-12-31T00:00:00Z")), "3");
}
