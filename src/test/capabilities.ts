import type { CapabilityRow } from "../api/types";

/** The rows of Settings → Connection as the gateway lists them: reads first, then each kind of change, all on. */
export const capabilityRows = (): CapabilityRow[] => [
  { capability: "account", label: "Your account", detail: "Reads your name and time zone to sign you in.", change: false, enabled: true },
  { capability: "projects", label: "Projects", detail: "Reads the list of projects you can pick from.", change: false, enabled: true },
  { capability: "issues", label: "Issues", detail: "Reads the issues of the projects you track.", change: false, enabled: true },
  { capability: "issueDetails", label: "Issue details", detail: "Reads comments, history and links of an issue you open.", change: false, enabled: true },
  { capability: "checkQueries", label: "Search checks", detail: "Asks Jira whether a filter query is valid.", change: false, enabled: true },
  { capability: "assign", label: "Assign issues", detail: "Sets or clears the assignee of an issue.", change: true, enabled: true },
  { capability: "changeStatus", label: "Change the status", detail: "Moves an issue through its workflow.", change: true, enabled: true },
  { capability: "changePriority", label: "Change the priority", detail: "Sets the priority of an issue.", change: true, enabled: true },
  { capability: "comment", label: "Add comments", detail: "Posts a comment on an issue.", change: true, enabled: true },
  { capability: "addWebLink", label: "Add web links", detail: "Attaches a link, such as a pull request, to an issue.", change: true, enabled: true },
];
