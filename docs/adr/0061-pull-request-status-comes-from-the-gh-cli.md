# 0061. Pull request status comes from the GitHub CLI

## Status

Accepted (2026-11-11). Owner's request. Builds on [0013](0013-network-only-to-listed-hosts.md): Taskboard itself
still talks only to its listed hosts.

## Context

An issue's pull requests are web links, shown far down the Issue view with only their address. The owner wants
them near the top, with what matters for the issue: open, draft, merged or closed; whether the checks pass;
whether the pull request is approved or still waits for a review. GitHub knows this, and the owner already uses
the GitHub CLI (`gh`), signed in with their own account.

## Decision

- **Rust runs `gh`; Taskboard holds no GitHub token and opens no connection to GitHub.** `pull_requests/gh.rs`
  runs `gh pr view <n> --repo <owner>/<repo> --json …` with fixed arguments, no shell, no prompt and a 20 second
  limit. It looks for `gh` in Homebrew's two places, since an app opened from the Finder has no shell `PATH`.
- **Only GitHub pull requests are asked about,** named by an address of the form
  `https://github.com/<owner>/<repo>/pull/<n>`; owner and repo must be plain GitHub names, so an untrusted link
  can't become a flag or a path (`pull_requests/github.rs`). GitLab and Bitbucket links show as pull requests
  without a status.
- **Statuses live in memory for `pull_requests::FRESH_FOR` (5 minutes),** failures too. The page asks Rust for
  what is known (`issue_pull_requests`); the answer says when a status is due (`refreshInMs`), and the page then
  asks Rust to sync (`issue_sync_pull_requests`). So an opened issue refreshes what is older than 5 minutes, and
  repeats every 5 minutes while a pull request isn't merged.
- **The debug build never runs `gh`:** it answers from demo statuses (`test_support::FakePullRequests`).
- Rejected: calling GitHub's API from `net.rs` (a new host and a token to keep for one read); passing the link's
  address to `gh` as it is (untrusted text as an argument); keeping statuses in the database (they're stale in
  minutes).

## Consequences

- Without `gh`, or signed out, a pull request shows "No status"; its tooltip says why.
- What reaches GitHub is what `gh pr view` sends, under the owner's own `gh` login and its scopes.
