import type { FilterOptions, ListConditions } from "../api/types";

/** The 14 standard filter fields and no cached choices; override per test. */
export function filterOptions(patch: Partial<FilterOptions> = {}): FilterOptions {
  return {
    fields: [
      { id: "status", name: "Status", kind: "status" },
      { id: "statusCategory", name: "Status category", kind: "statusCategory" },
      { id: "type", name: "Type", kind: "type" },
      { id: "assignee", name: "Assignee", kind: "user" },
      { id: "reporter", name: "Reporter", kind: "user" },
      { id: "priority", name: "Priority", kind: "priority" },
      { id: "epic", name: "Epic", kind: "epic" },
      { id: "release", name: "Release", kind: "release" },
      { id: "sprint", name: "Sprint", kind: "sprint" },
      { id: "labels", name: "Labels", kind: "label" },
      { id: "due", name: "Due date", kind: "date" },
      { id: "updated", name: "Updated", kind: "date" },
      { id: "created", name: "Created", kind: "date" },
      { id: "project", name: "Project", kind: "project" },
    ],
    statuses: [],
    types: [],
    users: [],
    priorities: [],
    epics: [],
    releases: [],
    sprints: [],
    labels: [],
    projects: [],
    ...patch,
  };
}

/** A level's conditions: Everything's (none) unless overridden. */
export function listConditions(patch: Partial<ListConditions> = {}): ListConditions {
  return { kind: "everything", locked: [], own: [], error: null, ...patch };
}
