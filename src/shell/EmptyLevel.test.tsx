import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type SidebarTree } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { narrowingKey } from "../state/slices/narrowing";
import { resetStore, useStore } from "../state/store";
import { sidebarTree, space } from "../test/spaces";
import { listNode } from "../test/views";
import { EmptyLevel } from "./EmptyLevel";

const mocked = vi.mocked(api, true);

const SPACE_ID = 11;
const FILTER_ID = 21;
const VIEW_ID = 101;
const KEY = narrowingKey(FILTER_ID, VIEW_ID);

const treeWithFilter = (description = "Priority = High"): SidebarTree =>
  sidebarTree([space({ lists: [listNode({ id: FILTER_ID, name: "My open issues", description })] })]);

async function renderEmpty(listId: number, tree: SidebarTree) {
  mocked.sidebarTree.mockResolvedValue(tree);
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <EmptyLevel listId={listId} viewId={VIEW_ID} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
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
    it("describes the list's own filter", async () => {
      await renderEmpty(FILTER_ID, treeWithFilter());

      expect(await screen.findByRole("heading", { name: "Nothing in My open issues" })).toBeInTheDocument();
      expect(screen.getByText("No issues match Priority = High.")).toBeInTheDocument();
    });

    it.each<[string, string, boolean, string, string]>([
      ["Me on", "Priority = High", true, "", "No issues match Priority = High and Assignee = me."],
      ["a text", "Priority = High", false, "pricing", "No issues match Priority = High and the text “pricing”."],
      ["Me and a text", "Priority = High", true, "  pricing ", "No issues match Priority = High and Assignee = me and the text “pricing”."],
      ["a blank text", "Priority = High", false, "   ", "No issues match Priority = High."],
      ["no description and Me on", "", true, "", "No issues match Assignee = me."],
      ["no description and a text", "", false, "pricing", "No issues match the text “pricing”."],
      ["no description, no Me and no text", "", false, "", "No issues here yet."],
    ])("words the narrowing with %s", async (_name, description, me, text, sentence) => {
      useStore.getState().setNarrowing(KEY, { me, text });
      await renderEmpty(FILTER_ID, treeWithFilter(description));

      expect(await screen.findByText(sentence)).toBeInTheDocument();
    });

    it("asks the toolbar to open the Filter popover from Edit filter", async () => {
      await renderEmpty(FILTER_ID, treeWithFilter());
      const before = useStore.getState().filterRequests;

      await userEvent.click(await screen.findByRole("button", { name: "Edit filter" }));

      expect(useStore.getState().filterRequests).toBe(before + 1);
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

  describe("Clear Me filter", () => {
    it("is not offered while Me is off", async () => {
      await renderEmpty(FILTER_ID, treeWithFilter());

      await screen.findByRole("heading", { name: /^Nothing in / });
      expect(screen.queryByRole("button", { name: "Clear Me filter" })).not.toBeInTheDocument();
    });

    it("turns the Me narrowing off and drops it from the sentence", async () => {
      useStore.getState().setNarrowing(KEY, { me: true, text: "pricing" });
      await renderEmpty(FILTER_ID, treeWithFilter());

      await userEvent.click(await screen.findByRole("button", { name: "Clear Me filter" }));

      expect(useStore.getState().narrowings[KEY]).toMatchObject({ me: false, text: "pricing" });
      expect(screen.queryByRole("button", { name: "Clear Me filter" })).not.toBeInTheDocument();
      expect(screen.getByText("No issues match Priority = High and the text “pricing”.")).toBeInTheDocument();
    });

    it("is offered on a Space too", async () => {
      useStore.getState().setNarrowing(narrowingKey(SPACE_ID, VIEW_ID), { me: true });
      await renderEmpty(SPACE_ID, sidebarTree([space()]));

      expect(await screen.findByRole("button", { name: "Clear Me filter" })).toBeInTheDocument();
      expect(screen.getByText("No issues match Assignee = me.")).toBeInTheDocument();
    });
  });
});
