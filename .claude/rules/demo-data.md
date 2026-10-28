# Demo data only

The repo is published as source. Nothing in it may carry the owner's real data.

- Code, comments, docs, ADRs, tests, fixtures, seeds, commit messages and screenshots use made-up values
  only: invented projects, tasks, people, clients, companies and dates. Use `example.com` addresses and
  hosts. Never copy a real name, customer, ticket key, path under the owner's home or token into the repo,
  not even as an example.
- The debug build never sees the owner's data: it runs on its own data folder
  ([ADR 0011](../../docs/adr/0011-debug-build-is-a-separate-app-with-demo-data.md)) and starts from the demo
  seed in `src-tauri/src/test_support/` the first time it runs.
- Screenshots that are committed (`docs/screens/`) come from the debug app on the demo seed, produced by a
  script, never from the release app.
- When the owner shares real examples to explain a feature, translate them into neutral demo values before
  they reach a file.
