import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { type AppStatusRow, api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { appStatus } from "../../../test/issues";
import { MenuHost } from "../../menu";
import { Toasts } from "../../Toasts";
import { StatusesPage } from "./StatusesPage";

const mocked = vi.mocked(api, true);

const BACKLOG = appStatus({ id: "1", name: "Backlog", category: "new", detected: "backlog" });
const IN_PROGRESS = appStatus();
const IN_REVIEW = appStatus({ id: "4", name: "In Review", detected: "review" });
const WAITING = appStatus({ id: "6", name: "Waiting for QA", detected: "todo", chosen: "blocked" });

async function renderPage(rows: AppStatusRow[]) {
  mocked.statusStylesAll.mockResolvedValue(rows);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StatusesPage />
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
const section = (name: string) => screen.getByRole("region", { name });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("StatusesPage", () => {
  it("says statuses appear after the first sync when there are none", async () => {
    await renderPage([]);

    expect(await screen.findByText("Statuses appear after the first sync.")).toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Search statuses" })).not.toBeInTheDocument();
  });

  it("puts each status in the section of its chosen style, or of its detected style without a choice", async () => {
    await renderPage([BACKLOG, IN_REVIEW, WAITING, IN_PROGRESS]);

    expect(within(await screen.findByRole("region", { name: "Backlog" })).getByRole("button", { name: "Style of Backlog" })).toBeInTheDocument();
    expect(within(section("In progress")).getByText("In Progress")).toBeInTheDocument();
    expect(within(section("In review")).getByText("In Review")).toBeInTheDocument();
    expect(within(section("Blocked")).getByText("Waiting for QA")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "To do" })).not.toBeInTheDocument();
  });

  it("orders the sections In progress, In review, Blocked, To do, Backlog, Done", async () => {
    const todo = appStatus({ id: "7", name: "Ready", detected: "todo" });
    const done = appStatus({ id: "5", name: "Shipped", detected: "done" });
    await renderPage([done, BACKLOG, todo, WAITING, IN_REVIEW, IN_PROGRESS]);

    await screen.findByRole("region", { name: "Done" });
    const order = ["In progress", "In review", "Blocked", "To do", "Backlog", "Done"];
    const names = screen.getAllByRole("region").map((r) => r.getAttribute("aria-label"));
    expect(names.filter((n) => order.includes(n ?? ""))).toEqual(order);
  });

  it("labels the button Automatic with the detected style without a choice, and with the chosen style's name otherwise", async () => {
    await renderPage([IN_REVIEW, WAITING]);

    expect(await screen.findByRole("button", { name: "Style of In Review" })).toHaveTextContent("Automatic · In review");
    expect(screen.getByRole("button", { name: "Style of Waiting for QA" })).toHaveTextContent(/^Blocked$/);
  });

  it("offers Automatic with the detected style, then the six styles", async () => {
    await renderPage([IN_REVIEW]);

    const menu = await openStyleMenu("In Review");

    expect(within(menu).getAllByRole("menuitemradio").map((r) => r.textContent)).toEqual([
      "Automatic (In review)",
      "Backlog",
      "To do",
      "In progress",
      "In review",
      "Blocked",
      "Done",
    ]);
    expect(within(menu).getByRole("menuitemradio", { name: "Automatic (In review)" })).toHaveAttribute("aria-checked", "true");
  });

  it("sends the chosen style and moves the row into the section of the rows Rust returns", async () => {
    await renderPage([IN_REVIEW]);
    mocked.statusStyleSetApp.mockResolvedValue([{ ...IN_REVIEW, chosen: "done" }]);
    const menu = await openStyleMenu("In Review");

    await choose(menu, "Done");

    expect(mocked.statusStyleSetApp).toHaveBeenCalledWith("4", "done");
    expect(await screen.findByRole("region", { name: "Done" })).toBeInTheDocument();
    expect(within(section("Done")).getByText("In Review")).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "In review" })).not.toBeInTheDocument();
  });

  it("resets to the detected style by sending no style when Automatic is chosen", async () => {
    await renderPage([WAITING]);
    mocked.statusStyleSetApp.mockResolvedValue([{ ...WAITING, chosen: null }]);
    const menu = await openStyleMenu("Waiting for QA");
    expect(within(menu).getByRole("menuitemradio", { name: "Blocked" })).toHaveAttribute("aria-checked", "true");

    await choose(menu, "Automatic (To do)");

    expect(mocked.statusStyleSetApp).toHaveBeenCalledWith("6", null);
    expect(await screen.findByRole("button", { name: "Style of Waiting for QA" })).toHaveTextContent("Automatic · To do");
    expect(within(section("To do")).getByText("Waiting for QA")).toBeInTheDocument();
  });

  it("reports a failed change and keeps the row where it was", async () => {
    await renderPage([IN_PROGRESS]);
    mocked.statusStyleSetApp.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    const menu = await openStyleMenu("In Progress");

    await choose(menu, "Done");

    expect(await screen.findByText(/Could not change In Progress/)).toBeInTheDocument();
    expect(within(section("In progress")).getByText("In Progress")).toBeInTheDocument();
  });

  describe("search", () => {
    it("keeps only the statuses whose name contains the text, ignoring case", async () => {
      await renderPage([IN_PROGRESS, IN_REVIEW, WAITING]);

      await userEvent.type(await screen.findByRole("textbox", { name: "Search statuses" }), "QA");

      expect(screen.getByText("Waiting for QA")).toBeInTheDocument();
      expect(screen.queryByText("In Review")).not.toBeInTheDocument();
      expect(screen.queryByText("In Progress")).not.toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "In review" })).not.toBeInTheDocument();
    });

    it("says no status matches when nothing does", async () => {
      await renderPage([IN_PROGRESS]);

      await userEvent.type(await screen.findByRole("textbox", { name: "Search statuses" }), "zzz");

      expect(screen.getByText("No status matches “zzz”.")).toBeInTheDocument();
      expect(screen.queryByText("In Progress")).not.toBeInTheDocument();
    });
  });
});
