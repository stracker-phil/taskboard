# 0042. Inline images load through an app scheme

## Status

Accepted (2026-10-30). Performance cleanup. Supersedes in part
[0039](0039-inline-images-come-from-issue-attachments-through-rust.md) (the `data:` URL, the image memory, the
CSP); keeps the rest of it, [0025](0025-all-jira-traffic-goes-through-one-allowlisted-gateway.md) and
[0035](0035-attachments-preview-in-quick-look.md).

## Context

0039 answered each inline image as a base64 `data:` URL over IPC. A 10 MB screenshot became a 13 MB string that
Rust built, cloned on every memory hit and serialized into JSON, the page kept in its query cache, and Rust's
memory of the last answers held up to 50 MB more. The file read blocked a runtime worker. The bytes are already a
file in the app's cache folder.

## Decision

- **An asynchronous URI scheme serves the file.** The app registers `inline-image:`
  (`register_asynchronous_uri_scheme_protocol`, `issues::commands::inline_image`); the page sets
  `<img src="inline-image://localhost/<issue id>/<attachment id>">` (`api.inlineImageUrl`). Both ids must be
  digits. The handler keeps 0039's checks (`issues::attachment_image`): an attachment of that issue, a raster
  type, at most 10 MB, bytes that match the declared type; it reads the cached file asynchronously and downloads
  it through the gateway when it isn't there yet. The answer carries the image's type and `no-store`, so the
  webview keeps no copy of its own; anything else is a 404 (or 500 on a failure), and the page shows the chip.
- **No memory in Rust or the page.** The webview decodes and caches images as it does any other; the IPC carries
  no image bytes.
- **The same name in both builds.** The scheme exists only inside the app's own webview, so the debug build
  doesn't need its own.
- **The CSP adds the scheme:** `img-src 'self' data: inline-image:`. Avatars stay `data:` URLs (at most 512 KB).
- Two asks for the same attachment download it once (a lock per file); each download writes a temporary file
  of its own before it is renamed into place.
- Rejected: keeping `data:` URLs with a smaller cap (still copies per answer); `convertFileSrc` with the asset
  protocol (it would open the cache folder to any path the page names, and skip the checks); streaming the file
  in ranges (images are small enough to answer whole).

## Consequences

- An image costs its bytes once in Rust per load and once in the webview; scrolling back reads the local file.
- The page can no longer tell "no image" from "not loaded yet" before the load ends: it shows the box until the
  image loads or fails.
