import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Behaviour } from "../api";
import { installCommands } from "../commands";
import { MenuHost } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { issueDetail } from "../test/issues";
import { behaviour, settings } from "../test/settings";
import { PriorityMenuButton } from "./PriorityMenuButton";

const mocked = vi.mocked(api, true);

function renderButton() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PriorityMenuButton issue={issueDetail({ id: "10042", key: "NW-1042" })}>High</PriorityMenuButton>
      <MenuHost />
    </QueryClientProvider>,
  );
}

const allowChanges = (patch: Partial<Behaviour> = {}) =>
  mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, ...patch }) }));
const PRIORITIES = ["Highest", "High", "Medium", "Low", "Lowest"].map((name, i) => ({ name, level: i + 1 }));
const toasts =() => useStore.getState().toasts.map((t) => [t.kind, t.message]);

async function openMenu() {
  renderButton();
  await userEvent.click(await screen.findByRole("button", { name: "Change the priority of NW-1042" }));
  return screen.findByRole("menu", { name: "Priority of NW-1042" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
  allowChanges();
  mocked.issuePriorities.mockResolvedValue({ options: PRIORITIES, current: "Medium" });
  mocked.issueWrite.mockResolvedValue({ done: "Done", reverse: null });
});

describe("PriorityMenuButton", () => {
  describe("the button", () => {
    it("wraps the priority in a button that opens a menu while changes to Jira are on", async () => {
      renderButton();

      const button = await screen.findByRole("button", { name: "Change the priority of NW-1042" });

      expect(button).toHaveAttribute("title", expect.stringContaining("Set priority"));
      expect(button).toHaveAttribute("aria-haspopup", "menu");
      expect(button).toHaveTextContent("High");
    });

    it("shows the plain priority while priority changes are switched off, though changes to Jira are on", async () => {
      allowChanges({ disabledChanges: ["changePriority"] });
      renderButton();
      await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

      expect(screen.getByText("High")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });

    it("stays a button when only the assignee change is switched off", async () => {
      allowChanges({ disabledChanges: ["assign"] });
      renderButton();

      expect(await screen.findByRole("button", { name: "Change the priority of NW-1042" })).toBeInTheDocument();
    });

    it("shows the plain priority while changes to Jira are off", async () => {
      allowChanges({ allowJiraChanges: false });
      renderButton();
      await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

      expect(screen.getByText("High")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });
  });

  describe("the menu", () => {
    it("lists the priorities Jira offers under a caption and asks Rust for this issue's", async () => {
      const menu = await openMenu();

      expect(screen.getByText("Set NW-1042 to")).toBeInTheDocument();
      expect(within(menu).getAllByRole("menuitemradio").map((item) => item.textContent)).toEqual(
        ["Highest", "High", "Medium", "Low", "Lowest"].map((name) => expect.stringContaining(name)),
      );
      expect(mocked.issuePriorities).toHaveBeenCalledWith("10042");
    });

    it("checks only the current priority", async () => {
      const menu = await openMenu();

      expect(within(menu).getByRole("menuitemradio", { name: /Medium/ })).toHaveAttribute("aria-checked", "true");
      expect(within(menu).getByRole("menuitemradio", { name: /Highest/ })).toHaveAttribute("aria-checked", "false");
      expect(within(menu).getByRole("menuitemradio", { name: /Lowest/ })).toHaveAttribute("aria-checked", "false");
    });

    it("checks nothing when the issue has no priority", async () => {
      mocked.issuePriorities.mockResolvedValue({ options: [{ name: "High", level: 2 }, { name: "Low", level: 4 }], current: null });

      const menu = await openMenu();

      expect(within(menu).getAllByRole("menuitemradio").map((item) => item.getAttribute("aria-checked"))).toEqual(["false", "false"]);
    });

    it("sets the chosen priority and confirms the outcome", async () => {
      mocked.issueWrite.mockResolvedValue({ done: "Set the priority of NW-1042 to Low", reverse: null });
      const menu = await openMenu();

      await userEvent.click(within(menu).getByRole("menuitemradio", { name: /Low$/ }));

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "priority", name: "Low" });
      await vi.waitFor(() => expect(toasts()).toEqual([["success", "Set the priority of NW-1042 to Low"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("shows an error toast when Jira refuses the change", async () => {
      mocked.issueWrite.mockRejectedValue({ kind: "invalid_input", message: "The priority is not on the screen", code: null });
      const menu = await openMenu();

      await userEvent.click(within(menu).getByRole("menuitemradio", { name: /Highest/ }));

      await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not change the priority: The priority is not on the screen"]]));
    });

    it("shows an error toast and no menu when the priorities cannot be loaded", async () => {
      mocked.issuePriorities.mockRejectedValue({ kind: "io", message: "Network is down", code: null });
      renderButton();

      await userEvent.click(await screen.findByRole("button", { name: "Change the priority of NW-1042" }));

      await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not load the priorities: Network is down"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  });
});
