import type { MouseEvent } from "react";

/** A ⇧-click picks a range of rows; without this it would also select their text. */
export const keepTextUnselected = (e: MouseEvent) => {
  if (e.shiftKey) e.preventDefault();
};

/** A double-click on a row's own controls (pin, copy, mark read, its checkbox) stays with that control, and one with
 *  ⇧ alone picks rows rather than opening one; with ⌘ it opens the issue in a new tab. */
export const opensIssue = (e: MouseEvent) =>
  !(e.shiftKey && !e.metaKey) && !e.ctrlKey && !(e.target instanceof Element && e.target.closest("button, a, input"));
