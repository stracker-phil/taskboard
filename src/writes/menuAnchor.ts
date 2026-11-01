import type { MenuAt } from "../components/menu";

/** Where a key opens an issue's menu: under the first of its `attribute` buttons or cells that is in sight, else
 *  under its row or card, else near the top of the window. A cell hidden under the Preview isn't in sight. */
export function menuAnchor(attribute: string, issueId: string): MenuAt {
  const id = CSS.escape(issueId);
  const inSight = (selector: string) => [...document.querySelectorAll<HTMLElement>(selector)].find(isInSight);
  return inSight(`[${attribute}="${id}"]`) ?? inSight(`[data-issue-id="${id}"]`) ?? { x: window.innerWidth / 2, y: 120 };
}

function isInSight(element: HTMLElement): boolean {
  const box = element.getBoundingClientRect();
  // Without a layout to hit-test, everything counts as in sight.
  const hit = document.elementFromPoint?.(box.x + box.width / 2, box.y + box.height / 2);
  return hit === undefined || (hit !== null && element.contains(hit));
}
