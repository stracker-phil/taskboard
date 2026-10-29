import type { Connection, ConnectionView, ProjectCounts, ProjectRow, SyncOverview, TokenExpiry } from "../api/types";

/** A connection that was tested successfully; override per test. */
export function connection(patch: Partial<Connection> = {}): Connection {
  return {
    siteUrl: "https://example.atlassian.net/",
    email: "alex.morgan@example.com",
    tokenKind: "classic",
    tokenExpires: null,
    cloudId: null,
    accountId: "demo-account-1",
    displayName: "Alex Morgan",
    timeZone: "Europe/Berlin",
    ...patch,
  };
}

export function connectionView(patch: Partial<Connection> = {}, hasToken = true, expiry: TokenExpiry = { state: "fine" }): ConnectionView {
  return { connection: connection(patch), hasToken, expiry };
}

/** A tracked project as the cache holds it; override per test. */
export function projectCounts(patch: Partial<ProjectCounts> = {}): ProjectCounts {
  return { id: "10001", key: "NW", name: "Website relaunch", cached: 33, open: 21, lastFullAt: null, ...patch };
}

/** An idle, never-synced overview with no tracked projects; override per test. */
export function syncOverview(patch: Partial<SyncOverview> = {}): SyncOverview {
  return { state: { state: "idle", lastOk: null }, lastFullAt: null, projects: [], ...patch };
}

export const PROJECTS:ProjectRow[] = [
  { id: "10001", key: "NW", name: "Website relaunch", tracked: true },
  { id: "10002", key: "LUM", name: "Lumen iOS app", tracked: false },
];
