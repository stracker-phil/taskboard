import type { IssueHit, ListHit, MenuCommand, TextPart, ViewHit } from "../api/types";

/** Plain text, or the parts of a match: `parts("Fix the ", ["login"], " form")` marks "login" strong. */
export function parts(...pieces: (string | [string])[]): TextPart[] {
  return pieces.map((p) => (typeof p === "string" ? { text: p, strong: false } : { text: p[0], strong: true }));
}

/** A menu item from Rust (File › Refresh now); override per test. */
export function menuCommand(patch: Partial<MenuCommand> = {}): MenuCommand {
  return { menu: "File", label: "Refresh now", command: "sync:refresh", accelerator: null, checked: false, ...patch };
}

/** An issue hit in the "Website relaunch" project; override per test. */
export function issueHit(patch: Partial<IssueHit> = {}): IssueHit {
  return {
    id: "10043",
    key: "NW-43",
    projectId: "10001",
    summary: parts("Fix the contact form"),
    style: "progress",
    statusName: "In Progress",
    matchedIn: "summary",
    comment: null,
    gone: false,
    ...patch,
  };
}

/** A filter list of the "Website relaunch" Space; override per test. */
export function listHit(patch: Partial<ListHit> = {}): ListHit {
  return { id: 21, name: "Current sprint", projectId: "10001", isSpace: false, ...patch };
}

/** A view of the "Current sprint" filter list; override per test. */
export function viewHit(patch: Partial<ViewHit> = {}): ViewHit {
  return { list: listHit(), viewId: 201, name: "Sprint board", layout: "board", ...patch };
}
