import { describe, expect, it } from "vitest";
import { ranked, rankMatch } from "./rank";

describe("rankMatch", () => {
  it.each<[string, string, string, number | null]>([
    ["Toggle Sidebar", "", "tog", 0],
    ["Toggle Sidebar", "", "side", 1],
    ["Group: Sort by › Updated", "", "upd", 1],
    ["Toggle Sidebar", "View", "view sidebar", 2],
    ["Toggle Sidebar", "", "tsb", 3],
    ["Toggle Sidebar", "", "xyz", null],
  ])("ranks %s (detail “%s”) against “%s” as %s", (label, detail, query, expected) => {
    expect(rankMatch(label, detail, query)).toBe(expected);
  });

  it("matches everything for an empty query", () => {
    expect(rankMatch("Anything", "", "")).toBe(0);
  });

  it("ignores case and accents in the label", () => {
    expect(rankMatch("Café öffnen", "", "cafe")).toBe(0);
  });

  it("finds a word that only the detail has", () => {
    expect(rankMatch("Settings: Connection", "Jira site, Token", "token")).toBe(2);
  });

  it("ignores spaces when matching scattered letters", () => {
    expect(rankMatch("Toggle Sidebar", "", "t sb")).toBe(3);
  });
});

describe("ranked", () => {
  const text = (label: string): [string, string] => [label, ""];

  it("lists the best match first and drops non-matches", () => {
    const items = ["Open sidebar menu", "Sidebar", "Reload", "Side by side"];

    expect(ranked(items, "side", text)).toEqual(["Sidebar", "Side by side", "Open sidebar menu"]);
  });

  it("keeps the original order among equal ranks", () => {
    expect(ranked(["Beta view", "Alpha view", "Gamma view"], "view", text)).toEqual(["Beta view", "Alpha view", "Gamma view"]);
  });

  it("folds and trims the query", () => {
    expect(ranked(["Éclair"], "  ECLAIR ", text)).toEqual(["Éclair"]);
  });

  it("returns every item for an empty query", () => {
    expect(ranked(["a", "b"], "", text)).toEqual(["a", "b"]);
  });
});
