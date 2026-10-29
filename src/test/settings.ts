import type { Connection, Settings, SyncSettings } from "../api/types";

const emptyConnection = (): Connection => ({
  siteUrl: "",
  email: "",
  tokenKind: "classic",
  tokenExpires: null,
  cloudId: null,
  accountId: null,
  displayName: null,
  timeZone: null,
});

export const defaultSync = (): SyncSettings => ({ scopeOverride: null, focusedMinutes: 5, backgroundMinutes: 15, keepDoneDays: 90 });

/** The settings of a fresh install (Automatic appearance), with overrides. */
export const settings = (patch: Partial<Settings> = {}): Settings => ({
  connection: emptyConnection(),
  sync: defaultSync(),
  appearance: "system",
  behaviour: { seenAfterSeconds: 2, pinDefault: "untilUnpinned", openIssuesIn: "preview" },
  ...patch,
});
