import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { MenuHost } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { ALEX, issueDetail, SAM } from "../test/issues";
import { behaviour, settings } from "../test/settings";
import { AssigneeMenuButton } from "./AssigneeMenuButton";

const mocked = vi.mocked(api, true);

function renderButton() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AssigneeMenuButton issue={issueDetail({ id: "10042", key: "NW-1042" })}>Alex Morgan</AssigneeMenuButton>
      <MenuHost />
    </QueryClientProvider>,
  );
}

const allowChanges = (on = true) => mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: on }) }));
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

async function openMenu() {
  renderButton();
  await userEvent.click(await screen.findByRole("button", { name: "Change the assignee of NW-1042" }));
  return screen.findByRole("menu", { name: "Assignee of NW-1042" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
  mocked.issueAssignees.mockResolvedValue({ people: [ALEX, SAM], current: ALEX.accountId });
});

describe("AssigneeMenuButton", () => {
  describe("the button", () => {
    it("wraps the assignee in a button that opens a menu while changes to Jira are on", async () => {
      renderButton();

      const button = await screen.findByRole("button", { name: "Change the assignee of NW-1042" });

      expect(button).toHaveAttribute("title", "Assign to… (A)");
      expect(button).toHaveAttribute("aria-haspopup", "menu");
      expect(button).toHaveTextContent("Alex Morgan");
    });

    it("shows the plain assignee while assigning is switched off, though changes to Jira are on", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["assign"] }) }));
      renderButton();
      await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

      expect(screen.getByText("Alex Morgan")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });

    it("stays a button when only the status change is switched off", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["changeStatus"] }) }));
      renderButton();

      expect(await screen.findByRole("button", { name: "Change the assignee of NW-1042" })).toBeInTheDocument();
    });

    it("shows the plain assignee while changes to Jira are off", async () => {
      allowChanges(false);
      renderButton();
      await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

      expect(screen.getByText("Alex Morgan")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });
  });

  describe("the menu", () => {
    it("lists the team under a caption, then Unassigned, and asks Rust for this issue's team", async () => {
      const menu = await openMenu();

      expect(screen.getByText("Assign NW-1042 to")).toBeInTheDocument();
      expect(within(menu).getAllByRole("menuitemradio").map((row) => row.textContent)).toEqual(["Alex Morgan", "Sam Lee", "Unassigned"].map((name) => expect.stringContaining(name)));
      expect(mocked.issueAssignees).toHaveBeenCalledWith("10042");
    });

    it("checks the current assignee", async () => {
      const menu = await openMenu();

      expect(within(menu).getByRole("menuitemradio", { name: /Alex Morgan/ })).toHaveAttribute("aria-checked", "true");
      expect(within(menu).getByRole("menuitemradio", { name: /Sam Lee/ })).toHaveAttribute("aria-checked", "false");
      expect(within(menu).getByRole("menuitemradio", { name: "Unassigned" })).toHaveAttribute("aria-checked", "false");
    });

    it("checks Unassigned when nobody is assigned", async () => {
      mocked.issueAssignees.mockResolvedValue({ people: [ALEX, SAM], current: null });

      const menu = await openMenu();

      expect(within(menu).getByRole("menuitemradio", { name: "Unassigned" })).toHaveAttribute("aria-checked", "true");
      expect(within(menu).getByRole("menuitemradio", { name: /Alex Morgan/ })).toHaveAttribute("aria-checked", "false");
    });

    it("assigns the chosen person and confirms the outcome", async () => {
      mocked.issueWrite.mockResolvedValue({ done: "Assigned NW-1042 to Sam Lee", reverse: null });
      const menu = await openMenu();

      await userEvent.click(within(menu).getByRole("menuitemradio", { name: /Sam Lee/ }));

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "assign", accountId: "demo-account-2" });
      await vi.waitFor(() => expect(toasts()).toEqual([["success", "Assigned NW-1042 to Sam Lee"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("has a focused search field that assigns the person typed, on Enter", async () => {
      mocked.issueWrite.mockResolvedValue({ done: "Assigned NW-1042 to Sam Lee", reverse: null });
      const menu = await openMenu();

      expect(within(menu).getByRole("textbox", { name: "Search people" })).toHaveFocus();
      await userEvent.keyboard("sam{Enter}");

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "assign", accountId: "demo-account-2" });
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("unassigns with a null account when Unassigned is chosen", async () => {
      mocked.issueWrite.mockResolvedValue({ done: "Unassigned NW-1042", reverse: null });
      const menu = await openMenu();

      await userEvent.click(within(menu).getByRole("menuitemradio", { name: "Unassigned" }));

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "assign", accountId: null });
      await vi.waitFor(() => expect(toasts()).toEqual([["success", "Unassigned NW-1042"]]));
    });

    it("shows an error toast when Jira refuses the change", async () => {
      mocked.issueWrite.mockRejectedValue({ kind: "invalid_input", message: "The user cannot be assigned", code: null });
      const menu = await openMenu();

      await userEvent.click(within(menu).getByRole("menuitemradio", { name: /Sam Lee/ }));

      await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not change the assignee: The user cannot be assigned"]]));
    });

    it("shows an error toast and no menu when the team cannot be loaded", async () => {
      mocked.issueAssignees.mockRejectedValue({ kind: "io", message: "Network is down", code: null });
      renderButton();

      await userEvent.click(await screen.findByRole("button", { name: "Change the assignee of NW-1042" }));

      await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not load the team: Network is down"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  });
});
