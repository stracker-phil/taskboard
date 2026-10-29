# 0036. Rich text renders from ADF, not renderedFields

## Status

Accepted (2026-10-29). Owner's bug report. Supersedes in part
[0027](0027-jira-html-is-sanitised-and-shows-no-remote-content.md): its rejected option "rendering ADF ourselves
now" is taken, and the sanitizer allows a fixed set of class names and `start`.

## Context

Descriptions, rich text custom fields and comments were shown from Jira's `renderedFields` HTML. Jira Cloud
builds that HTML by converting the document (ADF) to legacy wiki markup and then to HTML, which loses
formatting: inline code comes back as `<tt>`, which the sanitizer dropped, and text such as
`Shop()->cart->get_totals()` comes back with `->cart->` struck through, because wiki markup reads `-text-` as
strike-through. Headings, lists, bold text and links survived, so the loss was easy to miss. The ADF itself
arrives with every issue and comment (`fields.description`, `comment.body` in API v3).

## Decision

- **Rust renders ADF to HTML** (`src-tauri/src/adf/`, pure; [ADR 0001](0001-rust-owns-the-logic.md)). It escapes
  every text and writes only tags the page's sanitizer allows. Block nodes, inline nodes and marks map to plain
  HTML; panels, status lozenges, mentions and expands get fixed class names (`adf-*`). Unknown nodes render their
  text or children; a malformed document falls back to Jira's HTML.
- **Nothing remote** ([ADR 0020](0020-jira-site-and-atlassian-gateway-are-the-hosts.md)): media become text chips
  ("Attachment · name"); links and smart links keep an `https` address or a path on the site, else no address.
- **Comments keep their ADF**: `cache.comment.body_adf` (migration `cache/0010`), filled by the sync and by the
  on-demand comment fetch. Rows from before it show `body_html` until the next sync.
- **The page still receives HTML strings** (`IssueDetail.descriptionHtml`, `CustomField.html`,
  `ActivityItem.bodyHtml`) and still sanitizes them (defense in depth). The sanitizer adds `tt` for old cached
  HTML, `start`, and `class` filtered to the renderer's fixed names after sanitizing.
- Search text stays derived from Jira's HTML (`sync::map::html_to_text`): wording, not formatting, matters there.
- Rejected: patching Jira's HTML (the strike-through can't be told apart from a real one); Atlaskit's renderer
  (large, loads remote resources).

## Consequences

- What the author wrote is what the owner sees, including code with `->` and `--`.
- New ADF node types show as their text until the renderer learns them.
- The same renderer is the base for writing comments later (M4).
