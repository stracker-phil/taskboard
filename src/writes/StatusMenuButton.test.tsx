import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type TransitionOption } from "../api";
import { MenuHost } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { issueDetail } from "../test/issues";
import { connectionView } from "../test/jira";
import { behaviour, settings } from "../test/settings";
import { StatusMenuButton } from "./StatusMenuButton";

const mocked = vi.mocked(api, true);

const REVIEW: TransitionOption = { id: "31", statusId: "3", statusName: "In Review", style: "review", opensJira: false };
const DONE: TransitionOption = { id: "41", statusId: "4", statusName: "Done", style: "done", opensJira: false };
const SCREEN_MOVE: TransitionOption = { id: "51", statusId: "5", statusName: "Released", style: "done", opensJira: true };

function renderButton() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StatusMenuButton issue={issueDetail({ id: "10042", key: "NW-1042", statusName: "In Progress" })} />
      <MenuHost />
    </QueryClientProvider>,
  );
}

const allowChanges = (on = true) => mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: on }) }));
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

async function openMenu() {
  renderButton();
  await userEvent.click(await screen.findByRole("button", { name: "Change the status of NW-1042" }));
  return screen.findByRole("menu", { name: "Status of NW-1042" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
  mocked.connectionGet.mockResolvedValue(connectionView());
  mocked.issueTransitions.mockResolvedValue([REVIEW, DONE]);
});

describe("StatusMenuButton", () => {
  describe("the button", () => {
    it("wraps the status pill in a button that opens a menu while changes to Jira are on", async () => {
      renderButton();

      const button = await screen.findByRole("button", { name: "Change the status of NW-1042" });

      expect(button).toHaveAttribute("title", "Change status (S)");
      expect(button).toHaveAttribute("aria-haspopup", "menu");
      expect(button).toHaveTextContent("In Progress");
    });

    it("shows just the status pill while changes to Jira are off", async () => {
      allowChanges(false);
      renderButton();
      await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

      expect(screen.getByText("In Progress")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });
  });

  describe("the menu", () => {
    it("lists the issue's moves by target status under a caption", async () => {
      const menu = await openMenu();

      expect(screen.getByText("Move NW-1042 to")).toBeInTheDocument();
      expect(within(menu).getAllByRole("menuitem").map((row) => row.textContent)).toEqual(["In Review", "Done"]);
      expect(mocked.issueTransitions).toHaveBeenCalledWith("10042");
    });

    it("says there is no other status, in a disabled row, when there are no moves", async () => {
      mocked.issueTransitions.mockResolvedValue([]);

      const menu = await openMenu();

      expect(within(menu).getByRole("menuitem", { name: "No other status" })).toHaveAttribute("aria-disabled", "true");
    });

    it("sends the chosen transition and confirms the outcome", async () => {
      mocked.issueWrite.mockResolvedValue({ done: "Moved NW-1042 to In Review", reverse: null });
      const menu = await openMenu();

      await userEvent.click(within(menu).getByRole("menuitem", { name: "In Review" }));

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "transition", transitionId: "31" });
      await vi.waitFor(() => expect(toasts()).toEqual([["success", "Moved NW-1042 to In Review"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("shows an error toast when Jira refuses the transition", async () => {
      mocked.issueWrite.mockRejectedValue({ kind: "invalid_input", message: "The issue needs a resolution", code: null });
      const menu = await openMenu();

      await userEvent.click(within(menu).getByRole("menuitem", { name: "Done" }));

      await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not change the status: The issue needs a resolution"]]));
    });

    it("opens the issue in Jira, writing nothing, for a move that needs Jira's own screen", async () => {
      mocked.issueTransitions.mockResolvedValue([REVIEW, SCREEN_MOVE]);
      const menu = await openMenu();

      const row = within(menu).getByRole("menuitem", { name: /^Released/ });
      expect(row).toHaveTextContent("Opens Jira");
      await userEvent.click(row);

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-1042");
      expect(mocked.issueWrite).not.toHaveBeenCalled();
    });

    it("shows an error toast and no menu when the moves cannot be loaded", async () => {
      mocked.issueTransitions.mockRejectedValue({ kind: "io", message: "Network is down", code: null });
      renderButton();

      await userEvent.click(await screen.findByRole("button", { name: "Change the status of NW-1042" }));

      await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not load the statuses: Network is down"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  });
});
