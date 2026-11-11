# 0063. Comments are written in Markdown

## Status

Accepted (2026-11-11). Owner's request.

## Context

The comment box sent plain text: paragraphs, line breaks and links only. Bold, italic and lists meant switching
to Jira's web page for one comment. Jira stores comments as ADF, which has all of these.

## Decision

The comment box takes Markdown (CommonMark with `~~strike~~`), and Rust turns it into ADF
(`adf::from_markdown`) with the `pulldown-cmark` parser (no HTML writer). A line break inside a paragraph stays a
line break and bare `http(s)://` addresses become links, as before. Raw HTML stays text. Links point to web or
mail addresses only. Blocks ADF doesn't allow where they were typed (a heading in a list, a quote in a quote)
become what it allows there. The box stays a plain text field; no editor formats while typing.

## Consequences

- Text that happens to be Markdown changes: `*word*` turns italic, a line starting with `- ` or `1. ` starts a
  list. A backslash keeps the character (`\*`).
- The pending comment shows formatted at once, from the same ADF.
- One more dependency, `pulldown-cmark`.
