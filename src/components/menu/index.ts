/** The app's one menu: context menus at the mouse, and the menus of pop-up buttons under them. */
import type { MouseEvent } from "react";
import type { MenuAt, MenuEntry } from "../../state/slices/menu";
import { useStore } from "../../state/store";

export type { MenuAt, MenuEntry, MenuItem } from "../../state/slices/menu";
export { MenuHost } from "./MenuHost";

/** Opens a menu at a point or under an element; `label` is its accessible name. */
export function openMenu(entries: MenuEntry[], at: MenuAt, label?: string) {
  useStore.getState().openMenu(entries, at, label);
}

/** A right-click's menu, at the mouse; the WebView's own menu stays shut. */
export function openContextMenu(e: MouseEvent, entries: MenuEntry[], label?: string) {
  e.preventDefault();
  e.stopPropagation();
  openMenu(entries, { x: e.clientX, y: e.clientY }, label);
}
