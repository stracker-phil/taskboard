import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { installCommands } from ".";
import { UnreadButton } from "../shell/toolbar/UnreadButton";
import { clearCommands, setOverrides } from "./registry";
import { useShortcuts } from "./useShortcuts";

function Probe() {
  const { key, title } = useShortcuts();
  return (
    <button type="button" title={title("Only my issues", "toolbar.me")}>
      {key("toolbar.me") || "unbound"}
    </button>
  );
}

const probe = () => screen.getByRole("button");

beforeEach(() => {
  clearCommands();
  installCommands("main");
});

afterEach(() => {
  clearCommands();
});

describe("useShortcuts", () => {
  it("appends the command's default key to the label", () => {
    render(<Probe />);

    expect(probe()).toHaveAttribute("title", "Only my issues (⌘⇧M)");
    expect(probe()).toHaveTextContent("⌘⇧M");
  });

  it("follows a rebinding without remounting", () => {
    render(<Probe />);

    act(() => setOverrides({ "toolbar.me": "Mod+Alt+M" }));

    expect(probe()).toHaveAttribute("title", "Only my issues (⌘⌥M)");
  });

  it("shows only the label once the command is unbound", () => {
    render(<Probe />);

    act(() => setOverrides({ "toolbar.me": null }));

    expect(probe()).toHaveAttribute("title", "Only my issues");
    expect(probe()).toHaveTextContent("unbound");
  });

  it("shows only the label for a command that is not registered", () => {
    clearCommands();
    render(<Probe />);

    expect(probe()).toHaveAttribute("title", "Only my issues");
  });

  it("puts the current key in a toolbar button's tooltip", () => {
    render(<UnreadButton on={false} onChange={() => {}} />);
    expect(screen.getByRole("button", { name: "Unread only" })).toHaveAttribute("title", "Only unread issues, in every view (⌘⇧U)");

    act(() => setOverrides({ "toolbar.unread": "Mod+Alt+U" }));

    expect(screen.getByRole("button", { name: "Unread only" })).toHaveAttribute("title", "Only unread issues, in every view (⌘⌥U)");
  });
});
