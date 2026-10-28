import type { Connection, ConnectionView, ProjectRow, TokenExpiry } from "../api/types";

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
    timeZone: "Europe/Vienna",
    ...patch,
  };
}

export function connectionView(patch: Partial<Connection> = {}, hasToken = true, expiry: TokenExpiry = { state: "fine" }): ConnectionView {
  return { connection: connection(patch), hasToken, expiry };
}

export const PROJECTS: ProjectRow[] = [
  { id: "10001", key: "NW", name: "Website relaunch", tracked: true },
  { id: "10002", key: "LUM", name: "Lumen iOS app", tracked: false },
];
