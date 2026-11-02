import type { Command } from "./registry";

type SelectTab = (index: number) => void;

let selectTab: SelectTab | null = null;

/** The tab strip takes ⌘⇧1–⌘⇧9 while shown; returns the release. */
export function onSelectTab(handler: SelectTab): () => void {
  selectTab = handler;
  return () => {
    if (selectTab === handler) selectTab = null;
  };
}

/** ⌘⇧1–⌘⇧9: the nth tab of the main window. */
export const TAB_KEY_COMMANDS: Command[] = Array.from({ length: 9 }, (_, i) => ({
  id: `tab.select.${i + 1}`,
  title: `Show tab ${i + 1}`,
  scope: "main",
  defaultBinding: `Mod+Shift+${i + 1}`,
  when: () => selectTab !== null,
  run: () => selectTab?.(i),
}));
