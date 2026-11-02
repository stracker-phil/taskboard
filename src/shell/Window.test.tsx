import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Nav } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { NAV_KEY } from "../queries/sidebar";
import { resetStore, useStore } from "../state/store";
import { issueDetail, related, viewData } from "../test/issues";
import { sidebarTree, space } from "../test/spaces";
import { nav as navOf, tab, tabbedNav } from "../test/nav";
import { listNode, view } from "../test/views";
import { Window } from "./Window";

const mocked = vi.mocked(api, true);

const nav = (listId: number, viewId: number): Nav => navOf({ listId, viewId });

/** Tab 1 on the Space's view, tab 2 on a filter list's view; `active` is the one shown. */
const twoTabs = (active: 1 | 2, tab1 = { listId: 11, viewId: 101 }): Nav => tabbedNav([tab(1, tab1), tab(2, { listId: 21, viewId: 201 })], active);

let client: QueryClient;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  client = createQueryClient();
  mocked.navCurrent.mockResolvedValue(nav(11, 101));
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space({ lists: [listNode({ id: 21, name: "My open issues" })] })]));
  mocked.viewsOf.mockResolvedValue([view()]);
  mocked.issueGet.mockResolvedValue(issueDetail({ id: "10042" }));
  mocked.issueRelated.mockResolvedValue(related());
  mocked.viewRows.mockResolvedValue(viewData([], []));
});

async function renderWindow(open: boolean, initial: Nav = nav(11, 101)) {
  mocked.navCurrent.mockResolvedValue(initial);
  resetStore(open ? { issueView: { origin: { listId: 11, viewId: 101 }, stack: ["10042"] } } : {});
  render(
    <QueryClientProvider client={client}>
      <Window sidebarHidden>
        <p>View body</p>
      </Window>
    </QueryClientProvider>,
  );
  await vi.waitFor(() => expect(client.getQueryData(NAV_KEY)).toEqual(initial));
}

/** Rust's answer changes with the cache, so a refetch (a view mounting its own `useNav`) never brings back an older place. */
const goTo = (location: Nav) => {
  mocked.navCurrent.mockResolvedValue(location);
  return act(async () => void client.setQueryData(NAV_KEY, location));
};

