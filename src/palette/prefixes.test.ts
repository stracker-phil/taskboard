import { describe, expect, it } from "vitest";
import { parsePrefix, PREFIXES, SCOPES } from "./prefixes";

describe("parsePrefix", () => {
  it.each([
    [">", "commands"],
    ["#", "issues"],
    ["/", "lists"],
  ])("reads %s as the %s prefix", (char, mode) => {
    expect(parsePrefix(`${char}sprint`).prefix?.mode).toBe(mode);
  });

  it("returns the text after the prefix, trimmed", () => {
    expect(parsePrefix(">  toggle sidebar  ").query).toBe("toggle sidebar");
  });

  it("returns an empty query when only the prefix is typed", () => {
    expect(parsePrefix("#")).toMatchObject({ prefix: PREFIXES[1], query: "" });
  });

  it("has no prefix for plain text, which is trimmed", () => {
    expect(parsePrefix("  contact form ")).toEqual({ prefix: null, query: "contact form" });
  });

  it("only reads a prefix as the first character", () => {
    expect(parsePrefix("fix #12 > now")).toEqual({ prefix: null, query: "fix #12 > now" });
  });

  it("does not read a prefix after leading whitespace", () => {
    expect(parsePrefix(" >x").prefix).toBeNull();
  });
});

describe("SCOPES", () => {
  it("searches issues and lists in Rust by their own scope", () => {
    expect(SCOPES).toEqual({ issues: "issues", lists: "lists" });
  });
});
