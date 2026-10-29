# 0039. Inline images come from issue attachments through Rust

## Status

Accepted (2026-10-29). Owner's bug report ("Rich text content does not display inline images"). Supersedes in
part [0036](0036-rich-text-renders-from-adf-not-renderedfields.md) (media always become text chips) and
[0027](0027-jira-html-is-sanitised-and-shows-no-remote-content.md) (every image becomes a chip); keeps
[0020](0020-jira-site-and-atlassian-gateway-are-the-hosts.md), [0025](0025-all-jira-traffic-goes-through-one-allowlisted-gateway.md)
and [0035](0035-attachments-preview-in-quick-look.md).

## Context

A screenshot pasted into a description or a comment showed as a text chip, "Attachment · name.png". In ADF the
image is a `media` node (inside `mediaSingle`, `mediaGroup`, or inline as `mediaInline`) whose `id` is a Media
Services file id, not the Jira attachment id, and Media Services is a host the app never calls. The file
itself is also one of the issue's attachments, which Rust already downloads for Quick Look from the Jira site
(`GET attachment/content/{id}?redirect=false`). The webview loads nothing remote.

## Decision

- **The file name is the link.** A media node's name (`attrs.alt`, else `name`, else `__fileName`) is matched
  against the filenames of the same issue's attachments (`adf::Media`, `adf/media.rs`). Of several with that
  name, the newest created at or before the comment was written wins, else the newest (a description takes the
  newest). Only a PNG, JPEG, GIF or WebP (by MIME type, else extension; never SVG) with a numeric id matches.
- **Rust writes a placeholder, never an address**: `<img data-attachment-id="…" alt="…" width="…" height="…">`,
  the size from the media node, scaled to `mediaSingle`'s pixel width. Unmatched media, other files and
  `type: "external"` media (a chip linking to the address, never loaded) stay chips.
- **The page asks for a `data:` URL.** `attachment_image(issue_id, attachment_id)` (`issues/inline_image.rs`)
  answers only for an attachment of that issue, through Quick Look's download and cache folder (a file already
  there is reused, a new one stays for Quick Look), and only when the bytes' magic number is the declared
  format. It caps inline images at 10 MB (`MAX_INLINE_IMAGE_BYTES`, by the listed size and the bytes); a larger
  or mismatching one returns `None` and the page keeps the chip. The last 20 answers, at most 50 MB, stay in
  memory (`ImageMemory`).
- **The sanitizer keeps only the placeholder**: `img` with a numeric `data-attachment-id`, `alt` and numeric
  `width`/`height`, no `src`; every other `img` still becomes a chip. `JiraHtml` fills each placeholder
  (`InlineImage`, TanStack Query): a neutral box of its size while loading, the image (at most the content
  width, radius 8, hairline border) when it arrives, the chip when there is none. A click shows it in Quick
  Look; the context menu is the attachment tile's.
- **Lazy everywhere.** An image is asked for only once it is on screen (IntersectionObserver), so the Peek,
  the preview and long comment threads download nothing they don't show.
- **The CSP stays `img-src 'self' data:`.**
- Rejected: Media Services (another host, another token flow); loading the attachment address in the webview
  (remote content, the owner's cookie); a custom URL scheme (a second way for bytes to reach the page next to
  the `data:` URLs avatars use); matching by Media Services id (Jira's attachment JSON doesn't carry it).

## Consequences

- Images pasted into descriptions and comments show where they were written, offline once fetched in a run.
- An image renamed after it was attached, or attached to another issue, stays a chip.
- The cache folder now also fills from reading rich text, still cleared when the app starts and quits.
