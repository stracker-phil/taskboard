import { describe, expect, it } from "vitest";
import type { Person } from "../api";
import { insertMention, matching, mentionAt } from "./mentions";

const person = (name: string): Person => ({ accountId: name.toLowerCase().replace(/\s+/g, "-"), name, initials: name.slice(0, 2), color: 1 });
const marta = person("Marta Klein");
const jonas = person("Jonas Reiter");

describe("mentionAt", () => {
  it.each([
    ["at the start of the text", "@ma", 3, { start: 0, query: "ma" }],
    ["after a space", "Thanks @ma", 10, { start: 7, query: "ma" }],
    ["after an opening parenthesis", "see (@ma", 8, { start: 5, query: "ma" }],
    ["with nothing typed yet", "hi @", 4, { start: 3, query: "" }],
    ["with a space inside the query", "@Marta K", 8, { start: 0, query: "Marta K" }],
    ["at the longest query", `@${"a".repeat(40)}`, 41, { start: 0, query: "a".repeat(40) }],
  ])("opens for an @ %s", (_, text, caret, expected) => {
    expect(mentionAt(text, caret)).toEqual(expected);
  });

  it.each([
    ["after a letter, as in an address", "a@b", 3],
    ["after an earlier @ without a space", "@a@b", 4],
    ["with a line break in the query", "@a\nb", 4],
    ["with a space right after the @", "@ ma", 4],
    ["with a query over 40 characters", `@${"a".repeat(41)}`, 42],
    ["with the caret before the @", "hi @ma", 2],
    ["without any @", "hello", 5],
  ])("stays closed for an @ %s", (_, text, caret) => {
    expect(mentionAt(text, caret)).toBeNull();
  });
});

describe("matching", () => {
  const people = [marta, jonas];

  it("matches the start of the full name", () => {
    expect(matching(people, "mar")).toEqual([marta]);
  });

  it("matches the start of any word of the name", () => {
    expect(matching(people, "kl")).toEqual([marta]);
    expect(matching(people, "rei")).toEqual([jonas]);
  });

  it("matches a query that spans words from the start of the name", () => {
    expect(matching(people, "marta k")).toEqual([marta]);
  });

  it("ignores case", () => {
    expect(matching(people, "JON")).toEqual([jonas]);
  });

  it("lists everyone for an empty query", () => {
    expect(matching(people, "")).toEqual(people);
  });

  it("finds nobody when no name starts with the query", () => {
    expect(matching(people, "lein")).toEqual([]);
  });
});

describe("insertMention", () => {
  it("replaces the typed @query with the full name and a trailing space", () => {
    const result = insertMention("Thanks @ma", 10, { start: 7, query: "ma" }, marta);

    expect(result).toEqual({ text: "Thanks @Marta Klein ", caret: 20 });
  });

  it("keeps the text after the caret and puts the caret after the inserted space", () => {
    const result = insertMention("Thanks @ma for this", 10, { start: 7, query: "ma" }, marta);

    expect(result.text).toBe("Thanks @Marta Klein  for this");
    expect(result.caret).toBe("Thanks @Marta Klein ".length);
  });
});
