import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Condition, type ListConditions, type SidebarTree } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { narrowingKey } from "../state/slices/narrowing";
import { resetStore, useStore } from "../state/store";
import { sidebarTree, space } from "../test/spaces";
import { listNode, view } from "../test/views";
import { EmptyLevel } from "./EmptyLevel";

const mocked = vi.mocked(api, true);

const SPACE_ID = 11;
const FILTER_ID = 21;
const VIEW_ID = 101;
const KEY = narrowingKey(FILTER_ID, VIEW_ID);
const SPACE_KEY = narrowingKey(SPACE_ID, VIEW_ID);

const BLOCKED: Condition = { field: "status", op: "is", value: "Blocked" };
const securitySpace = () => space({ name: "Security" });
const treeWithFilter = (): SidebarTree => sidebarTree([space({ lists: [listNode({ id: FILTER_ID, name: "Security" })] })]);

const conditions = (patch: Partial<ListConditions> = {}): ListConditions => ({
  kind: "space",
  locked: [],
  own: [],
  error: null,
  view: [],
  viewError: null,
  projectFixed: false,
  fixed: null,
  ...patch,
});

async function renderEmpty(listId: number, tree: SidebarTree) {
  mocked.sidebarTree.mockResolvedValue(tree);
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <EmptyLevel listId={listId} viewId={VIEW_ID} />
    </QueryClientProvider>,
  );
}

const renderSpace = () => renderEmpty(SPACE_ID, sidebarTree([securitySpace()]));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: false, bookmarksBar: false });
  mocked.viewsOf.mockImplementation(async (listId) => [view({ id: VIEW_ID, listId, name: "Release" })]);
  mocked.listConditions.mockResolvedValue(conditions());
  mocked.listMatchCount.mockResolvedValue({ open: 3, total: 3 });
});

