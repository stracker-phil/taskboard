import type { MouseEvent } from "react";

/** A ⇧-click picks a range of rows; without this it would also select their text. */
export const keepTextUnselected = (e: MouseEvent) => {
  if (e.shiftKey) e.preventDefault();
};

/** A double-click on a row's own controls (pin, copy, mark read, its checkbox) stays with that control, and one with
 *  ⌘ or ⇧ picks rows rather than opening one. */
export const opensIssue = (e: MouseEvent) =>
  !e.metaKey && !e.shiftKey && !e.ctrlKey && !(e.target instanceof Element && e.target.closest("button, a, input"));
