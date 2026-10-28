import type { Column, Group, IssueDetail, Person, Row, StatusStyleRow, ViewData } from "../api/types";

export const ALEX: Person = { accountId: "demo-account-1", name: "Alex Morgan", initials: "AM", color: 1 };
export const SAM: Person = { accountId: "demo-account-2", name: "Sam Lee", initials: "SL", color: 2 };

/** A plain issue row of the "Website relaunch" Space; override per test. */
export function row(patch: Partial<Row> = {}): Row {
  return {
    id: "10042",
    key: "NW-1042",
    summary: "Redesign the pricing page",
    style: "progress",
    statusName: "In Progress",
    commentCount: 0,
    changed: false,
    changeNote: null,
    assignees: [],
    epic: null,
    release: null,
    priority: null,
    labels: [],
    sprint: null,
    due: null,
    custom: {},
    ...patch,
  };
}

/** An issue as the Peek shows it; override per test. */
export function issueDetail(patch: Partial<IssueDetail> = {}): IssueDetail {
  return {
    id: "10042",
    key: "NW-1042",
    issueType: "Story",
    summary: "Checkout: address validation for DACH postcodes",
    style: "progress",
    statusName: "In Progress",
    epic: { id: "10001", name: "Checkout", color: 1 },
    releases: ["2.4.0"],
    priority: { name: "High", level: 2 },
    assignees: [ALEX],
    reporter: SAM,
    sprint: null,
    labels: [],
    due: null,
    createdAt: "2026-10-01T09:00:00Z",
    updatedAt: "2026-10-20T14:30:00Z",
    descriptionHtml: "<p>Validate German, Austrian and Swiss postcodes.</p>",
    commentCount: 4,
    isEpic: false,
    gone: false,
    ...patch,
  };
}

export function group(patch: Partial<Group> = {}): Group {
  return { key: "status:progress", label: "In Progress", mark: null, count: 0, collapsedByDefault: false, rowIds: [], ...patch };
}

/** A group holding these rows, with its count filled in. */
export function groupOf(rows: Row[], patch: Partial<Group> = {}): Group {
  return group({ count: rows.length, rowIds: rows.map((r) => r.id), ...patch });
}

export function viewData(groups: Group[], rows: Row[], patch: Partial<ViewData> = {}): ViewData {
  return { groupBy: "status", groups, rows: Object.fromEntries(rows.map((r) => [r.id, r])), columns: [], ...patch };
}

export const COLUMNS: Column[] = [
  { id: "assignee", name: "Assignee", custom: false },
  { id: "epic", name: "Epic", custom: false },
  { id: "priority", name: "Priority", custom: false },
  { id: "customfield_10071", name: "Reviewer", custom: true },
];

export function statusStyle(patch: Partial<StatusStyleRow> = {}): StatusStyleRow {
  return { id: "3", name: "In Progress", category: "indeterminate", detected: "progress", chosen: null, ...patch };
}
