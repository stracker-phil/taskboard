import { act, render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { installCommands } from ".";
import { InboxButton } from "../inbox/InboxButton";
import { createQueryClient } from "../queries/queryClient";
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
    render(
      <QueryClientProvider client={createQueryClient()}>
        <InboxButton />
      </QueryClientProvider>,
    );
    expect(screen.getByRole("button", { name: "Notifications" })).toHaveAttribute("title", "Notifications (⌘⇧U)");

    act(() => setOverrides({ "inbox.toggle": "Mod+Alt+U" }));

    expect(screen.getByRole("button", { name: "Notifications" })).toHaveAttribute("title", "Notifications (⌘⌥U)");
  });
});
