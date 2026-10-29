import { render } from "@testing-library/react";
import { Star } from "lucide-react";
import { describe, expect, it } from "vitest";
import { Tile } from "./Tile";

function renderTile(props: Partial<Parameters<typeof Tile>[0]> = {}) {
  const { container } = render(<Tile icon={null} color={null} fallback="W" size={32} {...props} />);
  return container;
}

describe("Tile", () => {
  describe("with a color", () => {
    it("shows a letter in capitals", () => {
      expect(renderTile({ icon: "w", color: "3" })).toHaveTextContent(/^W$/);
    });

    it("keeps an emoji as it is", () => {
      expect(renderTile({ icon: "🚀", color: "3" })).toHaveTextContent(/^🚀$/);
    });

    it("draws a line icon as an svg instead of text", () => {
      const tile = renderTile({ icon: "rocket", color: "3" });

      expect(tile.querySelector("svg")).toBeInTheDocument();
      expect(tile).not.toHaveTextContent(/\S/);
    });

    it("falls back to the given letter, in capitals, without an icon", () => {
      expect(renderTile({ color: "3", fallback: "w" })).toHaveTextContent(/^W$/);
    });

    it("falls back to the given line icon without an icon", () => {
      const tile = renderTile({ color: "3", fallback: Star });

      expect(tile.querySelector("svg")).toBeInTheDocument();
    });

    it("treats an unknown icon name as text", () => {
      expect(renderTile({ icon: "q", color: "3", fallback: Star })).toHaveTextContent(/^Q$/);
    });

    it("draws the line icon smaller than the tile", () => {
      expect(renderTile({ icon: "rocket", color: "3", size: 32 }).querySelector("svg")).toHaveAttribute("width", "20");
    });
  });

  describe("without a color", () => {
    it("shows the icon alone at the full size", () => {
      expect(renderTile({ icon: "rocket", size: 16 }).querySelector("svg")).toHaveAttribute("width", "16");
    });

    it("shows a letter in capitals", () => {
      expect(renderTile({ icon: "w" })).toHaveTextContent(/^W$/);
    });

    it("shows the fallback line icon without an icon", () => {
      expect(renderTile({ fallback: Star }).querySelector("svg")).toBeInTheDocument();
    });
  });

  it("is hidden from assistive technology", () => {
    expect(renderTile({ icon: "w", color: "3" }).firstElementChild).toHaveAttribute("aria-hidden", "true");
  });
});
