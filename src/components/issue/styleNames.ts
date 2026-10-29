import type { Style } from "../../api";

/** Each status type's name, in the order a picker lists them (Backlog first, the way work moves). */
export const STYLE_NAMES: Record<Style, string> = {
  backlog: "Backlog",
  todo: "To do",
  progress: "In progress",
  review: "In review",
  blocked: "Blocked",
  done: "Done",
};

export const STYLE_ORDER = Object.keys(STYLE_NAMES) as Style[];
