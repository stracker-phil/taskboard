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
import { StatusesPage } from "./StatusesPage";

const mocked = vi.mocked(api, true);

const IN_PROGRESS = statusStyle();
const IN_REVIEW = statusStyle({ id: "4", name: "Code Review", category: "indeterminate", detected: "review", chosen: "blocked" });
const DONE = statusStyle({ id: "5", name: "Shipped", category: "done", detected: "done" });

async function renderPage(rows: StatusStyleRow[]) {
  mocked.statusStylesList.mockResolvedValue(rows);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StatusesPage />
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

describe("StatusesPage", () => {
  it("says statuses appear after the first sync when there are none", async () => {
    await renderPage([]);

    expect(screen.getByText("Statuses appear after the first sync.")).toBeInTheDocument();
  });

  it("lists each status as a pill with its Jira category", async () => {
    await renderPage([IN_PROGRESS, DONE]);

    expect(await screen.findByText("In Progress")).toBeInTheDocument();
    expect(screen.getByText("Jira category: In progress")).toBeInTheDocument();
    expect(screen.getByText("Shipped")).toBeInTheDocument();
    expect(screen.getByText("Jira category: Done")).toBeInTheDocument();
    expect(screen.queryByText("Statuses appear after the first sync.")).not.toBeInTheDocument();
  });

  it("labels the button Automatic without a choice and with the style's name otherwise", async () => {
    await renderPage([IN_PROGRESS, IN_REVIEW]);

    expect(await screen.findByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Automatic");
    expect(screen.getByRole("button", { name: "Style of Code Review" })).toHaveTextContent("Blocked");
  });

  it("offers Automatic with the detected style, then the five styles", async () => {
    await renderPage([IN_REVIEW]);

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

  it("sets the chosen style and shows the rows Rust returns", async () => {
    await renderPage([IN_PROGRESS]);
    mocked.statusStyleSet.mockResolvedValue([{ ...IN_PROGRESS, chosen: "blocked" }]);
    await userEvent.click(await screen.findByRole("button", { name: "Style of In Progress" }));

    await runEntry("Blocked");

    expect(mocked.statusStyleSet).toHaveBeenCalledWith("3", "blocked");
    expect(await screen.findByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Blocked");
  });

  it("goes back to Automatic by sending no style", async () => {
    await renderPage([IN_REVIEW]);
    mocked.statusStyleSet.mockResolvedValue([{ ...IN_REVIEW, chosen: null }]);
    await userEvent.click(await screen.findByRole("button", { name: "Style of Code Review" }));

    await runEntry("Automatic (In review)");

    expect(mocked.statusStyleSet).toHaveBeenCalledWith("4", null);
    expect(await within(screen.getByRole("region", { name: "Statuses" })).findByText("Automatic")).toBeInTheDocument();
  });

  it("reports a failed change and keeps the old choice", async () => {
    await renderPage([IN_PROGRESS]);
    mocked.statusStyleSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    await userEvent.click(await screen.findByRole("button", { name: "Style of In Progress" }));

    await runEntry("Done");

    expect(await screen.findByText(/Could not change In Progress/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Style of In Progress" })).toHaveTextContent("Automatic");
  });
});
