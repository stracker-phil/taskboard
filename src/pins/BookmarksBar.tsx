import { ChevronsRight, Folder, MessageSquare } from "lucide-react";
import { useLayoutEffect, useRef, useState, type MouseEvent } from "react";
import { api, type PinItem } from "../api";
import { InlineRename } from "../components/InlineRename";
import { StatusSquare } from "../components/issue/StatusSquare";
import { openContextMenu, openMenu, type MenuEntry } from "../components/menu";
import { usePins } from "../queries/pins";
import { useSidebar, useNavGo, useTabs } from "../queries/sidebar";
import { useDragReorder } from "../shell/Sidebar/useDragReorder";
import { ViewTile } from "../shell/viewLayouts";
import { attempt } from "../state/requests";
import { pinName, pinPlace, useOpenPin } from "./openPin";
import { useOpenInTab } from "../shell/openInTab";
import { fitCount } from "./fitCount";

const GAP = 2;

/** The bookmarks in a row above the content panel, in the owner's order: drag one to move it, right-click to
 *  rename or remove it. What doesn't fit folds into the » menu, by the window's width only. */
export function BookmarksBar() {
  const { data } = usePins();
  const { data: tree } = useSidebar();
  const go = useNavGo();
  const open = useOpenPin();
  const tabs = useTabs();
  const openInTab = useOpenInTab();
  const pins = data?.active ?? [];
  const [renaming, setRenaming] = useState<number | null>(null);
  const { shown, rowRef, measure } = useBarFit(pins.map((pin) => pin.id));
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    const moved = pins[from];
    if (!moved) return;
    const rest = pins.filter((pin) => pin.id !== moved.id);
    void attempt(api.pinMove(moved.id, rest[to]?.id ?? null), "Could not move the bookmark");
  }, "x");
  const setRow = (element: HTMLDivElement | null) => {
    containerRef.current = element;
    rowRef.current = element;
  };

  const openPlaceInTab = (pin: PinItem) => {
    const at = pinPlace(pin);
    if (at) void tabs.open(at.listId, at.viewId, true);
  };
  const toSidebar = () => void attempt(api.modeSet("bookmarksBar", false), "Could not move the bookmarks");
  const rename = (pin: PinItem, title: string | null) => void attempt(api.pinRename(pin.id, title), "Could not rename the bookmark");
  const pinEntries = (pin: PinItem): MenuEntry[] => [
    { text: "Open", run: () => open(pin) },
    ...(pinPlace(pin) ? [{ text: "Open in New Tab", run: () => openPlaceInTab(pin) }] : []),
    { text: "Rename Bookmark…", run: () => setRenaming(pin.id) },
    ...(pin.title ? [{ text: "Use Original Name", hint: pinName({ ...pin, title: null }), run: () => rename(pin, null) }] : []),
    { text: "Remove Bookmark", run: () => void attempt(api.pinRemove(pin.id), "Could not remove the bookmark") },
    "separator",
    { text: "Show Bookmarks in Sidebar", run: toSidebar },
  ];
  const bookmarksPage = tree?.home.find((node) => node.kind === "pinned");
  const more = (e: MouseEvent<HTMLButtonElement>) =>
    openMenu(
      [
        ...pins.slice(shown).map((pin): MenuEntry => ({ text: pinName(pin), icon: <PinIcon pin={pin} />, run: () => open(pin) })),
        ...(shown < pins.length ? ["separator" as const] : []),
        { text: "All Bookmarks", enabled: bookmarksPage !== undefined, run: () => bookmarksPage && void go(bookmarksPage.id) },
      ],
      e.currentTarget,
      "More bookmarks",
    );

  return (
    <nav
      className="bookmarks-bar"
      aria-label="Bookmarks"
      data-tauri-drag-region
      onContextMenu={(e) => {
        if (e.target === e.currentTarget) openContextMenu(e, [{ text: "Show Bookmarks in Sidebar", run: toSidebar }], "Bookmarks bar");
      }}
    >
      <div ref={setRow} className="bookmarks-bar__items" data-tauri-drag-region>
        {pins.length === 0 && <span className="bookmarks-bar__empty">Bookmark an issue, a list or a view to see it here.</span>}
        {pins.map((pin, index) => {
          const hidden = index >= shown;
          return (
            <button
              key={pin.id}
              ref={measure(pin.id)}
              type="button"
              className="bookmarks-bar__item"
              hidden={hidden}
              title={pin.title ? pinName({ ...pin, title: null }) : (pin.reason ?? undefined)}
              {...(hidden ? {} : pin.id === renaming ? item(index) : { ...item(index), ...handle(index) })}
              onClick={(e) => {
                const at = pinPlace(pin);
                if (!at || !openInTab(e, at.listId, at.viewId)) open(pin);
              }}
              onContextMenu={(e) => openContextMenu(e, pinEntries(pin), pinName(pin))}
            >
              <PinIcon pin={pin} />
              {pin.id === renaming ? (
                <InlineRename value={pinName(pin)} label="Bookmark name" onSave={(title) => rename(pin, title)} onDone={() => setRenaming(null)} className="bookmarks-bar__rename" />
              ) : (
                <span className="bookmarks-bar__name">{pinName(pin)}</span>
              )}
            </button>
          );
        })}
        {dropLineY !== null && <span className="bookmarks-bar__drop" style={{ left: dropLineY }} aria-hidden="true" />}
      </div>
      <button type="button" className="bookmarks-bar__more" aria-label="More bookmarks" aria-haspopup="menu" onClick={more}>
        <ChevronsRight size={14} strokeWidth={2} aria-hidden="true" />
      </button>
    </nav>
  );
}

function PinIcon({ pin }: { pin: PinItem }) {
  const target = pin.target;
  switch (target.kind) {
    case "issue":
      return <StatusSquare style={target.style} />;
    case "comment":
      return <MessageSquare size={13} strokeWidth={2} aria-hidden="true" className="bookmarks-bar__icon" />;
    case "list":
      return <Folder size={13} strokeWidth={2} aria-hidden="true" className="bookmarks-bar__icon" />;
    case "view":
      return <ViewTile layout={target.layout} icon={target.icon} color={target.color} />;
  }
}

/** How many bookmarks fit the row, measured at their natural width; the rest fold into the » menu. Only the row's
 *  width decides, never a click (docs/ux.md → Stable controls). */
function useBarFit(ids: number[]) {
  const rowRef = useRef<HTMLDivElement | null>(null);
  const widths = useRef(new Map<number, number>());
  const [available, setAvailable] = useState(Number.POSITIVE_INFINITY);
  const [version, setVersion] = useState(0);
  const [shown, setShown] = useState(ids.length);

  useLayoutEffect(() => {
    const row = rowRef.current;
    if (!row || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => setAvailable(row.clientWidth));
    observer.observe(row);
    return () => observer.disconnect();
  }, []);

  const key = ids.join();
  useLayoutEffect(() => {
    const next = fitCount(available, ids.map((id) => widths.current.get(id) ?? 0), GAP);
    setShown((current) => (current === next ? current : next));
    // `ids` is read through `key`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [available, version, key]);

  /** Records an item's width once it has been drawn; a hidden one keeps the width it had. */
  const measure = (id: number) => (element: HTMLElement | null) => {
    if (!element || element.hidden) return;
    const width = element.getBoundingClientRect().width;
    if (width === 0 || widths.current.get(id) === width) return;
    widths.current.set(id, width);
    setVersion((v) => v + 1);
  };
  return { shown, rowRef, measure };
}
