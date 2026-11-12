import type { Behaviour, Connection, Notifications, Settings, SyncSettings } from "../api/types";

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

export const defaultSync = (): SyncSettings => ({ scopeOverride: null, focusedMinutes: 5, backgroundMinutes: 15, keepDoneDays: 90, excludedTypes: [] });

/** The behaviour of a fresh install (changes to Jira off), with overrides. */
export const behaviour = (patch: Partial<Behaviour> = {}): Behaviour => ({
  seenAfterSeconds: 2,
  pinDefault: "untilUnpinned",
  openIssuesIn: "preview",
  allowJiraChanges: false,
  disabledChanges: [],
  meRules: [
    { rule: { kind: "field", field: "reporter" }, on: false },
    { rule: { kind: "mentioned" }, on: false },
    { rule: { kind: "subtasks" }, on: false },
  ],
  ...patch,
});

/** The notification settings of a fresh install: every event in app, the ones about you on macOS, with overrides. */
export const notifications = (patch: Partial<Notifications> = {}): Notifications => ({
  scope: "allIssues",
  involvement: { assigned: true, reported: true, peopleFields: true },
  excludedTypes: [],
  inApp: ["mentionsMe", "assignsMe", "unassignsMe", "addsMeToPeopleField", "newComment", "movedToDone", "reopened", "otherChange", "dueOrPriority", "created"],
  mac: ["mentionsMe", "assignsMe", "addsMeToPeopleField"],
  ...patch,
});

/** The settings of a fresh install (Automatic appearance), with overrides. */
export const settings = (patch: Partial<Settings> = {}): Settings => ({
  connection: emptyConnection(),
  sync: defaultSync(),
  appearance: "system",
  accent: "blue",
  behaviour: behaviour(),
  notifications: notifications(),
  ...patch,
});
