import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type StatusStyleRow } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { statusStyle } from "../../../test/issues";
import { MenuHost } from "../../menu";
import { Toasts } from "../../Toasts";
import { SpaceStatuses } from "./SpaceStatuses";

const mocked = vi.mocked(api, true);

const TO_DO = statusStyle({ id: "1", name: "Backlog", category: "new", detected: "todo" });
const IN_PROGRESS = statusStyle();
const IN_REVIEW = statusStyle({ id: "4", name: "Code Review", category: "indeterminate", detected: "review", chosen: "blocked" });
const DONE = statusStyle({ id: "5", name: "Shipped", category: "done", detected: "done" });

async function renderStatuses(rows: StatusStyleRow[], projectId = "10001") {
  mocked.statusStylesList.mockResolvedValue(rows);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpaceStatuses projectId={projectId} />
      <Toasts />
      <MenuHost />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Statuses" });
}

async function openStyleMenu(status: string) {
  await userEvent.click(await screen.findByRole("button", { name: `Style of ${status}` }));
  return screen.getByRole("menu", { name: `Style of ${status}` });
}
const choose = (menu: HTMLElement, name: string) => userEvent.click(within(menu).getByRole("menuitemradio", { name }));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("SpaceStatuses", () => {
  it("lists the statuses of the given Space", async () => {
    await renderStatuses([IN_PROGRESS], "10002");

    expect(await screen.findByText("In Progress")).toBeInTheDocument();
    expect(mocked.statusStylesList).toHaveBeenCalledWith("10002");
  });

  it("says statuses appear after the first sync when there are none", async () => {
    await renderStatuses([]);

    expect(screen.getByText("Statuses appear after the first sync.")).toBeInTheDocument();
  });

  it("groups the statuses by the style they show, in the order Backlog, To do, In progress, In review, Blocked, Done", async () => {
    const backlog = statusStyle({ id: "2", name: "Icebox", detected: "backlog" });
    await renderStatuses([DONE, backlog, TO_DO, IN_REVIEW, statusStyle({ id: "8", name: "Peer Review", detected: "review" }), IN_PROGRESS]);

    await screen.findByRole("region", { name: "Done" });
    const order = ["Backlog", "To do", "In progress", "In review", "Blocked", "Done"];
    const names = screen.getAllByRole("region").map((r) => r.getAttribute("aria-label"));
    expect(names.filter((n) => order.includes(n ?? ""))).toEqual(order);
    expect(within(screen.getByRole("region", { name: "Done" })).getByText("Shipped")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "In progress" })).getByText("In Progress")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "To do" })).getByText("Backlog")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "In review" })).getByText("Peer Review")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Blocked" })).getByText("Code Review")).toBeInTheDocument();
    expect(screen.queryByText("Statuses appear after the first sync.")).not.toBeInTheDocument();
  });

  it("puts a status without a choice of this Space in the section of the app-wide style", async () => {
    await renderStatuses([statusStyle({ id: "6", name: "Waiting for QA", detected: "todo", app: "review" })]);

    const review = await screen.findByRole("region", { name: "In review" });
    expect(within(review).getByText("Waiting for QA")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "To do" })).not.toBeInTheDocument();
  });

  it("puts a status the Space chose a style for in that section, whatever the app-wide style is", async () => {
    await renderStatuses([statusStyle({ id: "6", name: "Waiting for QA", detected: "todo", app: "review", chosen: "done" })]);

    expect(within(await screen.findByRole("region", { name: "Done" })).getByText("Waiting for QA")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "In review" })).not.toBeInTheDocument();
  });

  it("leaves out a style without statuses", async () => {
    await renderStatuses([IN_PROGRESS]);

    await screen.findByRole("region", { name: "In progress" });
    expect(screen.queryByRole("region", { name: "Done" })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "To do" })).not.toBeInTheDocument();
  });

  it.each([
    [0, "0 issues"],
    [1, "1 issue"],
    [12, "12 issues"],
  ])("shows %i issues in a status as %s", async (issues, text) => {
    await renderStatuses([statusStyle({ issues })]);

    expect(await screen.findByText(text)).toBeInTheDocument();
  });

  it("labels the button Default with the app-wide style without a choice and with the chosen style's name otherwise", async () => {
    await renderStatuses([IN_PROGRESS, IN_REVIEW]);

    expect(await screen.findByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Default · In progress");
    expect(screen.getByRole("button", { name: "Style of Code Review" })).toHaveTextContent(/^Blocked$/);
  });

  it("labels the button with the app-wide style rather than the detected one when the app chose one", async () => {
    await renderStatuses([statusStyle({ detected: "progress", app: "review" })]);

    expect(await screen.findByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Default · In review");
  });

  describe("search", () => {
    it("keeps only the statuses whose name contains the text, ignoring case", async () => {
      await renderStatuses([IN_PROGRESS, IN_REVIEW, DONE]);

      await userEvent.type(await screen.findByRole("textbox", { name: "Search statuses" }), "REVIEW");

      expect(screen.getByText("Code Review")).toBeInTheDocument();
      expect(screen.queryByText("In Progress")).not.toBeInTheDocument();
      expect(screen.queryByText("Shipped")).not.toBeInTheDocument();
    });

    it("says no status matches when nothing does", async () => {
      await renderStatuses([IN_PROGRESS]);

      await userEvent.type(await screen.findByRole("textbox", { name: "Search statuses" }), "zzz");

      expect(screen.getByText("No status matches “zzz”.")).toBeInTheDocument();
      expect(screen.queryByText("In Progress")).not.toBeInTheDocument();
    });

    it("has no search field without statuses", async () => {
      await renderStatuses([]);

      expect(screen.queryByRole("textbox", { name: "Search statuses" })).not.toBeInTheDocument();
    });
  });

  it("offers Default with the app-wide style, then the six styles starting with Backlog", async () => {
    await renderStatuses([statusStyle({ id: "4", name: "Code Review", detected: "todo", app: "review" })]);

    const menu = await openStyleMenu("Code Review");

    expect(within(menu).getAllByRole("menuitemradio").map((r) => r.textContent)).toEqual([
      "Default (In review)",
      "Backlog",
      "To do",
      "In progress",
      "In review",
      "Blocked",
      "Done",
    ]);
  });

  it("checks the chosen style in the menu", async () => {
    await renderStatuses([IN_REVIEW]);

    const menu = await openStyleMenu("Code Review");

    expect(within(menu).getByRole("menuitemradio", { name: "Blocked" })).toHaveAttribute("aria-checked", "true");
    expect(within(menu).getByRole("menuitemradio", { name: "Default (In review)" })).toHaveAttribute("aria-checked", "false");
  });

  it("checks Default in the menu while no style is chosen", async () => {
    await renderStatuses([IN_PROGRESS]);

    const menu = await openStyleMenu("In Progress");

    expect(within(menu).getByRole("menuitemradio", { name: /^Default/ })).toHaveAttribute("aria-checked", "true");
    expect(within(menu).getAllByRole("menuitemradio", { checked: true })).toHaveLength(1);
  });

  it("sets the chosen style for the Space and shows the rows Rust returns", async () => {
    await renderStatuses([IN_PROGRESS], "10002");
    mocked.statusStyleSet.mockResolvedValue([{ ...IN_PROGRESS, chosen: "blocked" }]);
    const menu = await openStyleMenu("In Progress");

    await choose(menu, "Blocked");

    expect(mocked.statusStyleSet).toHaveBeenCalledWith("10002", "3", "blocked");
    expect(await screen.findByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Blocked");
    expect(within(screen.getByRole("region", { name: "Blocked" })).getByText("In Progress")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "In progress" })).not.toBeInTheDocument();
  });

  it("goes back to Default by sending no style and moves the row to the app-wide style's section", async () => {
    const row = { ...IN_REVIEW, app: "todo" as const };
    await renderStatuses([row]);
    mocked.statusStyleSet.mockResolvedValue([{ ...row, chosen: null }]);
    const menu = await openStyleMenu("Code Review");

    await choose(menu, "Default (To do)");

    expect(mocked.statusStyleSet).toHaveBeenCalledWith("10001", "4", null);
    expect(await screen.findByRole("button", { name: "Style of Code Review" })).toHaveTextContent("Default · To do");
    expect(within(screen.getByRole("region", { name: "To do" })).getByText("Code Review")).toBeInTheDocument();
  });

  it("reports a failed change and keeps the old choice", async () => {
    await renderStatuses([IN_PROGRESS]);
    mocked.statusStyleSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    const menu = await openStyleMenu("In Progress");

    await choose(menu, "Done");

    expect(await screen.findByText(/Could not change In Progress/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Default · In progress");
  });
});
