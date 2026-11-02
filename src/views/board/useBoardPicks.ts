import { useCallback, useEffect, useMemo, useRef } from "react";
import type { BoardData } from "../../api";
import { useStore } from "../../state/store";
import { pickRange, toggleBox, togglePick } from "../list/picking";
import type { PickKeys } from "../list/useRowClicks";

const NO_PICKS: string[] = [];

/** The Board's picked cards (`list/picking.ts`), in reading order: column by column, top to bottom, open columns
 *  only. A ⌘ or ⇧ click picks, a plain click drops the picks and opens the card. A ⌘ double-click opens the card in a
 *  new tab and leaves the picks and the current card as they were before it. */
export function useBoardPicks(
  placeKey: string,
  data: BoardData,
  selected: string | null,
  select: (id: string) => void,
  openCard: (id: string) => void,
  openInTab: (id: string, show: boolean) => void,
  setSelected: (id: string | null) => void,
) {
  const stored = useStore((s) => s.picked[placeKey]) ?? NO_PICKS;
  const storePicked = useStore((s) => s.setPicked);
  const order = useMemo(() => data.columns.filter((column) => !column.collapsed).flatMap((column) => column.ids), [data.columns]);
  const picked = useMemo(() => stored.filter((id) => id in data.rows), [stored, data.rows]);
  useEffect(() => () => storePicked(placeKey, []), [storePicked, placeKey]);

  const latest = useRef({ order, picked, selected });
  useEffect(() => {
    latest.current = { order, picked, selected };
  });
  const setPicked = useCallback((ids: string[]) => storePicked(placeKey, ids), [storePicked, placeKey]);
  const beforeMetaClick = useRef<{ picked: string[]; selected: string | null } | null>(null);

  const onCard = useCallback(
    (id: string, mods?: PickKeys) => {
      const { order, picked, selected } = latest.current;
      if (mods?.metaKey) {
        if ((mods.detail ?? 1) <= 1) beforeMetaClick.current = { picked, selected };
        setPicked(togglePick(order, picked, id, selected));
        select(id);
        return;
      }
      if (mods?.shiftKey) {
        setPicked(pickRange(order, selected, id));
        if (selected === null) select(id);
        return;
      }
      if (picked.length > 0) setPicked([]);
      openCard(id);
    },
    [setPicked, select, openCard],
  );
  const onOpenInTab = useCallback(
    (id: string, show: boolean) => {
      const before = beforeMetaClick.current;
      if (before) {
        setPicked(before.picked);
        setSelected(before.selected);
      }
      openInTab(id, show);
    },
    [setPicked, setSelected, openInTab],
  );
  const onToggleBox = useCallback((id: string) => setPicked(toggleBox(latest.current.order, latest.current.picked, id)), [setPicked]);
  const pickedIds = useMemo(() => new Set(picked), [picked]);
  return { picked, pickedIds, setPicked, onCard, onToggleBox, onOpenInTab, pickAll: () => setPicked(order) };
}
