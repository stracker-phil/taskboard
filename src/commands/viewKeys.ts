import type { Command } from "./registry";

type SelectView = (index: number) => void;

let selectView: SelectView | null = null;

/** The view tabs take ⌘1–⌘9 while shown; returns the release. */
export function onSelectView(handler: SelectView): () => void {
  selectView = handler;
  return () => {
    if (selectView === handler) selectView = null;
  };
}

/** ⌘1–⌘9: the nth view tab of the current level. */
export const VIEW_KEY_COMMANDS: Command[] = Array.from({ length: 9 }, (_, i) => ({
  id: `view.select.${i + 1}`,
  title: `Open view ${i + 1}`,
  scope: "main",
  defaultBinding: `Mod+${i + 1}`,
  when: () => selectView !== null,
  run: () => selectView?.(i),
}));
