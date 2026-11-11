import type { Box } from "../components/menu/placement";

/** What decides where text wraps in the field. */
const COPIED = [
  "fontFamily",
  "fontSize",
  "fontWeight",
  "fontStyle",
  "lineHeight",
  "letterSpacing",
  "tabSize",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
] as const;

/** The caret's line box at `index`, in window coordinates. A textarea doesn't tell; a hidden copy of its text
 *  laid out the same way does. */
export function caretBox(input: HTMLTextAreaElement, index: number): Box {
  const style = getComputedStyle(input);
  const copy = document.createElement("div");
  for (const property of COPIED) copy.style[property] = style[property];
  Object.assign(copy.style, {
    position: "absolute",
    visibility: "hidden",
    top: "0",
    left: "0",
    boxSizing: "border-box",
    width: `${input.clientWidth}px`,
    whiteSpace: "pre-wrap",
    overflowWrap: "break-word",
  });
  copy.textContent = input.value.slice(0, index);
  const marker = document.createElement("span");
  marker.textContent = "\u200b";
  copy.append(marker);
  document.body.append(copy);
  const field = input.getBoundingClientRect();
  const left = field.left + input.clientLeft + marker.offsetLeft - input.scrollLeft;
  const top = field.top + input.clientTop + marker.offsetTop - input.scrollTop;
  const height = marker.offsetHeight;
  copy.remove();
  return { left, top, right: left, bottom: top + height };
}
