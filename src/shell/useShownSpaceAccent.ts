import type { Accent } from "../api";
import { useNav, useSidebar } from "../queries/sidebar";
import { findLevel, spaceOf } from "./levels";

/** The accent of the Space the main window shows, when it has its own. */
export function useShownSpaceAccent(): Accent | null {
  const { data: tree } = useSidebar();
  const { data: nav } = useNav();
  const listId = nav?.location?.listId;
  if (!tree || listId === undefined) return null;
  return spaceOf(findLevel(tree, listId))?.accent ?? null;
}
