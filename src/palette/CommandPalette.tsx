import { keepPreviousData, useQuery, useQueryClient } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { api, type IssueHit, type ListHit, type Nav, type ViewHit } from "../api";
import { NAV_KEY, useNav, useNavGo, useSidebar } from "../queries/sidebar";
import { runBusCommand } from "../state/requests";
import { useStore } from "../state/store";
import { useIssueActions } from "../views/list/useIssueActions";
import { collectCommands, snapshotPage, type PaletteCommand } from "./commands";
import { CommandRow, IssueRow, ListRow, PrefixButtons, PrefixRow, ViewRow } from "./PaletteRows";
import { parsePrefix, PREFIXES, SCOPES, type Prefix } from "./prefixes";
import { ranked } from "./rank";
import { saveSearch } from "./saveSearch";

/** Without a prefix at most this many commands show, so issues and lists stay in sight. */
const COMMAND_LIMIT = 6;
/** Searches once typing pauses, like the toolbar's text filter. */
const DEBOUNCE_MS = 80;
/** Every searched prefix is its own entry: unused ones go soon. */
const RESULTS_GC_MS = 15_000;
const MENU_COMMANDS_KEY = ["menuCommands"];

/** `value` once it has stopped changing for `DEBOUNCE_MS`. */
function useSettled(value: string): string {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [value]);
  return settled;
}

type Item = { kind: "issue"; hit: IssueHit } | { kind: "list"; hit: ListHit } | { kind: "view"; hit: ViewHit } | { kind: "command"; command: PaletteCommand } | { kind: "prefix"; prefix: Prefix };

interface Section {
  heading: string;
  items: Item[];
}

const plural = (count: number, word: string) => count > 0 && `${count} ${word}${count === 1 ? "" : "s"}`;

/** Commands grouped under their menus while nothing is typed, else one list, best match first. */
function commandSections(commands: PaletteCommand[], query: string): Section[] {
  const items = (list: PaletteCommand[]) => list.map((command): Item => ({ kind: "command", command }));
  if (query) return [{ heading: "Commands", items: items(ranked(commands, query, (c) => [c.label, `${c.section} ${c.detail ?? ""}`])) }];
  const sections = new Map<string, PaletteCommand[]>();
  for (const command of commands) sections.set(command.section, [...(sections.get(command.section) ?? []), command]);
  return [...sections].map(([heading, list]) => ({ heading, items: items(list) }));
}

/** ⌘K over a dimmed window (artboard 7): issues, filter lists and commands; `>`, `#` and `/` narrow it to one kind. */
export function CommandPalette() {
  const open = useStore((s) => s.paletteOpen);
  return open ? <Palette /> : null;
}

