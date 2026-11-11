import type { PrStatus, PullRequest } from "../api";

/** GitHub's answer for a pull request: open, nine of twelve checks passed, approved; override per test. */
export const prStatus = (patch: Partial<PrStatus> = {}): PrStatus => ({
  title: "Validate DACH postcodes",
  state: "open",
  checks: { state: "passing", passed: 9, total: 12 },
  review: "approved",
  ...patch,
});

/** An openable pull request without a status yet. */
export const pullRequest = (patch: Partial<PullRequest> = {}): PullRequest => ({
  id: "30002",
  url: "https://github.com/example/shop/pull/481",
  number: 481,
  repo: "example/shop",
  title: "Validate DACH postcodes",
  openable: true,
  status: null,
  problem: null,
  ...patch,
});
