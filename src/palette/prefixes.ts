import type { PaletteScope } from "../api";

/** What a prefix narrows the palette to. */
export type PaletteMode = "commands" | "issues" | "lists";

export interface Prefix {
  char: string;
  mode: PaletteMode;
  label: string;
  placeholder: string;
}

/** Typed first, each limits the palette to one kind; the footer offers them as buttons. */
export const PREFIXES: Prefix[] = [
  { char: ">", mode: "commands", label: "Commands", placeholder: "Run a menu command or a view option" },
  { char: "#", mode: "issues", label: "Issues", placeholder: "Find an issue by key or text" },
  { char: "/", mode: "lists", label: "Lists", placeholder: "Go to a Space or a filter list" },
];

/** The prefix in the text's first character, if any, and the query after it. */
export function parsePrefix(text: string): { prefix: Prefix | null; query: string } {
  const prefix = PREFIXES.find((p) => text.startsWith(p.char)) ?? null;
  return { prefix, query: (prefix ? text.slice(prefix.char.length) : text).trim() };
}

/** What Rust searches for a prefix; commands never ask it. */
export const SCOPES: Record<Exclude<PaletteMode, "commands">, PaletteScope> = { issues: "issues", lists: "lists" };
