import { Users } from "lucide-react";
import type { Person } from "../../api";
import { Avatar } from "../../components/issue/Avatar";
import { openMenu, type MenuAt, type MenuEntry } from "../../components/menu";
import { useRequestNarrowing, useViewPeople } from "../../queries/views";

interface Props {
  listId: number;
  viewId: number;
  /** The account the view is narrowed to; null for anyone. */
  person: string | null;
  onChange: (person: string | null) => void;
}

/** Narrows the view to one person's issues, on top of its filter: a draft until Save View, like the Me chip. */
export function PersonChip({ listId, viewId, person, onChange }: Props) {
  const { data: people = [] } = useViewPeople(listId, viewId, useRequestNarrowing(listId, viewId));
  const picked = people.find((p) => p.accountId === person) ?? null;
  return (
    <button
      type="button"
      className={picked ? "me-chip person-chip" : "me-chip person-chip me-chip--plain"}
      aria-pressed={picked !== null}
      aria-haspopup="menu"
      title={picked ? `Only issues assigned to ${picked.name}` : "Show one person's issues"}
      onClick={(e) => openPeopleMenu(people, person, onChange, e.currentTarget)}
    >
      {picked ? <Avatar person={picked} size={22} /> : <Users size={14} strokeWidth={2} aria-hidden="true" />}
      {picked ? firstName(picked.name) : "Person"}
    </button>
  );
}

/** The person chip's menu; also the "…" menu's entry when the chip has folded away. */
export function openPeopleMenu(people: Person[], person: string | null, onChange: (person: string | null) => void, at: MenuAt) {
  const entries: MenuEntry[] = [
    { caption: "Issues assigned to" },
    { text: "Anyone", checked: person === null, run: () => onChange(null) },
    "separator",
    ...(people.length > 0
      ? people.map((p) => ({ text: p.name, icon: <Avatar person={p} size={18} />, checked: p.accountId === person, run: () => onChange(p.accountId) }))
      : [{ text: "No teammates yet", enabled: false }]),
  ];
  openMenu(entries, at, "Person");
}

function firstName(name: string): string {
  return name.trim().split(/\s+/)[0] ?? name;
}
