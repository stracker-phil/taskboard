import { render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { narrowingKey } from "../state/slices/narrowing";
import { resetStore, useStore } from "../state/store";
import { groupOf, row, viewData } from "../test/issues";
import { projectCounts, syncOverview } from "../test/jira";
import { view } from "../test/views";
import { MainBody } from "./MainBody";

const mocked = vi.mocked(api, true);

type Restorable = { mockRestore: () => void };
let spies: Restorable[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  spies = [
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(1000),
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(800),
  ];
});

afterEach(() => {
  spies.forEach((s) => s.mockRestore());
});

function renderBody() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <MainBody />
    </QueryClientProvider>,
  );
}

describe("MainBody", () => {
  it("asks to connect Jira while no project is tracked", async () => {
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });

    renderBody();

    expect(await screen.findByText("Connect Jira in Settings")).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  it("shows the sync status while the tracked project has no cached issues", async () => {
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts({ cached: 0, open: 0 })] }));

    renderBody();

    expect(await screen.findByText("0 issues in 1 Space")).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  it("shows the list of the current place once a project has cached issues", async () => {
    const issue = row({ summary: "Redesign the pricing page" });
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
    mocked.viewRows.mockResolvedValue(viewData([groupOf([issue])], [issue]));

    renderBody();

    expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
    expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, expect.anything());
  });

  describe("layouts", () => {
    beforeEach(() => {
      const issue = row({ summary: "Redesign the pricing page" });
      mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
      mocked.viewRows.mockResolvedValue(viewData([groupOf([issue])], [issue]));
    });

    it("shows the table for a view with the table layout", async () => {
      mocked.viewsOf.mockResolvedValue([view({ layout: "table" })]);

      renderBody();

      expect(await screen.findByText("Key")).toBeInTheDocument();
      expect(screen.queryByText("Name")).not.toBeInTheDocument();
    });

    it.each(["list", "board"] as const)("shows the list for a view with the %s layout", async (layout) => {
      mocked.viewsOf.mockResolvedValue([view({ layout })]);

      renderBody();

      expect(await screen.findByRole("button", { name: /^(Collapse|Expand) / })).toBeInTheDocument();
      expect(screen.getByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.queryByText("Key")).not.toBeInTheDocument();
    });
  });

  describe("title wrapping", () => {
    beforeEach(() => {
      const issue = row({ key: "NW-1042", summary: "Redesign the pricing page" });
      mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
      mocked.viewRows.mockResolvedValue(viewData([groupOf([issue])], [issue]));
    });

    it("wraps the titles of a list view that wraps them", async () => {
      mocked.viewsOf.mockResolvedValue([view({ wrapTitles: true })]);

      renderBody();

      expect(await screen.findByRole("row", { name: /NW-1042/ })).toHaveClass("list-row--wrap");
    });

    it("truncates the titles of a list view with wrapping turned off", async () => {
      mocked.viewsOf.mockResolvedValue([view({ wrapTitles: false })]);

      renderBody();

      expect(await screen.findByRole("row", { name: /NW-1042/ })).not.toHaveClass("list-row--wrap");
    });
  });

  describe("narrowing", () => {
    beforeEach(() => {
      mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
    });

    it.each(["list", "table"] as const)("asks for the rows narrowed to me in a %s view", async (layout) => {
      mocked.viewsOf.mockResolvedValue([view({ layout })]);
      useStore.getState().setNarrowing(narrowingKey(11, 101), { me: true });

      renderBody();

      await waitFor(() => expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, { me: true, text: "", epic: null }));
    });

    it("asks for the rows narrowed by the text of this view only", async () => {
      mocked.viewsOf.mockResolvedValue([view()]);
      useStore.getState().setNarrowing(narrowingKey(11, 101), { text: "pricing" });
      useStore.getState().setNarrowing(narrowingKey(11, 102), { me: true });

      renderBody();

      await waitFor(() => expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, { me: false, text: "pricing", epic: null }));
      expect(mocked.viewRows).not.toHaveBeenCalledWith(11, 101, expect.objectContaining({ me: true }));
    });
  });

  it("keeps the sync status when there are issues but no place to show", async () => {
    mocked.navCurrent.mockResolvedValue({ location: null, expanded: [] });
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));

    renderBody();

    expect(await screen.findByText("33 issues in 1 Space")).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });
});
