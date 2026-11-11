import type { Person } from "../api";

/** Longer than any name: past it, the `@` was no mention. */
const MAX_QUERY = 40;

export interface MentionQuery {
  /** Index of the `@`. */
  start: number;
  query: string;
}

/** The `@name` being typed before the caret, if its `@` starts a word. */
export function mentionAt(text: string, caret: number): MentionQuery | null {
  const start = text.lastIndexOf("@", caret - 1);
  if (start < 0) return null;
  const query = text.slice(start + 1, caret);
  if (query.length > MAX_QUERY || query.includes("\n") || query.startsWith(" ")) return null;
  const before = text[start - 1];
  if (before !== undefined && !/[\s([]/.test(before)) return null;
  return { start, query };
}

/** People whose name, or a word of it, starts with the query. */
export function matching(people: Person[], query: string): Person[] {
  const q = query.toLowerCase();
  return people.filter((person) => {
    const name = person.name.toLowerCase();
    return name.startsWith(q) || name.split(/\s+/).some((word) => word.startsWith(q));
  });
}

/** The text with the query replaced by `@Name `, and where the caret goes. */
export function insertMention(text: string, caret: number, { start }: MentionQuery, person: Person) {
  const inserted = `@${person.name} `;
  return { text: text.slice(0, start) + inserted + text.slice(caret), caret: start + inserted.length };
}
