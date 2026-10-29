import { Check, ChevronRight } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import type { MenuEntry, MenuItem } from "../../state/slices/menu";
import { place, type Placement } from "./placement";

interface Props {
  entries: MenuEntry[];
  label: string;
  placement: Placement;
  /** Opened from the keyboard: the first row starts highlighted. */
  highlightFirst: boolean;
  /** Closes every panel, then runs the row. */
  onChoose: (item: MenuItem) => void;
  /** A submenu's ←: back to its parent row. */
  onBack?: () => void;
}

interface Section {
  caption?: string;
  /** Each entry with its index in `entries`. */
  entries: [MenuEntry, number][];
}

const isItem = (entry: MenuEntry): entry is MenuItem => entry !== "separator" && !("caption" in entry);
const isEnabled = (entry: MenuEntry | undefined) => !!entry && isItem(entry) && entry.enabled !== false;

/** A caption starts a captioned section, which runs to the next caption or separator. */
function sectionsOf(entries: MenuEntry[]): Section[] {
  const sections: Section[] = [{ entries: [] }];
  entries.forEach((entry, index) => {
    const last = sections[sections.length - 1];
    if (entry !== "separator" && "caption" in entry) sections.push({ caption: entry.caption, entries: [] });
    else if (entry === "separator" && last.caption !== undefined) sections.push({ entries: [[entry, index]] });
    else last.entries.push([entry, index]);
  });
  return sections.filter((section) => section.caption !== undefined || section.entries.length > 0);
}

/** One menu panel: its rows, the keys that move through them, and the submenu of the highlighted row. */
export function MenuPanel({ entries, label, placement, highlightFirst, onChoose, onBack }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const enabled = entries.flatMap((entry, index) => (isEnabled(entry) ? [index] : []));
  const [active, setActive] = useState(highlightFirst ? (enabled[0] ?? -1) : -1);
  const [submenu, setSubmenu] = useState<{ index: number; placement: Placement; focused: boolean } | null>(null);
  const [position, setPosition] = useState<CSSProperties>({ left: 0, top: 0, visibility: "hidden" });

  useLayoutEffect(() => {
    const panel = ref.current;
    if (!panel) return;
    const size = { width: panel.offsetWidth, height: panel.offsetHeight };
    setPosition(place(placement, size, { width: window.innerWidth, height: window.innerHeight }));
  }, [placement]);

  // The focus follows the highlight, unless a submenu opened from the keyboard holds it.
  useEffect(() => {
    if (submenu?.focused) return;
    const row = active >= 0 ? ref.current?.querySelector<HTMLElement>(`[data-index="${active}"]`) : null;
    (row ?? ref.current)?.focus({ preventScroll: true });
  }, [active, submenu]);

  const rowOf = (index: number) => ref.current?.querySelector<HTMLElement>(`[data-index="${index}"]`);
  const openSubmenu = (index: number, focused: boolean) => {
    const row = rowOf(index);
    if (!row) return;
    const { left, top, right, bottom } = row.getBoundingClientRect();
    setSubmenu({ index, placement: { beside: { left, top, right, bottom } }, focused });
  };

  const choose = (index: number) => {
    const entry = entries[index];
    if (!isEnabled(entry) || !isItem(entry)) return;
    if (entry.items) openSubmenu(index, true);
    else onChoose(entry);
  };

  const hover = (index: number) => {
    if (!isEnabled(entries[index])) return;
    setActive(index);
    const entry = entries[index];
    if (isItem(entry) && entry.items) openSubmenu(index, false);
    else setSubmenu(null);
  };

  const step = (by: 1 | -1) => {
    if (enabled.length === 0) return;
    const at = enabled.indexOf(active);
    const next = at === -1 ? (by === 1 ? 0 : enabled.length - 1) : (at + by + enabled.length) % enabled.length;
    setActive(enabled[next]);
  };

  const jumpTo = (letter: string) => {
    const from = enabled.indexOf(active);
    const order = [...enabled.slice(from + 1), ...enabled.slice(0, from + 1)];
    const match = order.find((index) => (entries[index] as MenuItem).text.toLowerCase().startsWith(letter.toLowerCase()));
    if (match !== undefined) setActive(match);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const entry = entries[active];
    switch (e.key) {
      case "ArrowDown":
        step(1);
        break;
      case "ArrowUp":
        step(-1);
        break;
      case "Home":
        if (enabled.length) setActive(enabled[0]);
        break;
      case "End":
        if (enabled.length) setActive(enabled[enabled.length - 1]);
        break;
      case "Enter":
      case " ":
        if (active >= 0) choose(active);
        break;
      case "ArrowRight":
        if (entry && isItem(entry) && entry.items) openSubmenu(active, true);
        break;
      case "ArrowLeft":
        onBack?.();
        break;
      case "Tab":
        break;
      default:
        if (e.key.length !== 1 || e.metaKey || e.ctrlKey || e.altKey) return;
        jumpTo(e.key);
    }
    e.preventDefault();
    e.stopPropagation();
  };

  const subEntry = submenu ? entries[submenu.index] : undefined;
  return (
    <>
      <div
        ref={ref}
        className="popover menu"
        role="menu"
        aria-label={label}
        tabIndex={-1}
        style={position}
        onKeyDown={onKeyDown}
        onMouseLeave={() => {
          if (!submenu) setActive(-1);
        }}
      >
        {sectionsOf(entries).map((section, at) => (
          <MenuSection key={at} section={section} active={active} openIndex={submenu?.index ?? null} onHover={hover} onChoose={choose} />
        ))}
      </div>
      {submenu && subEntry && isItem(subEntry) && subEntry.items && (
        <MenuPanel
          key={submenu.index}
          entries={subEntry.items}
          label={subEntry.text}
          placement={submenu.placement}
          highlightFirst={submenu.focused}
          onChoose={onChoose}
          onBack={() => setSubmenu(null)}
        />
      )}
    </>
  );
}

