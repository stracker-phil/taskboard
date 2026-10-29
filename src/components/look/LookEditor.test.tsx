import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Star } from "lucide-react";
import { describe, expect, it, vi } from "vitest";
import { LookEditor } from "./LookEditor";
import type { Look } from "./Tile";

function renderEditor(look: Look, props: { colorOptional?: boolean } = {}) {
  const onChange = vi.fn();
  render(<LookEditor look={look} fallback={Star} onChange={onChange} {...props} />);
  return onChange;
}

describe("LookEditor", () => {
  describe("icon grid", () => {
    it("names each option after its icon, with dashes as spaces", () => {
      renderEditor({ icon: null, color: "1" });

      const grid = screen.getByRole("radiogroup", { name: "Icon" });
      expect(within(grid).getByRole("radio", { name: "rocket" })).toBeInTheDocument();
      expect(within(grid).getByRole("radio", { name: "check circle" })).toBeInTheDocument();
    });

    it("checks the current line icon only", () => {
      renderEditor({ icon: "rocket", color: "1" });

      const grid = screen.getByRole("radiogroup", { name: "Icon" });
      expect(within(grid).getByRole("radio", { name: "rocket", checked: true })).toBeInTheDocument();
      expect(within(grid).getAllByRole("radio", { checked: true })).toHaveLength(1);
    });

    it("checks nothing without an icon", () => {
      renderEditor({ icon: null, color: "1" });

      expect(within(screen.getByRole("radiogroup", { name: "Icon" })).queryAllByRole("radio", { checked: true })).toHaveLength(0);
    });

    it("reports the picked icon with the color kept", async () => {
      const onChange = renderEditor({ icon: null, color: "4" });

      await userEvent.click(screen.getByRole("radio", { name: "check circle" }));

      expect(onChange).toHaveBeenCalledWith({ icon: "check-circle", color: "4" });
    });
  });

  describe("Reset", () => {
    it("reports no icon with the color kept", async () => {
      const onChange = renderEditor({ icon: "rocket", color: "4" });

      await userEvent.click(screen.getByRole("button", { name: "Reset" }));

      expect(onChange).toHaveBeenCalledWith({ icon: null, color: "4" });
    });

    it("is disabled without an icon", () => {
      renderEditor({ icon: null, color: "4" });

      expect(screen.getByRole("button", { name: "Reset" })).toBeDisabled();
    });
  });

  describe("icon kind", () => {
    it("starts on Icon for a line icon", () => {
      renderEditor({ icon: "rocket", color: "1" });

      expect(screen.getByRole("button", { name: "Icon", pressed: true })).toBeInTheDocument();
      expect(screen.queryByRole("textbox", { name: "Letter or emoji" })).not.toBeInTheDocument();
    });

    it("starts on Letter or emoji for a stored letter and shows it", () => {
      renderEditor({ icon: "W", color: "1" });

      expect(screen.getByRole("button", { name: "Letter or emoji", pressed: true })).toBeInTheDocument();
      expect(screen.getByRole("textbox", { name: "Letter or emoji" })).toHaveValue("W");
      expect(screen.queryByRole("radiogroup", { name: "Icon" })).not.toBeInTheDocument();
    });

    it("switches to the text field and back to the grid", async () => {
      renderEditor({ icon: null, color: "1" });

      await userEvent.click(screen.getByRole("button", { name: "Letter or emoji" }));
      expect(screen.getByRole("textbox", { name: "Letter or emoji" })).toBeInTheDocument();
      await userEvent.click(screen.getByRole("button", { name: "Icon" }));

      expect(screen.getByRole("radiogroup", { name: "Icon" })).toBeInTheDocument();
    });
  });

  describe("letter or emoji", () => {
    async function openGlyph(look: Look = { icon: null, color: "2" }) {
      const onChange = renderEditor(look);
      await userEvent.click(screen.getByRole("button", { name: "Letter or emoji" }));
      return { onChange, field: screen.getByRole("textbox", { name: "Letter or emoji" }) };
    }

    it("commits on Enter, keeping the color", async () => {
      const { onChange, field } = await openGlyph();

      await userEvent.type(field, "q{Enter}");

      expect(onChange).toHaveBeenCalledWith({ icon: "q", color: "2" });
    });

    it("commits on blur", async () => {
      const { onChange, field } = await openGlyph();

      await userEvent.type(field, "7");
      await userEvent.tab();

      expect(onChange).toHaveBeenCalledWith({ icon: "7", color: "2" });
    });

    it.each([
      ["several letters", "abc", "a"],
      ["an emoji followed by text", "🚀x", "🚀"],
      ["an emoji with a skin tone", "👍🏽!", "👍🏽"],
      ["surrounding spaces", "  k ", "k"],
    ])("keeps only the first character of %s", async (_name, typed, kept) => {
      const { onChange, field } = await openGlyph();

      await userEvent.type(field, `${typed}{Enter}`);

      expect(onChange).toHaveBeenCalledWith({ icon: kept, color: "2" });
    });

    it("reports no icon when the field is cleared", async () => {
      const { onChange, field } = await openGlyph({ icon: "W", color: "2" });

      await userEvent.clear(field);
      await userEvent.tab();

      expect(onChange).toHaveBeenCalledWith({ icon: null, color: "2" });
    });

    it("reports nothing when the value is unchanged", async () => {
      const { onChange, field } = await openGlyph({ icon: "W", color: "2" });

      await userEvent.click(field);
      await userEvent.tab();

      expect(onChange).not.toHaveBeenCalled();
    });
  });

  describe("colors", () => {
    it("offers the twelve colors in palette order", () => {
      renderEditor({ icon: null, color: "1" });

      const names = within(screen.getByRole("radiogroup", { name: "Color" })).getAllByRole("radio").map((r) => r.getAttribute("aria-label"));
      expect(names).toEqual(["Blue", "Slate", "Violet", "Teal", "Amber", "Red", "Pink", "Green", "Indigo", "Cyan", "Orange", "Stone"]);
    });

    it("checks the current color only", () => {
      renderEditor({ icon: null, color: "5" });

      const group = screen.getByRole("radiogroup", { name: "Color" });
      expect(within(group).getByRole("radio", { name: "Amber", checked: true })).toBeInTheDocument();
      expect(within(group).getAllByRole("radio", { checked: true })).toHaveLength(1);
    });

    it("reports the picked color with the icon kept", async () => {
      const onChange = renderEditor({ icon: "rocket", color: "1" });

      await userEvent.click(screen.getByRole("radio", { name: "Teal" }));

      expect(onChange).toHaveBeenCalledWith({ icon: "rocket", color: "4" });
    });

    describe("when optional", () => {
      it("offers No color first and checks it without a color", () => {
        renderEditor({ icon: null, color: null }, { colorOptional: true });

        const radios = within(screen.getByRole("radiogroup", { name: "Color" })).getAllByRole("radio");
        expect(radios[0]).toHaveAccessibleName("No color");
        expect(radios[0]).toBeChecked();
        expect(radios).toHaveLength(13);
      });

      it("reports no color with the icon kept", async () => {
        const onChange = renderEditor({ icon: "rocket", color: "6" }, { colorOptional: true });

        await userEvent.click(screen.getByRole("radio", { name: "No color" }));

        expect(onChange).toHaveBeenCalledWith({ icon: "rocket", color: null });
      });
    });

    it("offers no No color option by default", () => {
      renderEditor({ icon: null, color: "1" });

      expect(screen.queryByRole("radio", { name: "No color" })).not.toBeInTheDocument();
    });
  });
});
