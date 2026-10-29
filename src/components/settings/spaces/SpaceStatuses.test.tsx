import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type MenuEntry, type StatusStyleRow } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { statusStyle } from "../../../test/issues";
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
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Statuses" });
}

const lastMenu = (): MenuEntry[] => mocked.popupMenu.mock.lastCall![0];
const runEntry = (text: string) => {
  const entry = lastMenu().find((e) => e !== "separator" && e.text === text);
  if (!entry || entry === "separator") throw new Error(`No menu entry "${text}"`);
  return act(async () => entry.run?.());
};

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

  it("groups the statuses under their Jira category in the order To do, In progress, Done", async () => {
    await renderStatuses([DONE, IN_PROGRESS, TO_DO]);

    await screen.findByRole("region", { name: "Done" });
    const names = screen.getAllByRole("region").map((r) => r.getAttribute("aria-label"));
    expect(names.filter((n) => n === "To do" || n === "In progress" || n === "Done")).toEqual(["To do", "In progress", "Done"]);
    expect(within(screen.getByRole("region", { name: "Done" })).getByText("Shipped")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "In progress" })).getByText("In Progress")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "To do" })).getByText("Backlog")).toBeInTheDocument();
    expect(screen.queryByText("Statuses appear after the first sync.")).not.toBeInTheDocument();
  });

  it("leaves out a category without statuses", async () => {
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

  it("labels the button with the detected style without a choice and with the chosen style's name otherwise", async () => {
    await renderStatuses([IN_PROGRESS, IN_REVIEW]);

    expect(await screen.findByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Automatic · In progress");
    expect(screen.getByRole("button", { name: "Style of Code Review" })).toHaveTextContent(/^Blocked$/);
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

  it("offers Automatic with the detected style, then the five styles", async () => {
    await renderStatuses([IN_REVIEW]);

    await userEvent.click(await screen.findByRole("button", { name: "Style of Code Review" }));

    expect(lastMenu().map((e) => (e === "separator" ? e : e.text))).toEqual([
      "Automatic (In review)",
      "separator",
      "To do",
      "In progress",
      "In review",
      "Blocked",
      "Done",
    ]);
  });

  it("sets the chosen style for the Space and shows the rows Rust returns", async () => {
    await renderStatuses([IN_PROGRESS], "10002");
    mocked.statusStyleSet.mockResolvedValue([{ ...IN_PROGRESS, chosen: "blocked" }]);
    await userEvent.click(await screen.findByRole("button", { name: "Style of In Progress" }));

    await runEntry("Blocked");

    expect(mocked.statusStyleSet).toHaveBeenCalledWith("10002", "3", "blocked");
    expect(await screen.findByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Blocked");
  });

  it("goes back to Automatic by sending no style", async () => {
    await renderStatuses([IN_REVIEW]);
    mocked.statusStyleSet.mockResolvedValue([{ ...IN_REVIEW, chosen: null }]);
    await userEvent.click(await screen.findByRole("button", { name: "Style of Code Review" }));

    await runEntry("Automatic (In review)");

    expect(mocked.statusStyleSet).toHaveBeenCalledWith("10001", "4", null);
    expect(await screen.findByRole("button", { name: "Style of Code Review" })).toHaveTextContent("Automatic · In review");
  });

  it("reports a failed change and keeps the old choice", async () => {
    await renderStatuses([IN_PROGRESS]);
    mocked.statusStyleSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    await userEvent.click(await screen.findByRole("button", { name: "Style of In Progress" }));

    await runEntry("Done");

    expect(await screen.findByText(/Could not change In Progress/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Automatic");
  });
});
