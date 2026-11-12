# API token scopes

What a **scoped** API token needs (Atlassian account → Security → API tokens → Create API token with scopes,
app Jira). A classic token has all of its user's permissions and needs no scopes. Keep this page in step with
the gateway's allowlist (`jira/gateway/guard.rs` `RULES`): every new endpoint names its scope here.

## Overview

| Scope | Needed for | Without it |
|---|---|---|
| `read:jira-work` | Syncing issues, projects, fields, statuses, releases, comments, history, web links, attachments; checking JQL | Nothing syncs; Test Connection may pass, the sync fails |
| `read:jira-user` | Who you are (Test Connection, the Me chip, "Assigned to you") | Test Connection fails; the app doesn't know which issues are yours |
| `write:jira-work` | Assigning issues, changing their status, priority or title, adding comments and web links | Changes fail with "a scoped token lacks write:jira-work"; reading keeps working |

Settings → Connection lists the same in plain words: what Taskboard reads, and each kind of change with its own
switch (`jira::capability`, named by every request the gateway sends), each with the scope it needs as Jira names
it (`Capability::scope`). *Token scopes* above them sums it up: every scope once, Required or Needed for changes
(Optional while changes are off), with what uses it. `write:jira-work` is only needed with
**Allow changes to Jira** on
([writes.md](writes.md)). Leave it out for a token that can only read.

No scope covers profile pictures: they come from Atlassian's public avatar host without the token
([ADR 0032](adr/0032-avatars-come-from-atlassians-avatar-host-through-rust.md)). The site's cloud id
(`/_edge/tenant_info`) is asked without the token too.

## read:jira-work

Reads the issues and everything around them. Every request of the sync and of an opened issue
([sync.md](sync.md), [jira.md](jira.md#endpoints)):

- **Issues:** `POST search/jql` (full and change syncs; the change sync's also brings each issue's recent
  history with `expand=changelog`, no further scope), `POST issue/bulkfetch` (reconciliation and the fetch after
  a change), `POST search/approximate-count` (sync progress).
- **Around them:** `GET field` (custom fields, the sprint and epic color fields), `GET status`,
  `GET project/search` (Settings → Projects), `GET project/{key}/versions` (releases).
- **An opened issue:** `GET issue/{id}/comment`, `GET issue/{id}/remotelink` (web links),
  `GET issue/{id}/changelog` (history; also after a sync, for the authors of its changes), `GET attachment/content/{id}` (Quick Look, inline images).
- **The status menu and drags:** `GET issue/{id}/transitions`, the moves the workflow offers;
  `GET issue/{id}/editmeta`, the priorities an issue may get.
- **Settings → Sync:** `POST jql/parse` validates the sync scope's JQL.
- **Settings → Connection:** `GET mypermissions` (per tracked project) checks which changes Jira allows the user
  there.

## read:jira-user

`GET myself`: the account id, name and time zone of the token's user. Test Connection and every full sync ask
it. The account id decides what "me" is (Me chip, `me` in filters, Assign to me); the time zone is the one
Jira reads JQL dates in, which the change sync needs. People named on issues (assignee, reporter, authors)
arrive inside the issues under `read:jira-work`.

## write:jira-work

Changes, sent once and only while changes to Jira are allowed ([writes.md](writes.md)):

- `PUT issue/{id}/assignee`: Assign to me, Unassign, and "Change back to …".
- `POST issue/{id}/transitions`: a move from the status menu or a drag, and its "Change back to …".
- `PUT issue/{id}`: a priority from the priority menu or a drag, or a new title, and its "Change back to …"; each
  request sends that one field only (`priority` or `summary`).
- `POST issue/{id}/comment`: a comment from the Issue view's comment box.
- `POST issue/{id}/remotelink`: a web link (a pull request) from the Issue view's "Add link".
