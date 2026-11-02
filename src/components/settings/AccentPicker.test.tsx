import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Accent } from "../../api";
import { AccentPicker } from "./AccentPicker";

const NAMES = ["Blue", "Purple", "Pink", "Red", "Orange", "Yellow", "Green", "Teal", "Graphite"];

function renderPicker(value: Accent | null, appAccent?: Accent) {
  const onChange = vi.fn();
  render(<AccentPicker label="Accent color" value={value} onChange={onChange} appAccent={appAccent} />);
  return { onChange, group: screen.getByRole("radiogroup", { name: "Accent color" }) };
}

describe("AccentPicker", () => {
  it("offers the nine accents in order", () => {
    const { group } = renderPicker("blue");

    expect(within(group).getAllByRole("radio").map((r) => r.getAttribute("aria-label"))).toEqual(NAMES);
  });

  it("checks only the radio of the current value", () => {
    const { group } = renderPicker("green");

    expect(within(group).getByRole("radio", { name: "Green" })).toHaveAttribute("aria-checked", "true");
    expect(within(group).getAllByRole("radio", { checked: true })).toHaveLength(1);
  });

  it("reports the clicked accent", async () => {
    const { group, onChange } = renderPicker("blue");

    await userEvent.click(within(group).getByRole("radio", { name: "Purple" }));

    expect(onChange).toHaveBeenCalledWith("purple");
  });

  describe("with the app's accent", () => {
    it("offers a first radio that follows the app, checked when the value is null", () => {
      const { group } = renderPicker(null, "blue");

      const radios = within(group).getAllByRole("radio");
      expect(radios).toHaveLength(10);
      expect(radios[0]).toHaveAccessibleName("Same as the app (Blue)");
      expect(radios[0]).toHaveAttribute("aria-checked", "true");
      expect(within(group).getAllByRole("radio", { checked: true })).toHaveLength(1);
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
