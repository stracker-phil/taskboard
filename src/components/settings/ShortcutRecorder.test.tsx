import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { bindingFromEvent, bindingLabel } from "../../commands/registry";
import { ShortcutRecorder } from "./ShortcutRecorder";

const onChange = vi.fn();

function renderRecorder(value: string | null = null) {
  render(<ShortcutRecorder value={value} parse={bindingFromEvent} label={bindingLabel} onChange={onChange} name="Open Search" />);
  return screen.getByRole("button", { name: "Record shortcut for Open Search" });
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ShortcutRecorder", () => {
  it("shows the shortcut as its key label", () => {
    renderRecorder("Mod+Shift+K");

    expect(screen.getByRole("button")).toHaveTextContent("⌘⇧K");
  });

  it("shows unbound when there is no shortcut", () => {
    renderRecorder(null);

    expect(screen.getByRole("button")).toHaveTextContent("unbound");
  });

  it("starts recording on click", async () => {
    const field = renderRecorder("Mod+K");

    await userEvent.click(field);

    const recording = screen.getByRole("button", { name: "Press the new shortcut for Open Search" });
    expect(recording).toHaveTextContent("Press keys…");
    expect(recording).toHaveAttribute("data-recording", "true");
  });

  it("reports the parsed key combination and stops recording", async () => {
    await userEvent.click(renderRecorder("Mod+K"));

    await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");

    expect(onChange).toHaveBeenCalledWith("Mod+Shift+J");
    expect(screen.getByRole("button")).not.toHaveAttribute("data-recording");
  });

  it("keeps listening while only a modifier is pressed", async () => {
    await userEvent.click(renderRecorder());

    await userEvent.keyboard("{Meta>}");

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("button")).toHaveAttribute("data-recording", "true");
  });

  it.each(["{Backspace}", "{Delete}"])("clears the shortcut on %s", async (key) => {
    await userEvent.click(renderRecorder("Mod+K"));

    await userEvent.keyboard(key);

    expect(onChange).toHaveBeenCalledWith(null);
    expect(screen.getByRole("button")).not.toHaveAttribute("data-recording");
  });

  it("cancels on Escape without a change", async () => {
    await userEvent.click(renderRecorder("Mod+K"));

    await userEvent.keyboard("{Escape}");

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("button")).toHaveTextContent("⌘K");
  });

  it("stops recording when focus leaves", async () => {
    await userEvent.click(renderRecorder("Mod+K"));

    await userEvent.tab();

    expect(screen.getByRole("button")).not.toHaveAttribute("data-recording");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("ignores keys while not recording", async () => {
    renderRecorder("Mod+K").focus();

    await userEvent.keyboard("{Meta>}j{/Meta}");

    expect(onChange).not.toHaveBeenCalled();
  });
});
