import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands } from "../commands";
import { clearCommands } from "../commands/registry";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { sidebarTree, space } from "../test/spaces";
import { listNode, view } from "../test/views";
import { Window } from "./Window";

const mocked = vi.mocked(api, true);

function StoreWindow() {
  const sidebarHidden = useStore((s) => s.sidebarHidden);
  return (
    <Window sidebarHidden={sidebarHidden}>
      <p>View body</p>
    </Window>
  );
}

function renderWindow(sidebarHidden: boolean) {
  resetStore({ sidebarHidden });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StoreWindow />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  installCommands("main");
  mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space({ lists: [listNode({ id: 21, name: "My open issues" })] })]));
  mocked.viewsOf.mockResolvedValue([view()]);
});

afterEach(() => clearCommands());

describe("SidebarToggle", () => {
  it("shows a pressed Hide sidebar button while the sidebar is shown", async () => {
    renderWindow(false);

    const button = await screen.findByRole("button", { name: "Hide sidebar" });

    expect(button).toHaveAttribute("aria-pressed", "true");
    expect(button).toHaveAttribute("title", "Hide sidebar (⌘\\)");
    expect(await screen.findByRole("complementary", { name: "Sidebar" })).toBeInTheDocument();
  });

  it("shows an unpressed Show sidebar button while the sidebar is hidden", async () => {
    renderWindow(true);

    const button = await screen.findByRole("button", { name: "Show sidebar" });

    expect(button).toHaveAttribute("aria-pressed", "false");
    expect(button).toHaveAttribute("title", "Show sidebar (⌘\\)");
    expect(screen.queryByRole("complementary", { name: "Sidebar" })).not.toBeInTheDocument();
  });

  it.each([
    ["shown", false, "Hide sidebar"],
    ["hidden", true, "Show sidebar"],
  ])("runs the view:toggleSidebar command when clicked with the sidebar %s", async (_state, hidden, name) => {
    renderWindow(hidden);

    await userEvent.click(await screen.findByRole("button", { name }));

    expect(mocked.runCommand).toHaveBeenCalledWith("view:toggleSidebar");
  });

  it("stays the same focused element when the sidebar is toggled away", async () => {
    renderWindow(false);
    const button = await screen.findByRole("button", { name: "Hide sidebar" });
    await screen.findByRole("complementary", { name: "Sidebar" });
    await userEvent.click(button);
    expect(button).toHaveFocus();

    act(() => useStore.getState().toggleSidebar());

    expect(screen.getByRole("button", { name: "Show sidebar" })).toBe(button);
    expect(button).toHaveAttribute("aria-pressed", "false");
    expect(button).toHaveFocus();
    expect(screen.queryByRole("complementary", { name: "Sidebar" })).not.toBeInTheDocument();
  });
});
