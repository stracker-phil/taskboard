# 0027. Jira HTML is sanitised and shows no remote content

## Status

Accepted (2026-10-28). Decision D10 of the plan; builds on [0020](0020-jira-site-and-atlassian-gateway-are-the-hosts.md).

## Context

Descriptions and comments arrive as Jira's rendered HTML. It is untrusted: anyone who can edit an issue on the
site can write it. The spec requires sanitising it before display, opening links in the default browser and
blocking remote scripts and frames; the webview loads nothing remote at all.

## Decision

- `src/lib/jiraHtml.ts` `sanitizeJiraHtml` runs DOMPurify with an allow-list of text, list, table, code and
  link tags and `href`/`src`/`alt`/`title`/`colspan`/`rowspan` only: no `script`, `style`, `iframe`, `object`,
  `form`, inline styles, event handlers or data attributes.
- Links are resolved against the Jira site; a link that isn't `https` (or carries credentials) keeps its text
  and loses its address. `src/issue/JiraHtml.tsx` intercepts clicks and opens the address through `open_url`.
- Every image becomes a link chip ("Image · name"), so no image request leaves the webview, authenticated or
  not.
- Rejected: Atlaskit's renderer (large, loads remote resources); rendering ADF ourselves now (needed only for
  writes in M4).

## Consequences

- Images in descriptions open in the browser instead of showing inline.
- One function guards every place Jira HTML appears (peek, Issue view, comments); tests pin its behaviour.
