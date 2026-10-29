import type { Ref } from "react";
import type { SubtaskMode } from "../../api";
import { SubtaskIcon } from "../../components/issue/SubtaskIcon";
import { openMenu, type MenuEntry } from "../../components/menu";

const MODES: { mode: SubtaskMode; text: string; hint: string }[] = [
  { mode: "hidden", text: "Hidden", hint: "Only top-level issues" },
  { mode: "hierarchy", text: "Hierarchy", hint: "Under their parent" },
  { mode: "list", text: "List", hint: "As separate rows" },
];

/** How the view shows subtasks (ADR 0037): one choice, checked. */
export function subtasksMenu(current: SubtaskMode, choose: (mode: SubtaskMode) => void): MenuEntry[] {
  return [{ caption: "Subtasks" }, ...MODES.map(({ mode, text, hint }) => ({ text, hint, checked: mode === current, run: () => choose(mode) }))];
}

interface Props {
  ref: Ref<HTMLButtonElement>;
  mode: SubtaskMode;
  entries: () => MenuEntry[];
}

/** Right after Columns; shows as active while subtasks show. */
export function SubtasksButton({ ref, mode, entries }: Props) {
  return (
    <button
      ref={ref}
      type="button"
      className={mode === "hidden" ? "toolbar-btn" : "toolbar-btn toolbar-btn--active"}
      aria-haspopup="menu"
      onClick={(e) => openMenu(entries(), e.currentTarget, "Subtasks")}
    >
      <SubtaskIcon />
      Subtasks
    </button>
  );
}
