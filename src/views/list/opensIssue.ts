import type { MouseEvent } from "react";

/** A double-click on a row's own buttons (pin, copy, mark read) stays with that button. */
export const opensIssue = (e: MouseEvent) => !(e.target instanceof Element && e.target.closest("button, a"));
