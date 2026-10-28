import { keepPreviousData, useQuery, useQueryClient } from "@tanstack/react-query";
import { CornerDownLeft, Search, Zap } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { api, type IssueHit, type ListHit, type Nav, type TextPart } from "../api";
import { SpaceTile } from "../components/SpaceTile";
import { StatusPill } from "../components/issue/StatusPill";
import { StatusSquare } from "../components/issue/StatusSquare";
import { NAV_KEY, useNav, useNavGo, useSidebar } from "../queries/sidebar";
import { useStore } from "../state/store";
import { useIssueActions } from "../views/list/useIssueActions";
import { type PaletteAction, paletteActions, saveSearch } from "./actions";

/** At most this many actions show, so issues and lists stay in sight. */
const ACTION_LIMIT = 6;

type Item = { kind: "issue"; hit: IssueHit } | { kind: "list"; hit: ListHit } | { kind: "action"; action: PaletteAction };

/** ⌘K over a dimmed window (artboard 7): issues by key and full text, filter lists, actions. */
export function CommandPalette() {
  const open = useStore((s) => s.paletteOpen);
  return open ? <Palette /> : null;
}

function Marked({ parts }: { parts: TextPart[] }) {
  return (
    <>
      {parts.map((part, i) => (part.strong ? <strong key={i}>{part.text}</strong> : <span key={i}>{part.text}</span>))}
    </>
  );
}

