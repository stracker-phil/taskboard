/// <reference types="node" />
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { describe, expect, it } from "vitest";

// Read from disk: Vitest blanks every CSS module (also `?raw` ones) unless CSS processing is on.
const srcDir = resolve(process.cwd(), "src");
const palettesDir = join(srcDir, "styles", "palettes") + sep;

const all: [string, string][] = readdirSync(srcDir, { recursive: true, encoding: "utf8" })
  .filter((rel: string) => /\.(css|ts|tsx)$/.test(rel))
  .map((rel: string): [string, string] => [join(srcDir, rel), readFileSync(join(srcDir, rel), "utf8")]);

const paletteFiles = all.filter(([path]) => path.startsWith(palettesDir) && path.endsWith(".css"));
const scanned = all.filter(([path]) => !path.startsWith(palettesDir) && !/\.test\.tsx?$/.test(path) && !path.endsWith("types.gen.ts"));

const stripComments = (text: string) => text.replace(/\/\*[\s\S]*?\*\//g, "");

// CSS: any hex color. TS(X): only a hex inside a string literal, so "#3229" in prose or `#` in regexes pass.
const CSS_HEX = /#[0-9a-fA-F]{3,8}\b/g;
const TS_HEX = /["'`]#[0-9a-fA-F]{3,8}["'`]/g;
const FUNCTIONAL = /\b(?:rgba?|hsla?)\(/g;

function colorLiterals(path: string, source: string): string[] {
  const isCss = path.endsWith(".css");
  const text = isCss ? stripComments(source) : source;
  return [isCss ? CSS_HEX : TS_HEX, FUNCTIONAL].flatMap((re) => text.match(re) ?? []);
}

function customProperties(source: string): string[] {
  return [...stripComments(source).matchAll(/^\s*(--[\w-]+)\s*:/gm)].map((m) => m[1]).sort();
}

describe("color literals", () => {
  it("live only in the palette files", () => {
    const offenders = scanned
      .map(([path, source]) => ({ path, found: colorLiterals(path, source) }))
      .filter((o) => o.found.length > 0)
      .map((o) => `${o.path}: ${o.found.join(", ")}`);
    expect(offenders).toEqual([]);
  });

  it("scan sees the stylesheets and flags a literal", () => {
    expect(scanned.filter(([path]) => path.endsWith(".css")).length).toBeGreaterThan(1);
    expect(scanned.length).toBeGreaterThan(10);
    expect(colorLiterals("a.css", "color: #fa8d3e;")).toHaveLength(1);
    expect(colorLiterals("a.ts", 'const c = "#fa8d3e";')).toHaveLength(1);
    expect(colorLiterals("a.css", "box-shadow: 0 0 4px rgba(0, 0, 0, 0.2)")).toHaveLength(1);
    expect(colorLiterals("a.css", "background: transparent; color: currentColor")).toEqual([]);
    expect(colorLiterals("a.ts", 'label: "Order #4711"')).toEqual([]);
  });
});

describe("palette files", () => {
  it("include the light palette", () => {
    const names = paletteFiles.map(([path]) => path.slice(palettesDir.length));
    expect(names).toContain("light.css");
  });

  it("all define the same custom properties", () => {
    const light = paletteFiles.find(([path]) => path.endsWith(`${sep}light.css`));
    const expected = customProperties(light![1]);
    expect(expected.length).toBeGreaterThan(0);
    for (const [path, source] of paletteFiles) {
      expect({ path, tokens: customProperties(source) }).toEqual({ path, tokens: expected });
    }
  });
});
