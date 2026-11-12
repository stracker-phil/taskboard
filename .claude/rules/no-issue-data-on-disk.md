# No issue content on disk

Jira data lives in memory only and is gone when Taskboard quits. Nothing a person wrote in Jira is ever
written to disk: not to SQLite, JSON files, logs, exports or caches.

- **May be stored:** ids and keys (issue, project, account, status, field, comment ids), timestamps, counts,
  lengths, read and unread marks, and the owner's own setup (Spaces, filter lists, views, bookmarks, settings).
- **Never stored:** titles, descriptions, comments, labels, custom field values, web link titles or URLs, attachment
  names, people's names, or any other text from an issue.
- **Logs** name issues by key or id only, and errors by kind; never a title, comment or name.
- **Exceptions (owner, 2026-11-12):**
  - The record of changes sent to Jira (`writes::sent`) keeps a change's kind; for a value picked from a list
    (status, priority, assignee) the value itself; for typed text (a title, a comment, a web link) only its length
    and its first 10 characters.
  - Quick Look needs a file: an attachment being previewed is downloaded to the app's Caches folder and deleted
    at launch and quit.

Before adding a column, a file or a log line, check it against this list. A new kind of stored data needs the
owner's OK and a line here.
