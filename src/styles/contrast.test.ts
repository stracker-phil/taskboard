/// <reference types="node" />
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { contrastRatio, hexToRgb, over, type Rgb } from "./contrast";

const css = readFileSync(join(resolve(process.cwd(), "src"), "styles", "palettes", "light.css"), "utf8");

function token(name: string): string {
  const m = css.match(new RegExp(`^\\s*${name}:\\s*([^;]+);`, "m"));
  if (!m) throw new Error(`missing token ${name}`);
  return m[1].trim();
}

/** A token's color; an rgba() value is composited over `bg`. */
function color(name: string, bg?: Rgb): Rgb {
  const value = token(name);
  if (value.startsWith("#")) return hexToRgb(value);
  const m = value.match(/^rgba\(\s*(\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)$/);
  if (!m || !bg) throw new Error(`cannot resolve ${name}: ${value}`);
  return over([+m[1] / 255, +m[2] / 255, +m[3] / 255], +m[4], bg);
}

const ratio = (fg: string, bg: string) => {
  const background = color(bg);
  return contrastRatio(color(fg, background), background);
};

describe("contrastRatio", () => {
  it("is 21 for black on white and 1 for identical colors", () => {
    expect(contrastRatio(hexToRgb("#000000"), hexToRgb("#ffffff"))).toBeCloseTo(21, 5);
    expect(contrastRatio(hexToRgb("#777777"), hexToRgb("#777777"))).toBe(1);
  });
});

describe("light palette contrast", () => {
  it.each(["--text", "--text-strong-body", "--text-secondary", "--text-tertiary", "--text-muted", "--link", "--accent-text"])(
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

  it("selected text reads at 4.5:1 on the selection", () => {
    expect(ratio("--selection-text", "--selection-bg")).toBeGreaterThanOrEqual(4.5);
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
    ["--changed-chip-text", "--changed-chip-bg"],
    ["--banner-info-text", "--banner-info-bg"],
    ["--banner-warn-text", "--banner-warn-bg"],
    ["--banner-error-text", "--banner-error-bg"],
    ["--text-on-accent", "--accent"],
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