function Palette() {
  const close = useStore((s) => s.setPaletteOpen);
  const updatePlace = useStore((s) => s.updatePlace);
  const loadPlace = useStore((s) => s.loadPlace);
  const showIssue = useStore((s) => s.openIssue);
  const [text, setText] = useState("");
  const [active, setActive] = useState(0);
  // Before the palette is on screen, while the page still has the focus it had.
  const [page] = useState(snapshotPage);
  const { data: tree } = useSidebar();
  const { data: nav } = useNav();
  const client = useQueryClient();
  const go = useNavGo();
  const actions = useIssueActions();
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const { prefix, query } = parsePrefix(text);
  const mode = prefix?.mode ?? null;
  const settledText = useSettled(text);
  const settled = parsePrefix(settledText);
  const scope = settled.prefix && settled.prefix.mode !== "commands" ? SCOPES[settled.prefix.mode] : "all";
  const { data: results, isFetching } = useQuery({
    queryKey: ["palette", scope, settled.query],
    queryFn: () => api.paletteQuery(settled.query, scope),
    enabled: settled.prefix?.mode !== "commands" && (settled.query !== "" || scope === "lists"),
    placeholderData: keepPreviousData,
    gcTime: RESULTS_GC_MS,
  });
  const { data: menu } = useQuery({ queryKey: MENU_COMMANDS_KEY, queryFn: api.menuCommands });
  const commands = useMemo(() => collectCommands(menu ?? [], page), [menu, page]);

  const issues = (mode === null || mode === "issues") && query ? (results?.issues ?? []) : [];
  const lists = (mode === null || mode === "lists") && (query || mode === "lists") ? (results?.lists ?? []) : [];
  const views = (mode === null || mode === "lists") && query ? (results?.views ?? []) : [];
  // Issues come first, so a low index is an issue; elsewhere the first issue is meant.
  const activeIssue = issues[active] ?? issues[0];

  const own: PaletteCommand[] = [];
  const openInJira = actions.openInJira;
  if (activeIssue && openInJira) own.push({ id: "jira", label: `Open ${activeIssue.key} in Jira`, section: "Issue", shortcut: "⌘↩", run: () => openInJira(activeIssue.key) });
  if (query && (mode === null || mode === "issues")) {
    own.push({ id: "save", label: `Save “${query}” as a filter list`, section: "List", shortcut: "⌘S", run: () => void saveSearch(query, tree, nav, (id) => go(id)) });
  }

  const sections: Section[] = [];
  const add = (heading: string, items: Item[]) => {
    if (items.length > 0) sections.push({ heading, items });
  };
  if (mode === "commands") sections.push(...commandSections(commands, query));
  else if (mode === null && !query) add("Narrow to", PREFIXES.map((p): Item => ({ kind: "prefix", prefix: p })));
  else {
    add("Issues", issues.map((hit): Item => ({ kind: "issue", hit })));
    add(mode === "lists" ? "Lists" : "Filter lists", [...lists.map((hit): Item => ({ kind: "list", hit })), ...views.map((hit): Item => ({ kind: "view", hit }))]);
    const found = mode === null ? ranked(commands, query, (c) => [c.label, `${c.section} ${c.detail ?? ""}`]).slice(0, COMMAND_LIMIT) : [];
    add("Commands", [...own, ...found].map((command): Item => ({ kind: "command", command })));
  }
  const all = sections.flatMap((section) => section.items);
  const commandCount = all.filter((item) => item.kind === "command").length;

  useEffect(() => setActive(0), [text]);
  useEffect(() => {
    listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: "nearest" });
  }, [active]);

  const dismiss = () => close(false);
  const narrow = (next: string) => {
    setText(next);
    inputRef.current?.focus();
  };

  /** In the full Issue view, over its Space's last view (where the row stays selected). */
  const openIssue = async (hit: IssueHit) => {
    const space = tree?.spaces.find((s) => s.projectId === hit.projectId);
    dismiss();
    if (!space) return;
    await go(space.listId);
    const location = client.getQueryData<Nav>(NAV_KEY)?.location;
    if (!location) return;
    await loadPlace(location.listId, location.viewId);
    updatePlace(location.listId, location.viewId, { selectedId: hit.id, peekOpen: false });
    showIssue(hit.id, { listId: location.listId, viewId: location.viewId });
  };
  const run = async (item: Item | undefined) => {
    if (!item) return;
    if (item.kind === "prefix") return narrow(item.prefix.char);
    if (item.kind === "issue") return openIssue(item.hit);
    dismiss();
    if (item.kind === "list") return go(item.hit.id);
    if (item.kind === "view") return go(item.hit.list.id, item.hit.viewId);
    return item.command.run();
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
      if (activeIssue && openInJira) {
        dismiss();
        openInJira(activeIssue.key);
      }
    } else if (e.key === "Enter") {
      handled();
      void run(all[active]);
    } else if (mod && e.key.toLowerCase() === "c" && activeIssue && (window.getSelection()?.isCollapsed ?? true) && isCaretOnly(e.target)) {
      handled();
      actions.copyKey(activeIssue.key);
    } else if (mod && e.key.toLowerCase() === "s" && own.some((c) => c.id === "save")) {
      handled();
      dismiss();
      void saveSearch(query, tree, nav, (id) => go(id));
    } else if (mod && e.key.toLowerCase() === "r") {
      handled();
      dismiss();
      runBusCommand("sync:refresh", "Could not refresh");
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
  const row = (item: Item) => {
    switch (item.kind) {
      case "issue":
        return option(`issue:${item.hit.id}`, "palette__option palette__issue", <IssueRow hit={item.hit} />, item);
      case "list":
        return option(`list:${item.hit.id}`, "palette__option palette__list", <ListRow hit={item.hit} tree={tree} />, item);
      case "view":
        return option(`view:${item.hit.viewId}`, "palette__option palette__list", <ViewRow hit={item.hit} tree={tree} />, item);
      case "command":
        return option(`command:${item.command.id}`, "palette__option palette__action", <CommandRow command={item.command} showSection={mode !== "commands" || query !== ""} />, item);
      case "prefix":
        return option(`prefix:${item.prefix.char}`, "palette__option palette__action", <PrefixRow prefix={item.prefix} />, item);
    }
  };
  const counts = [plural(issues.length, "issue"), plural(lists.length + views.length, "list"), plural(commandCount, "command")].filter(Boolean).join(" · ");
  const placeholder = prefix?.placeholder ?? "Search issues, lists and commands";

  return (
    <div className="palette-scrim" onMouseDown={(e) => e.target === e.currentTarget && dismiss()}>
      <div className="palette" role="dialog" aria-label="Command palette" onKeyDown={onKeyDown}>
        <label className="palette__input">
          <Search size={18} strokeWidth={2} aria-hidden="true" />
          <input
            ref={inputRef}
            autoFocus
            role="combobox"
            aria-expanded="true"
            aria-controls="palette-results"
            aria-label={placeholder}
            placeholder={placeholder}
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
          <span className="palette__count">{counts}</span>
        </label>
        <div ref={listRef} id="palette-results" className="palette__results" role="listbox" aria-label="Results">
          {sections.map((section, i) => (
            <section key={section.heading} aria-label={section.heading}>
              {i > 0 && <hr className="palette__separator" />}
              <h2 className="palette__caps">{section.heading}</h2>
              {section.items.map(row)}
            </section>
          ))}
          {all.length === 0 && query !== "" && !isFetching && settledText === text && <p className="palette__empty">No results</p>}
        </div>
        <footer className="palette__footer">
          <PrefixButtons active={prefix} onPick={narrow} />
          <span className="palette__spacer" />
          {activeIssue ? (
            <>
              <span>⌘↩ open in Jira</span>
              <span>⌘C copy key</span>
            </>
          ) : (
            <span>↩ open</span>
          )}
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
