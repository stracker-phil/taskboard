# Architecture decision records

Each record (`NNNN-<slug>.md`, Nygard format: Status, Context, Decision, Consequences) explains why, in at
most a page; the docs pages explain how. A Decision names the alternatives it rejected. An accepted decision
is never edited: a new record supersedes it, and a dated amendment under Status only corrects facts that
changed.

Records inherited from Daybook, TimeTracker or DataBee say so under Status; they state the decision as it
stands in Taskboard. The numbers in this table are Taskboard's.

| No. | Title | Status |
|---|---|---|
| [0001](0001-rust-owns-the-logic.md) | Rust owns the logic, React only renders | Accepted (2026-10-28) |
| [0002](0002-ipc-in-api-files.md) | IPC only through `api.ts` files | Accepted (2026-10-28) |
| [0003](0003-errors-as-kinds.md) | Errors cross IPC as `{kind, message}` | Accepted (2026-10-28) |
| [0004](0004-generated-ipc-types.md) | TypeScript IPC types are generated from Rust with ts-rs | Accepted (2026-10-28) |
| [0005](0005-panic-unwind.md) | Keep `panic = "unwind"` | Accepted (2026-10-28) |
| [0006](0006-test-doubles-behind-test-util.md) | Test doubles and the demo seed ship behind a `test-util` feature | Accepted (2026-10-28) |
| [0007](0007-color-palettes.md) | Color palettes are complete token sets; no color literal outside a palette | Accepted (2026-10-28) |
| [0008](0008-cli-asks-the-running-app.md) | The CLI is a second process that asks the running app over the socket | Accepted (2026-10-28) |
| [0009](0009-a-scheme-per-system-mode.md) | Appearance follows macOS, with one of three schemes per mode | Superseded in part by 0017 |
| [0010](0010-native-macos-look.md) | The app looks native and light | Superseded by 0017 for the main window |
| [0011](0011-debug-build-is-a-separate-app-with-demo-data.md) | The debug build is a separate app with demo data | Accepted (2026-10-28) |
| [0012](0012-commands-live-in-rust.md) | Commands live in Rust | Accepted (2026-10-28) |
| [0013](0013-network-only-to-listed-hosts.md) | The app talks only to listed hosts | Accepted (2026-10-28) |
| [0014](0014-small-feature-modules.md) | Small feature modules with narrow seams | Accepted (2026-10-28) |
| [0015](0015-signed-with-a-local-certificate.md) | Builds are signed with a local certificate | Accepted (2026-10-28) |
| [0016](0016-closing-the-window-keeps-the-app-running.md) | Closing the window keeps the app running | Accepted (2026-10-28) |
| [0017](0017-the-main-window-follows-the-taskboard-mockup.md) | The main window follows the Taskboard mockup | Accepted (2026-10-28) |
| [0018](0018-reads-go-through-tanstack-query.md) | Reads go through TanStack Query, invalidated by events | Accepted (2026-10-28) |
| [0019](0019-two-sqlite-files-through-rusqlite.md) | Two SQLite files through rusqlite: the owner's setup and a disposable Jira cache | Accepted (2026-10-28) |

Open: integrations with Daybook and TimeTracker.