function Palette() {
  const close = useStore((s) => s.setPaletteOpen);
  const updatePlace = useStore((s) => s.updatePlace);
  const loadPlace = useStore((s) => s.loadPlace);
  const [text, setText] = useState("");
  const [active, setActive] = useState(0);
  const { data: tree } = useSidebar();
  const { data: nav } = useNav();
  const client = useQueryClient();
  const go = useNavGo();
  const actions = useIssueActions();
  const listRef = useRef<HTMLDivElement>(null);
  const { data: results } = useQuery({
    queryKey: ["palette", text],
    queryFn: () => api.paletteQuery(text),
    enabled: text.trim() !== "",
    placeholderData: keepPreviousData,
  });
  const issues = text.trim() ? (results?.issues ?? []) : [];
  const lists = text.trim() ? (results?.lists ?? []) : [];
  // Issues come first, so a low index is an issue; elsewhere the first issue is meant.
  const activeIssue = issues[active] ?? issues[0];
  const shownActions = paletteActions({
    text,
    issue: activeIssue,
    openInJira: actions.openInJira,
    tree,
    nav,
    go: (listId) => go(listId),
    viewId: nav?.location?.viewId ?? null,
  }).slice(0, ACTION_LIMIT);

  const all: Item[] = [
    ...issues.map((hit): Item => ({ kind: "issue", hit })),
    ...lists.map((hit): Item => ({ kind: "list", hit })),
    ...shownActions.map((action): Item => ({ kind: "action", action })),
  ];

  useEffect(() => setActive(0), [text]);
  useEffect(() => {
    listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: "nearest" });
  }, [active]);

  const dismiss = () => close(false);

  /** In its Space's last view with the peek open. */
  const openIssue = async (hit: IssueHit) => {
    const space = tree?.spaces.find((s) => s.projectId === hit.projectId);
    dismiss();
    if (!space) return;
    await go(space.listId);
    const location = client.getQueryData<Nav>(NAV_KEY)?.location;
    if (!location) return;
    await loadPlace(location.listId, location.viewId);
    updatePlace(location.listId, location.viewId, { selectedId: hit.id, peekOpen: true });
  };
  const run = async (item: Item | undefined) => {
    if (!item) return;
    if (item.kind === "issue") return openIssue(item.hit);
    dismiss();
    if (item.kind === "list") return go(item.hit.id);
    return item.action.run();
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey;
    const handled = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    if (e.key === "Escape") {
      handled();
      dismiss();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      handled();
      if (all.length > 0) setActive((at) => (at + (e.key === "ArrowDown" ? 1 : -1) + all.length) % all.length);
    } else if (e.key === "Enter" && mod) {
      handled();
      if (activeIssue && actions.openInJira) {
        dismiss();
        actions.openInJira(activeIssue.key);
      }
    } else if (e.key === "Enter") {
      handled();
      void run(all[active]);
    } else if (mod && e.key.toLowerCase() === "c" && activeIssue && (window.getSelection()?.isCollapsed ?? true) && isCaretOnly(e.target)) {
      handled();
      actions.copyKey(activeIssue.key);
    } else if (mod && e.key.toLowerCase() === "s" && text.trim()) {
      handled();
      dismiss();
      void saveSearch(text.trim(), tree, nav, (id) => go(id));
    } else if (mod && e.key.toLowerCase() === "r") {
      handled();
      dismiss();
      void shownActions.find((a) => a.id === "refresh")?.run();
    }
  };

  let index = -1;
  const option = (key: string, className: string, content: ReactNode, item: Item) => {
    index += 1;
    const at = index;
    return (
      <div
        key={key}
        role="option"
        aria-selected={at === active}
        className={at === active ? `${className} palette__option--active` : className}
        onMouseMove={() => setActive(at)}
        onClick={() => void run(item)}
      >
        {content}
      </div>
    );
  };
  const spaceOf = (projectId: string | null) => tree?.spaces.find((s) => s.projectId === projectId);
  const counts = [issues.length && `${issues.length} ${issues.length === 1 ? "issue" : "issues"}`, `${shownActions.length} ${shownActions.length === 1 ? "action" : "actions"}`]
    .filter(Boolean)
    .join(" · ");

  return (
    <div className="palette-scrim" onMouseDown={(e) => e.target === e.currentTarget && dismiss()}>
      <div className="palette" role="dialog" aria-label="Command palette" onKeyDown={onKeyDown}>
        <label className="palette__input">
          <Search size={18} strokeWidth={2} aria-hidden="true" />
          <input
            autoFocus
            role="combobox"
            aria-expanded="true"
            aria-controls="palette-results"
            aria-label="Search issues, filter lists and actions"
            placeholder="Search issues, filter lists and actions"
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
          <span className="palette__count">{counts}</span>
        </label>
        <div ref={listRef} id="palette-results" className="palette__results" role="listbox" aria-label="Results">
          {issues.length > 0 && (
            <section aria-label="Issues">
              <h2 className="palette__caps">Issues</h2>
              {issues.map((hit) =>
                option(
                  `issue:${hit.id}`,
                  "palette__option palette__issue",
                  <>
                    <span className="palette__issue-line">
                      <StatusSquare style={hit.style} />
                      <span className="palette__key">{hit.key}</span>
                      <span className="palette__summary">
                        <Marked parts={hit.summary} />
                      </span>
                      {hit.gone && <span className="palette__gone">gone</span>}
                      <StatusPill style={hit.style} name={hit.statusName} variant="activity" />
                      <CornerDownLeft className="palette__enter" size={12} strokeWidth={2} aria-hidden="true" />
                    </span>
                    {hit.comment && (
                      <span className="palette__comment">
                        Matched in comment: “<Marked parts={hit.comment} />”
                      </span>
                    )}
                  </>,
                  { kind: "issue", hit },
                ),
              )}
            </section>
          )}
          {lists.length > 0 && (
            <section aria-label="Filter lists">
              {issues.length > 0 && <hr className="palette__separator" />}
              <h2 className="palette__caps">Filter lists</h2>
              {lists.map((hit) => {
                const space = spaceOf(hit.projectId);
                return option(
                  `list:${hit.id}`,
                  "palette__option palette__list",
                  <>
                    {space ? <SpaceTile space={space} size={20} /> : <span className="palette__tile-gap" />}
                    <span className="palette__summary">{space && !hit.isSpace ? `${space.name} › ${hit.name}` : hit.name}</span>
                    <span className="palette__meta">{hit.isSpace ? "Space" : "filter list"}</span>
                  </>,
                  { kind: "list", hit },
                );
              })}
            </section>
          )}
          {shownActions.length > 0 && (
            <section aria-label="Actions">
              {(issues.length > 0 || lists.length > 0) && <hr className="palette__separator" />}
              <h2 className="palette__caps">Actions</h2>
              {shownActions.map((action) =>
                option(
                  `action:${action.id}`,
                  "palette__option palette__action",
                  <>
                    <Zap size={16} strokeWidth={1.8} aria-hidden="true" />
                    <span className="palette__summary">{action.label}</span>
                    {action.hint && <span className="palette__meta">{action.hint}</span>}
                  </>,
                  { kind: "action", action },
                ),
              )}
            </section>
          )}
        </div>
        <footer className="palette__footer">
          <span>↑↓ move</span>
          <span>↩ open</span>
          <span>⌘↩ open in Jira</span>
          <span>⌘C copy key</span>
          <span className="palette__spacer" />
          <span>esc close</span>
        </footer>
      </div>
    </div>
  );
}

/** ⌘C copies the issue key only while the field has no selected text to copy. */
function isCaretOnly(target: EventTarget) {
  return !(target instanceof HTMLInputElement) || target.selectionStart === target.selectionEnd;
}
