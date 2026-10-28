import { render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { narrowingKey } from "../state/slices/narrowing";
import { resetStore, useStore } from "../state/store";
import { boardColumn, boardData } from "../test/board";
import { groupOf, row, viewData } from "../test/issues";
import { projectCounts, syncOverview } from "../test/jira";
import { pinItem, pins } from "../test/pins";
import { lumenSpace, sidebarTree, space } from "../test/spaces";
import { listNode, view } from "../test/views";
import { MainBody } from "./MainBody";

const mocked = vi.mocked(api, true);

type Restorable = { mockRestore: () => void };
let spies: Restorable[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.pinsList.mockResolvedValue(pins());
  mocked.viewsOf.mockResolvedValue([]);
  mocked.placeGet.mockResolvedValue(null);
  mocked.sidebarTree.mockResolvedValue(sidebarTree([]));
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
    mocked.viewsOf.mockResolvedValue([view({ layout: "list" })]);
    mocked.viewRows.mockResolvedValue(viewData([groupOf([issue])], [issue]));

    renderBody();

    expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
    expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, expect.anything());
  });

  it("draws nothing while the views of the current place are still loading", async () => {
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
    mocked.viewsOf.mockReturnValue(new Promise(() => {}));

    const { container } = render(
      <QueryClientProvider client={createQueryClient()}>
        <MainBody />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(mocked.viewsOf).toHaveBeenCalledWith(11));
    expect(container).toBeEmptyDOMElement();
    expect(mocked.viewRows).not.toHaveBeenCalled();
    expect(mocked.viewBoard).not.toHaveBeenCalled();
  });

  describe("on the Pinned level", () => {
    beforeEach(() => {
      mocked.navCurrent.mockResolvedValue({ location: { listId: 3, viewId: 0 }, expanded: [] });
      mocked.sidebarTree.mockResolvedValue({ ...sidebarTree([]), home: [listNode({ id: 3, kind: "pinned", name: "Pinned", count: null })] });
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
    });

    it("shows the pins instead of a list", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ reason: "Needs a decision" })]));

      renderBody();

      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.getByText("Needs a decision")).toBeInTheDocument();
      expect(screen.queryByRole("grid")).not.toBeInTheDocument();
      expect(mocked.viewRows).not.toHaveBeenCalled();
    });

    it("says nothing is pinned when there are no pins", async () => {
      renderBody();

      expect(await screen.findByText(/^Nothing pinned/)).toBeInTheDocument();
    });
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

    it("shows the list for a view with the list layout", async () => {
      mocked.viewsOf.mockResolvedValue([view({ layout: "list" })]);

      renderBody();

      expect(await screen.findByRole("button", { name: /^(Collapse|Expand) / })).toBeInTheDocument();
      expect(screen.getByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.queryByText("Key")).not.toBeInTheDocument();
      expect(screen.queryByRole("list", { name: "Board" })).not.toBeInTheDocument();
    });

    it("shows the board for a view with the board layout", async () => {
      const issue = row({ summary: "Redesign the pricing page" });
      mocked.viewsOf.mockResolvedValue([view({ layout: "board" })]);
      mocked.viewBoard.mockResolvedValue(boardData([boardColumn({ ids: [issue.id] })], [issue]));

      renderBody();

      expect(await screen.findByRole("list", { name: "Board" })).toBeInTheDocument();
      expect(screen.getByRole("article", { name: /Redesign the pricing page/ })).toBeInTheDocument();
      expect(mocked.viewRows).not.toHaveBeenCalled();
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

  describe("saved place", () => {
    const SAVED = { scrollTop: 0, toggled: {}, selectedId: null, peekOpen: false, onlyMine: true, text: "pricing" };

    beforeEach(() => {
      mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
      mocked.placeGet.mockResolvedValue(SAVED);
    });

    it.each(["list", "board", "table"] as const)("loads the place of a %s view", async (layout) => {
      mocked.viewsOf.mockResolvedValue([view({ layout })]);

      renderBody();

      await waitFor(() => expect(useStore.getState().places[narrowingKey(11, 101)]).toEqual(SAVED));
      expect(mocked.placeGet).toHaveBeenCalledWith(11, 101);
    });

    it("narrows the rows by the Me chip and text saved with the view", async () => {
      mocked.viewsOf.mockResolvedValue([view({ layout: "table" })]);

      renderBody();

      await waitFor(() => expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, { me: true, text: "pricing", epic: null }));
    });
  });

  describe("during the first sync", () => {
    const RUNNING = { state: "running", job: "full" } as const;
    const FINISHED = projectCounts({ lastFullAt: "2026-10-27T09:00:00Z" });
    const UNFINISHED = projectCounts({ id: "10002", key: "LUM", name: "Lumen iOS app", cached: 5, lastFullAt: null });
    const at = (listId: number) => mocked.navCurrent.mockResolvedValue({ location: { listId, viewId: 101 }, expanded: [] });

    beforeEach(() => {
      const issue = row({ summary: "Redesign the pricing page" });
      mocked.sidebarTree.mockResolvedValue(sidebarTree([space({ lists: [listNode({ id: 21 })] }), lumenSpace()]));
      mocked.viewsOf.mockResolvedValue([view()]);
      mocked.viewRows.mockResolvedValue(viewData([groupOf([issue])], [issue]));
    });

    it("shows the progress instead of the view while the current Space is still loading", async () => {
      at(11);
      mocked.syncOverview.mockResolvedValue(syncOverview({ state: RUNNING, projects: [projectCounts({ lastFullAt: null })] }));

      renderBody();

      expect(await screen.findByText("Loading your Jira issues")).toBeInTheDocument();
      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
    });

    it("shows the progress for a filter list of a Space that is still loading", async () => {
      at(21);
      mocked.syncOverview.mockResolvedValue(syncOverview({ state: RUNNING, projects: [projectCounts({ lastFullAt: null })] }));

      renderBody();

      expect(await screen.findByText("Loading your Jira issues")).toBeInTheDocument();
    });

    it("shows the view of a finished Space while another one is still loading", async () => {
      at(11);
      mocked.syncOverview.mockResolvedValue(syncOverview({ state: RUNNING, projects: [FINISHED, UNFINISHED] }));

      renderBody();

      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.queryByText("Loading your Jira issues")).not.toBeInTheDocument();
    });

    it("shows the progress when the current place is the Space that is still loading", async () => {
      at(12);
      mocked.syncOverview.mockResolvedValue(syncOverview({ state: RUNNING, projects: [FINISHED, UNFINISHED] }));

      renderBody();

      expect(await screen.findByText("Loading your Jira issues")).toBeInTheDocument();
    });

    it("shows the view once the sync is not running, even for a Space never fully synced", async () => {
      at(11);
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts({ lastFullAt: null })] }));

      renderBody();

      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
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