describe("Window", () => {
  describe("with the Issue view open", () => {
    it("keeps it while the place stays on the view it was opened from", async () => {
      await renderWindow(true);

      expect(await screen.findByRole("button", { name: /^Back to / })).toBeInTheDocument();
      expect(screen.queryByText("View body")).not.toBeInTheDocument();
      expect(useStore.getState().issueView).not.toBeNull();
    });

    it("keeps it when the same place is announced again", async () => {
      await renderWindow(true);
      await screen.findByRole("button", { name: /^Back to / });

      await goTo(nav(11, 101));

      expect(screen.getByRole("button", { name: /^Back to / })).toBeInTheDocument();
    });

    it.each([
      ["another list", nav(21, 201)],
      ["another view of the same list", nav(11, 102)],
    ])("closes it and shows the view body when the place moves to %s", async (_name, target) => {
      await renderWindow(true);
      await screen.findByRole("button", { name: /^Back to / });

      await goTo(target);

      expect(await screen.findByText("View body")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^Back to / })).not.toBeInTheDocument();
      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("tab strip", () => {
    it("is hidden while there is one tab", async () => {
      await renderWindow(false);
      await screen.findByText("View body");

      expect(screen.queryByRole("navigation", { name: "Tabs" })).not.toBeInTheDocument();
    });

    it("shows once there are two tabs, each named after its level, and drops away when one is left", async () => {
      await renderWindow(false);
      await screen.findByText("View body");

      await goTo(twoTabs(1));

      const strip = await screen.findByRole("navigation", { name: "Tabs" });
      await waitFor(() => expect(within(strip).getAllByRole("tab").map((t) => t.textContent)).toEqual(["Website relaunch", "My open issues"]));
      await goTo(nav(11, 101));
      await waitFor(() => expect(screen.queryByRole("navigation", { name: "Tabs" })).not.toBeInTheDocument());
    });
  });

  describe("with two tabs", () => {
    const backButton = () => screen.queryByRole("button", { name: /^Back to / });

    it("hides the Issue view when the other tab is shown and brings it back with the first tab", async () => {
      await renderWindow(true, twoTabs(1));
      await screen.findByRole("button", { name: /^Back to / });

      await goTo(twoTabs(2));

      expect(await screen.findByText("View body")).toBeInTheDocument();
      expect(backButton()).not.toBeInTheDocument();
      await goTo(twoTabs(1));
      expect(await screen.findByRole("button", { name: /^Back to / })).toBeInTheDocument();
      expect(screen.queryByText("View body")).not.toBeInTheDocument();
    });

    it("closes the Issue view when the shown tab moves to another place", async () => {
      await renderWindow(true, twoTabs(1));
      await screen.findByRole("button", { name: /^Back to / });

      await goTo(twoTabs(1, { listId: 11, viewId: 102 }));

      expect(await screen.findByText("View body")).toBeInTheDocument();
      expect(backButton()).not.toBeInTheDocument();
      expect(useStore.getState().issueView).toBeNull();
    });

    describe("drafts", () => {
      const saveView = () => screen.queryByRole("button", { name: "Save View" });

      const SECOND_VIEW = { listId: 11, viewId: 102 };
      /** Both tabs on the Space: view 101 (tab 1) and the Board that saved Me on (tab 2). */
      const sameSpace = (active: 1 | 2) => tabbedNav([tab(1, { listId: 11, viewId: 101 }), tab(2, SECOND_VIEW)], active);

      beforeEach(() => {
        mocked.viewsOf.mockResolvedValue([view(), view({ id: 102, name: "Board", onlyMine: true })]);
      });

      async function draftMeOnFirstTab() {
        await renderWindow(false, sameSpace(1));
        await userEvent.click(await screen.findByRole("button", { name: "Me", pressed: false }));
        await screen.findByRole("button", { name: "Save View" });
      }

      it("of a background tab's view survive a switch to the other tab and back", async () => {
        await draftMeOnFirstTab();

        await goTo(sameSpace(2));
        expect(await screen.findByRole("button", { name: "Me", pressed: true })).toBeInTheDocument();
        expect(saveView()).not.toBeInTheDocument();
        await goTo(sameSpace(1));

        expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Me", pressed: true })).toBeInTheDocument();
      });

      it("are dropped once the tab that held the view is closed", async () => {
        await draftMeOnFirstTab();
        await goTo(sameSpace(2));

        await goTo(navOf(SECOND_VIEW, { tabs: [tab(2, SECOND_VIEW)], activeTab: 2 }));

        await waitFor(() => expect(useStore.getState().narrowings["11:101"]).toBeUndefined());
      });
    });
  });

  describe("level chrome", () => {
    const tree = () => ({ ...sidebarTree([space()]), home: [listNode({ id: 3, kind: "pinned", name: "Pinned", count: null })] });

    it("shows the view tabs on a Space", async () => {
      mocked.sidebarTree.mockResolvedValue(tree());
      await renderWindow(false);

      expect(await screen.findByRole("tablist", { name: "Views" })).toBeInTheDocument();
    });

    it("shows no view tabs and no toolbar on the Pinned level", async () => {
      mocked.sidebarTree.mockResolvedValue(tree());
      mocked.navCurrent.mockResolvedValue(nav(3, 0));
      mocked.viewsOf.mockResolvedValue([view({ id: 0, listId: 3 })]);
      render(
        <QueryClientProvider client={client}>
          <Window sidebarHidden>
            <p>View body</p>
          </Window>
        </QueryClientProvider>,
      );

      expect(await screen.findByRole("heading", { name: "Pinned" })).toBeInTheDocument();
      expect(screen.queryByRole("tablist", { name: "Views" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^Filter/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "More toolbar items" })).not.toBeInTheDocument();
    });
  });

  describe("unsaved view drafts", () => {
    const saveView = () => screen.queryByRole("button", { name: "Save View" });

    beforeEach(() => {
      mocked.viewsOf.mockImplementation(async (listId) =>
        listId === 11 ? [view(), view({ id: 102, name: "Board" })] : [view({ id: 201, listId: 21, onlyMine: true })],
      );
    });

    async function draftMe() {
      await renderWindow(false);
      await userEvent.click(await screen.findByRole("button", { name: "Me", pressed: false }));
      return screen.findByRole("button", { name: "Save View" });
    }

    it.each([
      ["another view of the same list", nav(11, 102)],
      ["another list", nav(21, 201)],
    ])("are dropped when the place moves to %s", async (_name, target) => {
      await draftMe();

      await goTo(target);

      await waitFor(() => expect(saveView()).not.toBeInTheDocument());
      expect(useStore.getState().narrowings["11:101"]).toBeUndefined();
    });

    it("are gone when the owner comes back to the view", async () => {
      await draftMe();
      await goTo(nav(11, 102));
      await waitFor(() => expect(saveView()).not.toBeInTheDocument());

      await goTo(nav(11, 101));

      expect(await screen.findByRole("button", { name: "Me", pressed: false })).toBeInTheDocument();
      expect(saveView()).not.toBeInTheDocument();
    });

    it("take the text filter with them", async () => {
      await renderWindow(false);
      await userEvent.type(await screen.findByRole("textbox", { name: "Filter this view" }), "pricing");
      await waitFor(() => expect(useStore.getState().narrowings["11:101"]?.text).toBe("pricing"));

      await goTo(nav(11, 102));
      await waitFor(() => expect(useStore.getState().narrowings["11:101"]).toBeUndefined());
      await goTo(nav(11, 101));

      expect(await screen.findByRole("textbox", { name: "Filter this view" })).toHaveValue("");
    });

    it("stay when the same place is announced again", async () => {
      await draftMe();

      await goTo(nav(11, 101));

      expect(saveView()).toBeInTheDocument();
    });

    it("stay while an issue is open over the view, and show again when it closes", async () => {
      await draftMe();

      act(() => useStore.getState().openIssue("10042", { listId: 11, viewId: 101 }));
      await userEvent.click(await screen.findByRole("button", { name: /^Back to / }));

      expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Me", pressed: true })).toBeInTheDocument();
    });

    it("do not carry over to a view that saved the Me chip: it opens with Me on", async () => {
      await renderWindow(false);
      await screen.findByRole("button", { name: "Me", pressed: false });

      await goTo(nav(21, 201));

      expect(await screen.findByRole("button", { name: "Me", pressed: true })).toBeInTheDocument();
      expect(saveView()).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Me", pressed: false })).not.toBeInTheDocument();
    });
  });

  describe("with a preview open", () => {
    it("lays the preview over the content panel, outside the view body", async () => {
      resetStore({ places: { "11:101": { scrollTop: 0, toggled: {}, collapsedParents: [], selectedId: "10042", peekOpen: true } } });
      render(
        <QueryClientProvider client={client}>
          <Window sidebarHidden>
            <p>View body</p>
          </Window>
        </QueryClientProvider>,
      );

      const preview = await screen.findByRole("complementary", { name: "Issue preview" });
      const body = screen.getByText("View body").parentElement!;

      expect(body).toHaveClass("content-panel__body");
      expect(body).not.toContainElement(preview);
      expect(preview.closest("main")).toHaveClass("content-panel");
      expect(preview.closest("main")).toContainElement(body);
    });
  });

  describe("bookmarks bar", () => {
    const bar = () => screen.queryByRole("navigation", { name: "Bookmarks" });

    function renderShell(sidebarHidden: boolean, bookmarksBar: boolean) {
      mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: false, bookmarksBar });
      render(
        <QueryClientProvider client={client}>
          <Window sidebarHidden={sidebarHidden}>
            <p>View body</p>
          </Window>
        </QueryClientProvider>,
      );
    }

    it.each([
      ["hidden", true],
      ["shown", false],
    ])("shows the bar above the panel while it is on and the sidebar is %s", async (_state, sidebarHidden) => {
      renderShell(sidebarHidden, true);

      const found = await screen.findByRole("navigation", { name: "Bookmarks" });
      expect(found.compareDocumentPosition(screen.getByRole("main")) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it.each([
      ["hidden", true],
      ["shown", false],
    ])("shows no bar while it is off and the sidebar is %s", async (_state, sidebarHidden) => {
      renderShell(sidebarHidden, false);
      await screen.findByText("View body");

      expect(bar()).not.toBeInTheDocument();
    });
  });

  describe("without an Issue view", () => {
    it("shows the view body and stays that way when the place changes", async () => {
      await renderWindow(false);
      expect(await screen.findByText("View body")).toBeInTheDocument();

      await goTo(nav(21, 201));

      expect(screen.getByText("View body")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^Back to / })).not.toBeInTheDocument();
    });
  });
});
