import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Accent } from "../../api";
import { AccentPicker } from "./AccentPicker";

const NAMES = ["System", "Orange", "Gold", "Red", "Magenta", "Purple", "Indigo", "Blue", "Teal", "Graphite"];

function renderPicker(value: Accent | null, appAccent?: Accent) {
  const onChange = vi.fn();
  render(<AccentPicker label="Accent color" value={value} onChange={onChange} appAccent={appAccent} />);
  return { onChange, group: screen.getByRole("radiogroup", { name: "Accent color" }) };
}

describe("AccentPicker", () => {
  it("offers System first, then the nine hues in order", () => {
    const { group } = renderPicker("blue");

    expect(within(group).getAllByRole("radio").map((r) => r.getAttribute("aria-label"))).toEqual(NAMES);
  });

  it("checks only the radio of the current value", () => {
    const { group } = renderPicker("gold");

    expect(within(group).getByRole("radio", { name: "Gold" })).toHaveAttribute("aria-checked", "true");
    expect(within(group).getAllByRole("radio", { checked: true })).toHaveLength(1);
  });

  it("reports the clicked accent", async () => {
    const { group, onChange } = renderPicker("blue");

    await userEvent.click(within(group).getByRole("radio", { name: "Magenta" }));

    expect(onChange).toHaveBeenCalledWith("magenta");
  });

  it("reports System when its radio is clicked", async () => {
    const { group, onChange } = renderPicker("blue");

    await userEvent.click(within(group).getByRole("radio", { name: "System" }));

    expect(onChange).toHaveBeenCalledWith("system");
  });

  describe("with the app's accent", () => {
    it("offers a first radio that follows the app, checked when the value is null", () => {
      const { group } = renderPicker(null, "indigo");

      const radios = within(group).getAllByRole("radio");
      expect(radios).toHaveLength(11);
      expect(radios[0]).toHaveAccessibleName("Same as the app (Indigo)");
      expect(radios[0]).toHaveAttribute("aria-checked", "true");
      expect(within(group).getAllByRole("radio", { checked: true })).toHaveLength(1);
    });

    it("names System as the app's accent", () => {
      const { group } = renderPicker(null, "system");

      expect(within(group).getByRole("radio", { name: "Same as the app (System)" })).toHaveAttribute("aria-checked", "true");
    });

    it("leaves the app radio unchecked when the value is an accent", () => {
      const { group } = renderPicker("teal", "orange");

      expect(within(group).getByRole("radio", { name: "Same as the app (Orange)" })).toHaveAttribute("aria-checked", "false");
      expect(within(group).getByRole("radio", { name: "Teal" })).toHaveAttribute("aria-checked", "true");
    });

    it("reports null when the app radio is clicked", async () => {
      const { group, onChange } = renderPicker("teal", "blue");

      await userEvent.click(within(group).getByRole("radio", { name: "Same as the app (Blue)" }));

      expect(onChange).toHaveBeenCalledWith(null);
    });
  });
});