interface SectionProps {
  section: Section;
  active: number;
  openIndex: number | null;
  onHover: (index: number) => void;
  onChoose: (index: number) => void;
}

function MenuSection({ section, active, openIndex, onHover, onChoose }: SectionProps) {
  const rows = section.entries.map(([entry, index]) =>
    isItem(entry) ? (
      <MenuRow key={index} item={entry} index={index} active={index === active} open={index === openIndex} onHover={onHover} onChoose={onChoose} />
    ) : (
      <div key={index} className="menu__separator" role="separator" />
    ),
  );
  if (section.caption === undefined) return <>{rows}</>;
  return (
    <div className="menu__group" role="group" aria-label={section.caption}>
      <span className="popover__caps" aria-hidden="true">
        {section.caption}
      </span>
      {rows}
    </div>
  );
}

interface RowProps {
  item: MenuItem;
  index: number;
  active: boolean;
  open: boolean;
  onHover: (index: number) => void;
  onChoose: (index: number) => void;
}

function MenuRow({ item, index, active, open, onHover, onChoose }: RowProps) {
  const choice = item.checked !== undefined;
  const role = !choice ? "menuitem" : item.toggle ? "menuitemcheckbox" : "menuitemradio";
  const disabled = item.enabled === false;
  return (
    <button
      type="button"
      role={role}
      tabIndex={-1}
      data-index={index}
      className={item.danger ? "menu__item menu__item--danger" : "menu__item"}
      data-active={active || undefined}
      aria-checked={choice ? item.checked : undefined}
      aria-disabled={disabled || undefined}
      aria-haspopup={item.items ? "menu" : undefined}
      aria-expanded={item.items ? open : undefined}
      onMouseEnter={() => onHover(index)}
      onClick={() => onChoose(index)}
    >
      {item.icon && <span className="menu__icon">{item.icon}</span>}
      <span className="menu__text">{item.text}</span>
      {item.hint && <span className="menu__hint">{item.hint}</span>}
      {item.checked && <Check className="menu__mark" size={14} strokeWidth={2.2} aria-hidden="true" />}
      {item.items && <ChevronRight className="menu__mark" size={14} strokeWidth={2} aria-hidden="true" />}
    </button>
  );
}
