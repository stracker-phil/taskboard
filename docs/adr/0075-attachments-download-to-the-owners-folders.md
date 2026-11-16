# 0075. Attachments download to the owner's folders on request

## Status

Accepted (2026-11-16). Owner's request; amends [ADR 0035](0035-attachments-preview-in-quick-look.md), which
rejected downloading to the Downloads folder.

## Context

Quick Look shows images and PDFs well, but a zip or a Word document gets a generic summary, and the copy it
shows lives in the app's cache folder, which is cleared at launch and quit. The owner couldn't get at the file
itself without opening Jira in the browser. A file downloaded by hand from Jira's web page also lands in
Downloads; the rule against issue content on disk is about storing data in the background, not about a file the
owner asks for.

## Decision

- **Download** (the tile's download button and its context menu): the attachment is copied from the cache copy
  (`issues::preview`, downloaded first when needed) into the Downloads folder under its safe name, or the first
  free "name 2.ext", "name 3.ext" as Finder numbers copies (`issues::download::save_new`, claiming the name and
  writing in one step), and Finder shows it selected (`tauri_plugin_opener::reveal_item_in_dir`).
- **Save As…** (context menu): the system save panel, a sheet on the main window starting in Downloads with the
  file's name, then the same copy to the chosen place (`save_to`); cancelling saves nothing.
- Taskboard neither records nor deletes these files; they belong to the owner like any download.
- A click on the tile stays Quick Look.
- **New dependency:** `tauri-plugin-dialog` for the save panel, called from Rust only (no page permission).
- Rejected: showing the cache copy in Finder (a hidden folder whose file disappears at quit, so edits are lost);
  only Save As… (a panel for the common case).

## Consequences

- Attachment bytes reach the owner's folders, but only on the owner's click, one file at a time.
- The 50 MB cap of ADR 0035 applies to downloads too; a larger file still opens in the browser.
