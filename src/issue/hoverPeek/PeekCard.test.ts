import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MAX_HEIGHT, place, WIDTH } from "./PeekCard";

const rectOf = (left: number, right: number, top = 100): DOMRect => ({ left, right, top, bottom: top + 200, width: right - left, height: 200, x: left, y: top }) as DOMRect;

const originalSize = { width: window.innerWidth, height: window.innerHeight };

beforeEach(() => {
  window.innerWidth = 1200;
  window.innerHeight = 800;
});

afterEach(() => {
  window.innerWidth = originalSize.width;
  window.innerHeight = originalSize.height;
});

describe("place", () => {
  describe("preferring the left", () => {
    it("sits left of the rect, with a gap, when there is room", () => {
      const style = place(rectOf(600, 900), "left");

      expect(style.left).toBe(600 - 10 - WIDTH);
      expect(style.width).toBe(WIDTH);
      expect(style.maxHeight).toBe(MAX_HEIGHT);
    });

    it("sits right of the rect when the left has no room but the right fits", () => {
      const style = place(rectOf(200, 500), "left");

      expect(style.left).toBe(500 + 10);
    });

    it("clamps to the left margin when neither side has room", () => {
      window.innerWidth = 600;

      const style = place(rectOf(100, 400), "left");

      expect(style.left).toBe(8);
    });
  });

  describe("preferring the right (the default)", () => {
    it("sits right of the rect when there is room, even if the left has room too", () => {
      expect(place(rectOf(500, 700)).left).toBe(700 + 10);
      expect(place(rectOf(500, 700), "right").left).toBe(700 + 10);
    });

    it("falls back to the left of the rect when the right has no room", () => {
      expect(place(rectOf(600, 900)).left).toBe(600 - 10 - WIDTH);
    });

    it("clamps to the left margin when neither side has room", () => {
      window.innerWidth = 600;

      expect(place(rectOf(100, 400)).left).toBe(8);
    });
  });

  describe("vertically", () => {
    it.each([
      ["lines up with the top of the rect", 100, 100],
      ["keeps a margin from the top of the window", 0, 8],
      ["stays above the bottom of the window", 790, 800 - MAX_HEIGHT - 8],
    ])("%s", (_name, top, expected) => {
      expect(place(rectOf(600, 900, top), "left").top).toBe(expected);
    });
  });
});
