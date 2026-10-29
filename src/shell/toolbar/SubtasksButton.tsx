import type { Ref } from "react";
import type { SubtaskMode } from "../../api";
import { SubtaskIcon } from "../../components/issue/SubtaskIcon";
import { openMenu, type MenuEntry } from "../../components/menu";

const MODES: { mode: SubtaskMode; text: string; hint: string }[] = [
  { mode: "hidden", text: "Hide subtasks", hint: "Top-level issues only" },
  { mode: "hierarchy", text: "Show under parent", hint: "Indented below their parent" },
  { mode: "list", text: "Show as items", hint: "Separate rows, filtered on their own" },
];

/** The mode in a word, for the "…" menu. */
export const MODE_NAMES: Record<SubtaskMode, string> = { hidden: "Hidden", hierarchy: "Under parent", list: "As items" };

/** How the view shows subtasks (ADR 0037): one choice, checked. */
export function subtasksMenu(current: SubtaskMode, choose: (mode: SubtaskMode) => void): MenuEntry[] {
  return [{ caption: "Subtasks" }, ...MODES.map(({ mode, text, hint }) => ({ text, hint, checked: mode === current, run: () => choose(mode) }))];
}

interface Props {
  ref: Ref<HTMLButtonElement>;
  mode: SubtaskMode;
  entries: () => MenuEntry[];
}

/** Right after Columns: only the icon while subtasks are hidden, else active and labeled by the mode. */
export function SubtasksButton({ ref, mode, entries }: Props) {
  const hidden = mode === "hidden";
  return (
    <button
      ref={ref}
      type="button"
      className={hidden ? "toolbar-btn toolbar-btn--icon" : "toolbar-btn toolbar-btn--active"}
      aria-haspopup="menu"
      aria-label={hidden ? "Subtasks: Hidden" : undefined}
      title={hidden ? "Subtasks" : undefined}
      onClick={(e) => openMenu(entries(), e.currentTarget, "Subtasks")}
    >
      <SubtaskIcon />
      {!hidden && (mode === "list" ? "As items" : "Subtasks")}
    </button>
  );
}
