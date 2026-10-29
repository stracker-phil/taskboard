import { describe, expect, it } from "vitest";
import { place } from "./placement";

const view = { width: 800, height: 600 };
const size = { width: 200, height: 100 };

describe("place", () => {
  describe("at a point", () => {
    it("opens at the point when the menu fits", () => {
      expect(place({ point: { x: 100, y: 120 } }, size, view)).toEqual({ left: 100, top: 120 });
    });

    it("flips to the left of the point when it overflows the right edge", () => {
      expect(place({ point: { x: 700, y: 120 } }, size, view)).toEqual({ left: 500, top: 120 });
    });

    it("flips above the point when it overflows the bottom edge", () => {
      expect(place({ point: { x: 100, y: 550 } }, size, view)).toEqual({ left: 100, top: 450 });
    });
  });

  describe("below an anchor", () => {
    const box = { left: 100, top: 40, right: 160, bottom: 64 };

    it("left-aligns under the box with a gap", () => {
      expect(place({ below: box }, size, view)).toEqual({ left: 100, top: 68 });
    });

    it("flips above the box when there is no room below", () => {
      const low = { left: 100, top: 560, right: 160, bottom: 584 };

      expect(place({ below: low }, size, view)).toEqual({ left: 100, top: 456 });
    });

    it("right-aligns with the box when it overflows the right edge", () => {
      const right = { left: 740, top: 40, right: 790, bottom: 64 };

      expect(place({ below: right }, size, view)).toEqual({ left: 590, top: 68 });
    });
  });

  describe("beside a row", () => {
    const row = { left: 100, top: 200, right: 300, bottom: 224 };

    it("opens to the right of the row, lined up with its first row", () => {
      expect(place({ beside: row }, size, view)).toEqual({ left: 300, top: 193 });
    });

    it("flips to the left of the row when it overflows the right edge", () => {
      const far = { left: 500, top: 200, right: 700, bottom: 224 };

      expect(place({ beside: far }, size, view)).toEqual({ left: 300, top: 193 });
    });
  });

  describe("clamping", () => {
    it("keeps the menu at least 6px inside the top-left corner", () => {
      expect(place({ below: { left: -50, top: -80, right: -10, bottom: -60 } }, size, view)).toEqual({ left: 6, top: 6 });
    });

    it("keeps a menu larger than its room 6px inside the far edges", () => {
      const big = { width: 790, height: 590 };

      expect(place({ point: { x: 0, y: 0 } }, big, view)).toEqual({ left: 6, top: 6 });
    });

    it("pulls a flipped menu back inside when the point is at the far corner", () => {
      expect(place({ point: { x: 799, y: 599 } }, size, view)).toEqual({ left: 594, top: 494 });
    });
  });
});
