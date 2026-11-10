/// <reference types="node" />
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { contrastRatio, hexToRgb, over, type Rgb } from "./contrast";

const SCHEMES = ["light", "dark"] as const;

const sheets = Object.fromEntries(
  SCHEMES.map((scheme) => [scheme, readFileSync(join(resolve(process.cwd(), "src"), "styles", "palettes", `${scheme}.css`), "utf8")]),
);

describe("contrastRatio", () => {
  it("is 21 for black on white and 1 for identical colors", () => {
    expect(contrastRatio(hexToRgb("#000000"), hexToRgb("#ffffff"))).toBeCloseTo(21, 5);
    expect(contrastRatio(hexToRgb("#777777"), hexToRgb("#777777"))).toBe(1);
  });
});

const ACCENTS = ["blue", "purple", "pink", "red", "orange", "yellow", "green", "teal", "graphite"] as const;

/** Splits `a, b(c, d), e` at the commas outside parentheses. */
function splitArgs(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    if (text[i] === "(") depth++;
    else if (text[i] === ")") depth--;
    else if (text[i] === "," && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts.map((p) => p.trim());
}

/**
 * Resolves a palette's tokens to colors: hex, rgba(), var(--x) and color-mix(in srgb, A P%, B). Mixing with
 * `transparent` composites A at P% over the background. `overrides` replace tokens, like a `data-accent` rule.
 */
function paletteColors(css: string, overrides: Record<string, string> = {}) {
  function token(name: string): string {
    if (name in overrides) return overrides[name];
    const m = css.match(new RegExp(`^\\s*${name}:\\s*([^;]+);`, "m"));
    if (!m) throw new Error(`missing token ${name}`);
    return m[1].trim();
  }

  function resolve(value: string, bg?: Rgb): Rgb {
    if (value.startsWith("#")) return hexToRgb(value);
    const variable = value.match(/^var\((--[\w-]+)\)$/);
    if (variable) return resolve(token(variable[1]), bg);
    const rgba = value.match(/^rgba\(\s*(\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)$/);
    if (rgba && bg) return over([+rgba[1] / 255, +rgba[2] / 255, +rgba[3] / 255], +rgba[4], bg);
    const mix = value.match(/^color-mix\(in srgb,(.*)\)$/);
    if (mix) {
      const [first, second] = splitArgs(mix[1]);
      const share = first.match(/^(.+?)\s+([\d.]+)%$/);
      if (share && second) {
        const alpha = +share[2] / 100;
        if (second === "transparent") {
          if (!bg) throw new Error(`cannot resolve ${value} without a background`);
          return over(resolve(share[1], bg), alpha, bg);
        }
        return over(resolve(share[1], bg), alpha, resolve(second, bg));
      }
    }
    throw new Error(`cannot resolve ${value}`);
  }

  const color = (name: string, bg?: Rgb): Rgb => resolve(token(name), bg);

  return (fg: string, bg: string) => {
    const background = color(bg);
    return contrastRatio(color(fg, background), background);
  };
}

describe.each(SCHEMES)("%s palette contrast", (scheme) => {
  const css = sheets[scheme];
  const ratio = paletteColors(css);

  describe.each(ACCENTS)("with the %s accent", (accent) => {
    const accentRatio = paletteColors(css, {
      "--accent": `var(--accent-${accent})`,
      "--accent-text": `var(--accent-${accent}-text)`,
    });

    it.each(["--link", "--accent-text"])("%s reads at 4.5:1 on the panel", (fg) => {
      expect(accentRatio(fg, "--panel-bg")).toBeGreaterThanOrEqual(4.5);
    });

    it("selected text reads at 4.5:1 on the selection", () => {
      expect(accentRatio("--selection-text", "--selection-bg")).toBeGreaterThanOrEqual(4.5);
    });

    it("--text-on-accent reads at 4.5:1 on --accent", () => {
      expect(accentRatio("--text-on-accent", "--accent")).toBeGreaterThanOrEqual(4.5);
    });
  });

  it.each(["--text", "--text-strong-body", "--text-secondary", "--text-tertiary", "--text-muted"])(
    "%s reads at 4.5:1 on the panel",
    (fg) => {
      expect(ratio(fg, "--panel-bg")).toBeGreaterThanOrEqual(4.5);
    },
  );

  it.each(["--text-strong-body", "--text-secondary", "--text-tertiary"])("%s reads at 4.5:1 on the sidebar", (fg) => {
    expect(ratio(fg, "--window-bg")).toBeGreaterThanOrEqual(4.5);
  });

  it("muted meta text reads at 3:1 on the sidebar", () => {
    expect(ratio("--text-muted", "--window-bg")).toBeGreaterThanOrEqual(3);
  });

  it.each(["todo", "progress", "review", "blocked", "done"])("the %s status pill reads at 4.5:1", (s) => {
    expect(ratio(`--status-${s}-fg`, `--status-${s}-bg`)).toBeGreaterThanOrEqual(4.5);
  });

  it.each([1, 2, 3, 4, 5])("epic chip %i reads at 4.5:1", (n) => {
    expect(ratio(`--epic-${n}-fg`, `--epic-${n}-bg`)).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["1", "2", "neutral"])("label chip %s reads at 4.5:1", (n) => {
    expect(ratio(`--label-${n}-fg`, `--label-${n}-bg`)).toBeGreaterThanOrEqual(4.5);
  });

  it.each([
    ["--banner-info-text", "--banner-info-bg"],
    ["--banner-warn-text", "--banner-warn-bg"],
    ["--banner-error-text", "--banner-error-bg"],
    ["--button-primary-text", "--button-primary-bg"],
    ["--settings-text", "--settings-bg"],
    ["--settings-text-secondary", "--settings-bg"],
  ])("%s reads at 4.5:1 on %s", (fg, bg) => {
    expect(ratio(fg, bg)).toBeGreaterThanOrEqual(4.5);
  });

  it("tertiary settings text reads at 3:1 on the settings page", () => {
    expect(ratio("--settings-text-tertiary", "--settings-bg")).toBeGreaterThanOrEqual(3);
  });
});
