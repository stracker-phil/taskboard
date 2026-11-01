import type { DropTarget } from "../api";
import type { Point } from "./followPointer";
import type { ScrollAxis } from "./useIssueDrag";

/** How close to the view's edge the pointer scrolls it, and how fast at most (px per frame). */
const EDGE_PX = 56;
const MAX_STEP = 18;
const PRIORITY_FLAGS = ["highest", "high", "medium", "low"] as const;
/** Inline styles a list places or fades a row with, which the copy drops. */
const PLACEMENT = ["opacity", "transform", "overflow", "overflow-y"];
/** Class modifiers that say how a pill is drawn, not which status it shows. */
const VARIANTS = ["peek", "activity", "tab", "vertical", "closed"];

/** A copy of the dragged row or card that follows the pointer where it was grabbed, while the original stays in
 *  place dimmed (`[data-drag-source]`). The copy hangs on the body (a moving ancestor would carry a fixed element
 *  along) and takes the custom properties the original inherits (the List's column template, the Table's column
 *  widths), so it looks exactly like the row or card. */
export function followDraggedCopy(element: HTMLElement, press: Point, view: HTMLElement, axis: ScrollAxis) {
  const rect = element.getBoundingClientRect();
  const grab = { x: press.x - rect.left, y: press.y - rect.top };
  const pristine = element.cloneNode(true) as HTMLElement;
  // The copy must not answer the lookups the page makes by issue id (menu anchors, the selected row).
  for (const node of [pristine, ...pristine.querySelectorAll<HTMLElement>("[data-issue-id]")]) node.removeAttribute("data-issue-id");
  const copy = pristine.cloneNode(true) as HTMLElement;
  copy.classList.add("drag-copy");
  copy.setAttribute("aria-hidden", "true");
  copy.inert = true;
  carryCustomProperties(element, copy);
  // A virtualized row's wrapper places and fades it in the list (a group opening, the fold); the copy is the row
  // itself, whole and fully drawn.
  for (const name of PLACEMENT) copy.style.removeProperty(name);
  Object.assign(copy.style, { left: `${rect.left}px`, top: `${rect.top}px`, width: `${rect.width}px`, height: `${rect.height}px` });
  document.body.appendChild(copy);
  element.setAttribute("data-drag-source", "");

  let pointer = press;
  let frame = 0;
  let shown: DropTarget | null = null;
  const scroll = () => {
    frame = 0;
    const box = view.getBoundingClientRect();
    const step = axis === "y" ? edgeStep(pointer.y, box.top, box.bottom) : edgeStep(pointer.x, box.left, box.right);
    if (step === 0) return;
    if (axis === "y") view.scrollTop += step;
    else view.scrollLeft += step;
    frame = requestAnimationFrame(scroll);
  };

  return {
    moveTo(at: Point) {
      pointer = at;
      copy.style.left = `${at.x - grab.x}px`;
      copy.style.top = `${at.y - grab.y}px`;
      if (frame === 0) frame = requestAnimationFrame(scroll);
    },
    /** Shows the issue as the drop would leave it (the zone's status or priority), or as it is for `null`.
     *  `named`: the target is one status, whose name the copy's status pill takes. */
    preview(target: DropTarget | null, named: boolean) {
      if (target === shown) return;
      shown = target;
      copy.replaceChildren(...pristine.cloneNode(true).childNodes);
      if (target) restyle(copy, target, named);
    },
    remove() {
      cancelAnimationFrame(frame);
      copy.remove();
      element.removeAttribute("data-drag-source");
    },
  };
}

function carryCustomProperties(from: HTMLElement, to: HTMLElement) {
  const style = getComputedStyle(from);
  for (const name of Array.from(style)) {
    if (name.startsWith("--")) to.style.setProperty(name, style.getPropertyValue(name));
  }
}

function restyle(copy: HTMLElement, target: DropTarget, named: boolean) {
  const mark = target.mark;
  if (mark?.kind === "status") {
    for (const node of copy.querySelectorAll<HTMLElement>(".status-square, .status-pill")) {
      swapModifier(node, node.classList.contains("status-square") ? "status-square--" : "status-pill--", mark.style);
    }
    if (named) for (const node of copy.querySelectorAll<HTMLElement>(".status-pill, .status-name")) node.textContent = target.label;
  } else if (mark?.kind === "priority") {
    const flag = PRIORITY_FLAGS[Math.min(Math.max(mark.level, 1), PRIORITY_FLAGS.length) - 1];
    for (const node of copy.querySelectorAll<SVGElement>(".priority-flag")) swapModifier(node, "priority-flag--", flag);
  }
}

/** Replaces the node's `prefix…` class with `prefix + value`, keeping variants such as `status-pill--peek`. */
function swapModifier(node: Element, prefix: string, value: string) {
  for (const name of [...node.classList]) {
    if (name.startsWith(prefix) && !VARIANTS.includes(name.slice(prefix.length))) node.classList.remove(name);
  }
  node.classList.add(prefix + value);
}

/** Faster the closer the pointer is to an edge; 0 away from both. */
function edgeStep(at: number, start: number, end: number): number {
  if (at < start + EDGE_PX) return -Math.ceil(MAX_STEP * Math.min(1, (start + EDGE_PX - at) / EDGE_PX));
  if (at > end - EDGE_PX) return Math.ceil(MAX_STEP * Math.min(1, (at - (end - EDGE_PX)) / EDGE_PX));
  return 0;
}
