# Taskboard links

`taskboard://…` links open a place in the app from anywhere: a note, a chat message, a calendar event, a
shell (`open 'taskboard://issue/NW-12'`). Debug builds answer `taskboard-debug://` instead, so a link never
reaches the wrong app ([adapting.md](adapting.md)).

| Link | Opens |
|---|---|
| `taskboard://issue/NW-12` | the issue in the full Issue view, over its Space, as `taskboard open NW-12` does |
| `taskboard://go/<level>` | a sidebar level at the view it showed last |
| `taskboard://go/<level>/<view>` | a level at one of its views |
| `taskboard://search?q=<text>` | the palette with the text typed in |
| `taskboard://command/<id>` | a command from a short allowlist (below) |

A level and a view are named as the command line names them (`remote/levels.rs`): an id, a Space's key or
name, a filter list's name (`NW%2FOpen%20bugs`) or `everything`, `viewed`, `pinned`. The links the app hands out
use ids, so renaming keeps them working.

## Copy Taskboard Link

The Issue menu (⌘⌥⇧C), a view tab's menu and the menus of a Space and a filter list in the sidebar copy the
link to what they name. Rust writes the link (`links::link_to`, the `app_link` command).

## Following a link

`app.rs` `listen_for_links` hands every link to `remote::links::open`, including the one that started the app.
`links::parse` turns it into a `Link`; `remote::links` then does what the command line does for the same
request (`remote::places::open_issue`, `remote::places::go`) or queues a `LaunchEvent::Search` for the page. A
link that names nothing Taskboard knows, an issue outside the tracked Spaces or a command off the list shows
why in a toast.

## Commands

Any web page can open a link, so a link runs only commands that change nothing in Jira or on the Mac:
`sync:refresh`, `app:show`, `app:settings` and its pages (`app:settings:<page>`). The list is
`LINKABLE_COMMANDS` and `LINKABLE_FAMILIES` in `links.rs`; widen it only with commands of that kind.