describe("EmptyLevel", () => {
  it("names a Space and says there are no issues yet", async () => {
    await renderEmpty(SPACE_ID, sidebarTree([space()]));

    expect(await screen.findByRole("heading", { name: "Nothing in Website relaunch" })).toBeInTheDocument();
    expect(screen.getByText("No issues here yet.")).toBeInTheDocument();
  });

  it("renders nothing when the level is not in the sidebar tree", async () => {
    const { container } = await renderEmpty(404, sidebarTree([space()]));

    await waitFor(() => expect(mocked.sidebarTree).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });

  describe("on a filter list", () => {
    beforeEach(() => {
      mocked.listConditions.mockResolvedValue(conditions({ kind: "list", own: [BLOCKED] }));
    });

    it("says everything is done and offers Edit filter, without naming the filter values", async () => {
      await renderEmpty(FILTER_ID, treeWithFilter());

      expect(await screen.findByRole("heading", { name: "Nothing in Security" })).toBeInTheDocument();
      expect(screen.getByText("Looks like everything is done here.")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Edit filter" })).toBeInTheDocument();
      expect(screen.queryByText(/Blocked/)).not.toBeInTheDocument();
    });

    it("asks the toolbar to open the Filter popover from Edit filter", async () => {
      await renderEmpty(FILTER_ID, treeWithFilter());
      const before = useStore.getState().filterRequests;

      await userEvent.click(await screen.findByRole("button", { name: "Edit filter" }));

      expect(useStore.getState().filterRequests).toBe(before + 1);
    });
  });

  describe("on a Space view with its own condition", () => {
    beforeEach(() => {
      mocked.listConditions.mockResolvedValue(conditions({ view: [BLOCKED] }));
    });

    it("adds the view's name to the title and offers Edit filter", async () => {
      await renderSpace();

      expect(await screen.findByRole("heading", { name: "Nothing in Security → Release" })).toBeInTheDocument();
      expect(screen.getByText("Looks like everything is done here.")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Edit filter" })).toBeInTheDocument();
      expect(screen.queryByText(/Blocked/)).not.toBeInTheDocument();
    });

    it("counts a drafted view condition as the view's own", async () => {
      mocked.listConditions.mockResolvedValue(conditions());
      useStore.getState().setNarrowing(SPACE_KEY, { view: [BLOCKED] });
      await renderSpace();

      expect(await screen.findByRole("heading", { name: "Nothing in Security → Release" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Edit filter" })).toBeInTheDocument();
    });
  });

  describe("on a Space without narrowing", () => {
    it("says there are no issues yet and offers no buttons", async () => {
      await renderSpace();

      expect(await screen.findByRole("heading", { name: "Nothing in Security" })).toBeInTheDocument();
      expect(screen.getByText("No issues here yet.")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Edit filter" })).not.toBeInTheDocument();
      expect(screen.queryAllByRole("button")).toHaveLength(0);
    });
  });

  describe("Edit filter", () => {
    it("is offered on a filter list of the Home section", async () => {
      await renderEmpty(3, { ...sidebarTree([]), home: [listNode({ id: 3, name: "Due this week", description: "Due < 7d" })] });

      expect(await screen.findByRole("button", { name: "Edit filter" })).toBeInTheDocument();
    });

    it.each<[string, SidebarTree, number]>([
      ["a Space", sidebarTree([space()]), SPACE_ID],
      ["Everything", sidebarTree([]), 1],
    ])("is not offered on %s", async (_name, tree, listId) => {
      await renderEmpty(listId, tree);

      await screen.findByRole("heading", { name: /^Nothing in / });
      expect(screen.queryByRole("button", { name: "Edit filter" })).not.toBeInTheDocument();
    });
  });

  describe("Me mode", () => {
    beforeEach(() => {
      useStore.getState().setNarrowing(SPACE_KEY, { me: true });
    });

    it("adds 'for you' when Me is what empties the view, and offers Clear Me Mode", async () => {
      await renderSpace();

      expect(await screen.findByRole("heading", { name: "Nothing in Security for you" })).toBeInTheDocument();
      expect(screen.getByText("Looks like everything is done here.")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Clear Me Mode" })).toBeInTheDocument();
    });

    it("asks for the count with the fixed, list and view conditions and Me off", async () => {
      const fixed: Condition = { field: "project", op: "is", value: "NW" };
      const own: Condition = { field: "priority", op: "is", value: "High" };
      mocked.listConditions.mockResolvedValue(conditions({ fixed, own: [own], view: [BLOCKED] }));
      await renderSpace();

      await screen.findByRole("heading", { name: "Nothing in Security for you" });
      expect(mocked.listMatchCount).toHaveBeenCalledWith(SPACE_ID, { conditions: [fixed, own, BLOCKED] }, false);
    });

    it("falls back to the view's name when nothing would show even without Me", async () => {
      mocked.listMatchCount.mockResolvedValue({ open: 0, total: 0 });
      mocked.listConditions.mockResolvedValue(conditions({ view: [BLOCKED] }));
      await renderSpace();

      expect(await screen.findByRole("heading", { name: "Nothing in Security → Release" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Clear Me Mode" })).toBeInTheDocument();
    });

    it("shows a plain title while the count is loading", async () => {
      mocked.listMatchCount.mockReturnValue(new Promise(() => {}));
      await renderSpace();

      await screen.findByText("Looks like everything is done here.");
      expect(screen.getByRole("heading", { name: "Nothing in Security" })).toBeInTheDocument();
    });

    it("skips 'for you' while a text filter narrows too", async () => {
      useStore.getState().setNarrowing(SPACE_KEY, { me: true, text: "pricing" });
      await renderSpace();

      await screen.findByRole("button", { name: "Clear search" });
      await waitFor(() => expect(mocked.listConditions).toHaveBeenCalled());
      expect(screen.getByRole("heading", { name: "Nothing in Security" })).toBeInTheDocument();
    });

    it("skips 'for you' while Unread only is on", async () => {
      mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: true, bookmarksBar: false });
      await renderSpace();

      await screen.findByRole("button", { name: "Turn off Unread only" });
      await waitFor(() => expect(mocked.listConditions).toHaveBeenCalled());
      expect(screen.getByRole("heading", { name: "Nothing in Security" })).toBeInTheDocument();
    });

    it("turns the Me narrowing off and drops it from the title", async () => {
      await renderSpace();

      await userEvent.click(await screen.findByRole("button", { name: "Clear Me Mode" }));

      expect(useStore.getState().narrowings[SPACE_KEY]).toMatchObject({ me: false });
      expect(screen.queryByRole("button", { name: "Clear Me Mode" })).not.toBeInTheDocument();
      expect(screen.getByRole("heading", { name: "Nothing in Security" })).toBeInTheDocument();
      expect(screen.getByText("No issues here yet.")).toBeInTheDocument();
    });

    it("is not offered while Me is off", async () => {
      useStore.getState().setNarrowing(SPACE_KEY, { me: false });
      await renderSpace();

      await screen.findByRole("heading", { name: /^Nothing in / });
      expect(screen.queryByRole("button", { name: "Clear Me Mode" })).not.toBeInTheDocument();
    });

    it("turns off the Me chip the view saved as a draft, without saving the view", async () => {
      resetStore();
      mocked.viewsOf.mockResolvedValue([view({ id: VIEW_ID, listId: SPACE_ID, name: "Release", onlyMine: true })]);
      await renderSpace();

      await userEvent.click(await screen.findByRole("button", { name: "Clear Me Mode" }));

      await waitFor(() => expect(screen.queryByRole("button", { name: "Clear Me Mode" })).not.toBeInTheDocument());
      expect(mocked.viewSetOnlyMine).not.toHaveBeenCalled();
    });

    it("also switches Me off in every view when it applies everywhere", async () => {
      useStore.getState().setNarrowing(SPACE_KEY, { me: false });
      mocked.modesGet.mockResolvedValue({ meEverywhere: true, unreadOnly: false, bookmarksBar: false });
      await renderSpace();

      await userEvent.click(await screen.findByRole("button", { name: "Clear Me Mode" }));

      expect(mocked.modeSet).toHaveBeenCalledWith("meEverywhere", false);
    });

    it("leaves the everywhere mode alone when Me is only on in this view", async () => {
      await renderSpace();

      await userEvent.click(await screen.findByRole("button", { name: "Clear Me Mode" }));

      expect(mocked.modeSet).not.toHaveBeenCalled();
    });
  });

  describe("Clear search", () => {
    it("is not offered without a text", async () => {
      useStore.getState().setNarrowing(KEY, { text: "   " });
      await renderEmpty(FILTER_ID, treeWithFilter());

      await screen.findByRole("heading", { name: /^Nothing in / });
      expect(screen.queryByRole("button", { name: "Clear search" })).not.toBeInTheDocument();
    });

    it("empties the text and keeps the button list in step", async () => {
      useStore.getState().setNarrowing(SPACE_KEY, { text: "pricing" });
      await renderSpace();
      expect(await screen.findByText("Looks like everything is done here.")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Clear search" }));

      expect(useStore.getState().narrowings[SPACE_KEY]).toMatchObject({ text: "" });
      expect(screen.queryByRole("button", { name: "Clear search" })).not.toBeInTheDocument();
      expect(screen.getByText("No issues here yet.")).toBeInTheDocument();
    });
  });

  describe("Show anyone", () => {
    it("is not offered without a person chip", async () => {
      await renderSpace();

      await screen.findByRole("heading", { name: /^Nothing in / });
      expect(screen.queryByRole("button", { name: "Show anyone" })).not.toBeInTheDocument();
    });

    it("names the person by first name and clears the chip", async () => {
      mocked.viewPeople.mockResolvedValue([{ accountId: "user-7", name: "Dana Example", initials: "DE", color: 1 }]);
      useStore.getState().setNarrowing(SPACE_KEY, { person: "user-7" });
      await renderSpace();
      expect(await screen.findByRole("heading", { name: "Nothing in Security for Dana" })).toBeInTheDocument();
      expect(screen.getByText("Looks like everything is done here.")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Show anyone" }));

      expect(useStore.getState().narrowings[SPACE_KEY]?.person).toBeNull();
      expect(screen.queryByRole("button", { name: "Show anyone" })).not.toBeInTheDocument();
      expect(screen.getByRole("heading", { name: "Nothing in Security" })).toBeInTheDocument();
      expect(screen.getByText("No issues here yet.")).toBeInTheDocument();
    });
  });

  describe("Turn off Unread only", () => {
    it("is not offered while Unread only is off", async () => {
      await renderSpace();

      await screen.findByRole("heading", { name: /^Nothing in / });
      expect(screen.queryByRole("button", { name: "Turn off Unread only" })).not.toBeInTheDocument();
    });

    it("counts as narrowing and turns the mode off", async () => {
      mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: true, bookmarksBar: false });
      await renderSpace();
      expect(await screen.findByText("Looks like everything is done here.")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Turn off Unread only" }));

      expect(mocked.modeSet).toHaveBeenCalledWith("unreadOnly", false);
    });
  });

  describe("with several narrowings", () => {
    it("offers one button for each", async () => {
      mocked.listConditions.mockResolvedValue(conditions({ kind: "list", own: [BLOCKED] }));
      mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: true, bookmarksBar: false });
      useStore.getState().setNarrowing(KEY, { me: true, text: "pricing" });
      await renderEmpty(FILTER_ID, treeWithFilter());

      await screen.findByRole("button", { name: "Turn off Unread only" });
      expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual([
        "Edit filter",
        "Clear search",
        "Clear Me Mode",
        "Turn off Unread only",
      ]);
    });
  });
});
