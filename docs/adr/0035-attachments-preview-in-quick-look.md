# 0035. Attachments preview in Quick Look

## Status

Accepted (2026-10-29). Owner's request; overrides the spec's "no attachments preview".

## Context

The Issue view lists an issue's attachments. Opening one in the browser works, but it leaves the app, needs
the owner's Jira session in the browser and turns a glance at a screenshot into a tab to close. The owner wants
to look at a file in place. The webview loads nothing remote ([ADR 0027](0027-jira-html-is-sanitised-and-shows-no-remote-content.md)),
and drawing PDFs, images and other formats in the page would mean a viewer per format.

## Decision

- **Quick Look shows the file.** `quicklook.rs` opens the system's `QLPreviewPanel` in-process, on the main
  thread, with a data source that returns the one file URL. Quick Look draws every format it knows and a
  generic icon for the rest (zip).
- **Rust downloads it from the site, through the gateway.** `GET attachment/content/{id}?redirect=false` is one
  more allowlisted read ([ADR 0025](0025-all-jira-traffic-goes-through-one-allowlisted-gateway.md)): the
  site serves the bytes itself instead of a redirect to Atlassian's media host, so the no-redirect client and
  the host list stay as they are. The answer may be any format and is capped at 50 MB (`MAX_ATTACHMENT_BYTES`,
  by `Content-Length` and while reading).
- **A cache folder that lives only while the app runs.** The file goes to
  `<app cache>/attachments/<attachment id>/<safe name>` (`issues::preview`): the id must be digits, the name loses
  separators, control characters, `..` and leading dots and keeps its extension. A file already there with the
  size Jira lists is reused. The folder is removed when the app starts and when it quits.
- **New macOS-only dependencies:** `objc2`, `objc2-foundation`, `objc2-app-kit` and `objc2-quick-look-ui`.
- Opening in the browser stays, as the tile's menu entry "Open in Browser".
- Rejected: a viewer in the page (one renderer per format, remote content in the webview); `qlmanage -p` (a
  separate process and window, a debugging tool); downloading to the Downloads folder (leaves files behind).

## Consequences

- Attachment bytes reach the Mac, but only in the app's cache folder and only while the app runs.
- The Quick Look bridge is unsafe FFI and can't be unit-tested; it stays a thin file with no logic.
- A file over 50 MB doesn't preview; the browser still opens it.
