import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ListNode, type MenuEntry, type Nav, type SidebarTree, type SpaceNode } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { SIDEBAR_KEY } from "../../queries/sidebar";
import { resetStore, useStore } from "../../state/store";
import { listTarget, pinItem, pins } from "../../test/pins";
import { lumenSpace, space } from "../../test/spaces";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { Toasts } from "../../components/Toasts";
import { Sidebar } from "./Sidebar";

const mocked = vi.mocked(api, true);

// jsdom may lack PointerEvent; the stub keeps the mouse fields (clientY, buttons) that the drag reads.
window.PointerEvent ??= class extends MouseEvent {
  pointerId = 1;
} as unknown as typeof PointerEvent;

const list = (id: number, name: string, patch: Partial<ListNode> = {}): ListNode => ({
  id,
  kind: "list",
  name,
  count: null,
  ownConditions: 0,
  description: "",
  error: null,
  ...patch,
});

const SPRINT = list(21, "Current sprint", { count: 7 });
const EPICS = list(22, "Epics", { count: 3 });
const BUGS = list(23, "Open bugs", { count: 3 });
const RELEASE = list(24, "Release 2.4.0", { count: 4 });

function treeWith(patch: { nw?: Partial<SpaceNode>; nws?: ListNode[] } = {}): SidebarTree {
  return {
    everything: list(1, "Everything", { kind: "everything", count: 42 }),
    home: [list(2, "Recently changed", { kind: "recent", count: 8 }), list(3, "Pinned", { kind: "pinned", count: null })],
    pinnedLists: [],
    spaces: [
      space({ icon: "R", color: "3", client: "Northwind", lists: [SPRINT, EPICS], ...patch.nw }),
      space({ listId: 13, projectId: "10003", key: "NWS", name: "Northwind support", projectName: "Northwind support", icon: null, color: null, openCount: 5, lists: patch.nws ?? [BUGS] }),
      lumenSpace({ openCount: 9, lists: [RELEASE] }),
    ],
  };
}

const NW_ID = 11;
const NWS_ID = 13;
const LUM_ID = 12;

const navAt = (listId: number | null, expanded: number[] = []): Nav => ({ location: listId === null ? null : { listId, viewId: 0 }, expanded });

async function renderSidebar(tree: SidebarTree = treeWith(), nav: Nav = navAt(null)) {
  mocked.sidebarTree.mockResolvedValue(tree);
  mocked.navCurrent.mockResolvedValue(nav);
  const client = createQueryClient();
  render(
    <QueryClientProvider client={client}>
      <Sidebar />
      <Toasts />
      <ConfirmHost />
    </QueryClientProvider>,
  );
  await screen.findByRole("navigation", { name: "Spaces" });
  return client;
}

// A row's label and count are adjacent spans, so its accessible name is the label followed by the count, no space.
// Recently changed shows its count as a badge labelled "<n> unseen".
const named = (label: string) => new RegExp(`^${label}\\s*(\\d+( unseen)?)?$`);
const spaceButton = (label: string) => screen.getByRole("button", { name: named(label) });

function menuEntry(text: string) {
  const entries: MenuEntry[] = mocked.popupMenu.mock.calls.at(-1)?.[0] ?? [];
  return entries.find((e): e is Exclude<MenuEntry, "separator"> => e !== "separator" && e.text === text);
}

async function chooseFromMenu(text: string) {
  const entry = menuEntry(text);
  expect(entry, `menu entry "${text}"`).toBeDefined();
  await act(async () => entry?.run?.());
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.pinsList.mockResolvedValue(pins());
  mocked.listCreate.mockResolvedValue(30);
  mocked.listRename.mockResolvedValue(undefined);
  mocked.listDuplicate.mockResolvedValue(31);
  mocked.listDelete.mockResolvedValue(undefined);
  mocked.listMove.mockResolvedValue(undefined);
  mocked.spaceMove.mockResolvedValue(undefined);
  mocked.spaceUpdate.mockResolvedValue(undefined);
});

describe("Sidebar", () => {
  describe("Search", () => {
    it("opens the command palette", async () => {
      await renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: /^Search/ }));

      expect(useStore.getState().paletteOpen).toBe(true);
    });
  });

  describe("Home rows", () => {
    it("show each row's count", async () => {
      await renderSidebar();
      const home = screen.getByRole("navigation", { name: "Home" });

      expect(within(within(home).getByRole("button", { name: named("Everything") })).getByText("42")).toBeInTheDocument();
      expect(within(within(home).getByRole("button", { name: named("Recently changed") })).getByText("8")).toBeInTheDocument();
    });

    it("show Recently changed's count as an unseen badge", async () => {
      await renderSidebar();

      expect(within(screen.getByRole("navigation", { name: "Home" })).getByLabelText("8 unseen")).toHaveTextContent("8");
    });

    it.each([0, null])("show no badge and no count on Recently changed when its count is %s", async (count) => {
      const tree = treeWith();
      tree.home = [list(2, "Recently changed", { kind: "recent", count }), list(3, "Pinned", { kind: "pinned", count: null })];
      await renderSidebar(tree);

      const row = within(screen.getByRole("navigation", { name: "Home" })).getByRole("button", { name: "Recently changed" });
      expect(row).toHaveTextContent(/^Recently changed$/);
      expect(screen.queryByLabelText(/unseen/)).not.toBeInTheDocument();
    });

    it("show no count when it is null", async () => {
      await renderSidebar();

      expect(within(screen.getByRole("navigation", { name: "Home" })).getByRole("button", { name: "Pinned" })).toHaveTextContent(/^Pinned$/);
    });

    it("open the row's level when clicked", async () => {
      await renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: named("Recently changed") }));

      expect(mocked.navGo).toHaveBeenCalledWith(2, null);
    });

    it("mark the current one", async () => {
      await renderSidebar(treeWith(), navAt(1));

      expect(await screen.findByRole("button", { name: named("Everything") })).toHaveAttribute("aria-current", "page");
      expect(screen.getByRole("button", { name: "Pinned" })).not.toHaveAttribute("aria-current");
    });
  });

  describe("Spaces", () => {
    it("show their tile letter, name and open count", async () => {
      await renderSidebar();

      expect(within(spaceButton("Website relaunch")).getByText("21")).toBeInTheDocument();
      expect(within(spaceButton("Northwind support")).getByText("5")).toBeInTheDocument();
      expect(within(spaceButton("Lumen iOS app")).getByText("9")).toBeInTheDocument();
      expect(screen.getByText("R")).toBeInTheDocument();
      expect(screen.getByText("N")).toBeInTheDocument();
      expect(screen.getByText("L")).toBeInTheDocument();
    });

    it("open when clicked", async () => {
      await renderSidebar();

      await userEvent.click(spaceButton("Northwind support"));

      expect(mocked.navGo).toHaveBeenCalledWith(NWS_ID, null);
    });

    it("mark the current Space with aria-current", async () => {
      await renderSidebar(treeWith(), navAt(LUM_ID));

      expect(await screen.findByRole("button", { name: named("Lumen iOS app") })).toHaveAttribute("aria-current", "page");
      expect(spaceButton("Website relaunch")).not.toHaveAttribute("aria-current");
    });

    it("keep the lists collapsed unless the Space is in the expanded set", async () => {
      await renderSidebar(treeWith(), navAt(null, [LUM_ID]));

      expect(await screen.findByText("Release 2.4.0")).toBeInTheDocument();
      expect(screen.queryByText("Current sprint")).not.toBeInTheDocument();
      expect(screen.queryByText("Open bugs")).not.toBeInTheDocument();
    });

    it("show the lists of the Space holding the current list and mark the Space as holding it", async () => {
      await renderSidebar(treeWith(), navAt(SPRINT.id));

      const currentSprint = await screen.findByRole("button", { name: named("Current sprint") });
      expect(currentSprint).toHaveAttribute("aria-current", "page");
      expect(screen.getByRole("button", { name: named("Epics") })).not.toHaveAttribute("aria-current");
      expect(spaceButton("Website relaunch").parentElement).toHaveAttribute("data-state", "holds-active");
      expect(spaceButton("Website relaunch")).not.toHaveAttribute("aria-current");
      expect(spaceButton("Northwind support").parentElement).not.toHaveAttribute("data-state");
    });

    it("open a list when clicked", async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));

      await userEvent.click(await screen.findByRole("button", { name: named("Epics") }));

      expect(mocked.navGo).toHaveBeenCalledWith(EPICS.id, null);
    });

    it("show the reason instead of a count for a list whose filter fails", async () => {
      await renderSidebar(treeWith({ nws: [list(23, "Open bugs", { count: 3, error: "Unknown field 'severity'" })] }), navAt(null, [NWS_ID]));

      expect(await screen.findByRole("img", { name: "Unknown field 'severity'" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^Open bugs/ })).toHaveTextContent(/^Open bugs$/);
    });
  });

  describe("chevron", () => {
    it("expands a collapsed Space, keeping the other expanded Spaces", async () => {
      await renderSidebar(treeWith(), navAt(null, [LUM_ID]));
      await screen.findByText("Release 2.4.0");

      const chevron = screen.getByRole("button", { name: "Expand Website relaunch" });
      expect(chevron).toHaveAttribute("aria-expanded", "false");
      await userEvent.click(chevron);

      expect(mocked.navSetExpanded).toHaveBeenCalledWith([LUM_ID, NW_ID]);
    });

    it("collapses an expanded Space without it", async () => {
      await renderSidebar(treeWith(), navAt(null, [LUM_ID, NW_ID]));

      const chevron = await screen.findByRole("button", { name: "Collapse Website relaunch" });
      expect(chevron).toHaveAttribute("aria-expanded", "true");
      await userEvent.click(chevron);

      expect(mocked.navSetExpanded).toHaveBeenCalledWith([LUM_ID]);
    });

    it("expands only that Space on option-click", async () => {
      await renderSidebar(treeWith(), navAt(null, [LUM_ID]));
      await screen.findByText("Release 2.4.0");

      fireEvent.click(screen.getByRole("button", { name: "Expand Website relaunch" }), { altKey: true });

      expect(mocked.navSetExpanded).toHaveBeenCalledWith([NW_ID]);
    });
  });

  describe("New filter list", () => {
    it("creates the list, opens it and lets it be named in place", async () => {
      const newList = list(30, "New filter list");
      const client = await renderSidebar(treeWith(), navAt(null, [NW_ID]));
      mocked.listCreate.mockImplementation(async () => {
        mocked.sidebarTree.mockResolvedValue(treeWith({ nw: { lists: [SPRINT, EPICS, newList] } }));
        await client.invalidateQueries({ queryKey: SIDEBAR_KEY });
        return 30;
      });

      await userEvent.click(await screen.findByRole("button", { name: "New filter list" }));

      const field = await screen.findByRole("textbox", { name: "Filter list name" });
      expect(mocked.listCreate).toHaveBeenCalledWith(NW_ID, "New filter list", null);
      expect(mocked.navGo).toHaveBeenCalledWith(30, null);
      await userEvent.clear(field);
      await userEvent.type(field, "Roadmap{Enter}");
      expect(mocked.listRename).toHaveBeenCalledWith(30, "Roadmap");
    });
  });

  describe("pinned lists", () => {
    it("show as extra Home rows after the built-in ones", async () => {
      const tree = treeWith();
      tree.pinnedLists = [list(22, "Epics", { count: 3 })];
      await renderSidebar(tree);

      const rows = within(screen.getByRole("navigation", { name: "Home" })).getAllByRole("button");
      expect(rows.map((r) => r.textContent)).toEqual(["Everything42", "Recently changed8", "Pinned", "Epics3"]);
    });

    it("open the list's level when clicked", async () => {
      const tree = treeWith();
      tree.pinnedLists = [list(22, "Epics", { count: 3 })];
      await renderSidebar(tree);

      await userEvent.click(within(screen.getByRole("navigation", { name: "Home" })).getByRole("button", { name: named("Epics") }));

      expect(mocked.navGo).toHaveBeenCalledWith(22, null);
    });

    it("are absent when nothing is pinned", async () => {
      await renderSidebar();

      expect(within(screen.getByRole("navigation", { name: "Home" })).getAllByRole("button")).toHaveLength(3);
    });
  });

  describe("context menu of a pinned filter list", () => {
    beforeEach(async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 9, target: listTarget({ id: EPICS.id, name: "Epics" }) })]));
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));
      await waitFor(() => expect(mocked.pinsList).toHaveBeenCalled());
    });

    it("offers Unpin from Home instead of Pin to Home and ends the pin", async () => {
      await waitFor(() => {
        mocked.popupMenu.mockClear();
        fireEvent.contextMenu(screen.getByRole("button", { name: named("Epics") }));
        expect(menuEntry("Unpin from Home")).toBeDefined();
      });
      expect(menuEntry("Pin to Home")).toBeUndefined();

      await chooseFromMenu("Unpin from Home");

      expect(mocked.pinRemove).toHaveBeenCalledWith(9);
    });
  });

  describe("filter list context menu", () => {
    beforeEach(async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));
      fireEvent.contextMenu(await screen.findByRole("button", { name: named("Epics") }));
    });

    it("offers Rename, Duplicate, Pin to Home and Delete", () => {
      const entries: MenuEntry[] = mocked.popupMenu.mock.calls[0][0];

      expect(entries.map((e) => (e === "separator" ? e : e.text))).toEqual(["Rename", "Duplicate", "Pin to Home", "separator", "Delete"]);
      expect(menuEntry("Pin to Home")).not.toMatchObject({ enabled: false });
    });

    it("pins the list to Home until unpinned, without a reason", async () => {
      await chooseFromMenu("Pin to Home");

      expect(mocked.pinCreate).toHaveBeenCalledWith("filterList", String(EPICS.id), "untilUnpinned", null);
    });

    it("renames the list in place and sends the new name on enter", async () => {
      await chooseFromMenu("Rename");

      const field = screen.getByRole("textbox", { name: "Filter list name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Backlog{Enter}");

      expect(mocked.listRename).toHaveBeenCalledWith(EPICS.id, "Backlog");
      expect(screen.queryByRole("textbox", { name: "Filter list name" })).not.toBeInTheDocument();
    });

    it("keeps the name when the rename is cancelled with escape", async () => {
      await chooseFromMenu("Rename");

      await userEvent.type(screen.getByRole("textbox", { name: "Filter list name" }), "Backlog{Escape}");

      expect(mocked.listRename).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: named("Epics") })).toBeInTheDocument();
    });

    it("duplicates the list", async () => {
      await chooseFromMenu("Duplicate");

      expect(mocked.listDuplicate).toHaveBeenCalledWith(EPICS.id);
    });

    it("asks first and deletes the list only when Delete Filter List is clicked", async () => {
      await chooseFromMenu("Delete");

      expect(await screen.findByRole("alertdialog", { name: "Delete the filter list “Epics”?" })).toBeInTheDocument();
      expect(mocked.listDelete).not.toHaveBeenCalled();
      await userEvent.click(screen.getByRole("button", { name: "Delete Filter List" }));

      await waitFor(() => expect(mocked.listDelete).toHaveBeenCalledWith(EPICS.id));
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    });

    it("keeps the list when the deletion is cancelled", async () => {
      await chooseFromMenu("Delete");

      await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

      expect(mocked.listDelete).not.toHaveBeenCalled();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    });
  });

  describe("Space context menu", () => {
    beforeEach(async () => {
      await renderSidebar();
      fireEvent.contextMenu(spaceButton("Website relaunch"));
    });

    it("offers Rename, Colour and icon, Client label and Stop tracking", () => {
      const entries: MenuEntry[] = mocked.popupMenu.mock.calls[0][0];

      expect(entries.map((e) => (e === "separator" ? e : e.text))).toEqual(["Rename", "Colour and icon…", "Client label…", "separator", "Stop tracking"]);
    });

    it("renames the Space in place, keeping its icon, colour and client", async () => {
      await chooseFromMenu("Rename");

      const field = screen.getByRole("textbox", { name: "Space name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Public site{Enter}");

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", { name: "Public site", icon: "R", color: "3", client: "Northwind" });
    });

    it("does not rename the Space when escape is pressed", async () => {
      await chooseFromMenu("Rename");

      await userEvent.type(screen.getByRole("textbox", { name: "Space name" }), "Public site{Escape}");

      expect(mocked.spaceUpdate).not.toHaveBeenCalled();
    });

    it.each([
      ["Colour and icon…", "Colour and icon"],
      ["Client label…", "Client label"],
    ])("opens the %s dialog", async (entry, dialog) => {
      await chooseFromMenu(entry);

      expect(await screen.findByRole("dialog", { name: dialog })).toBeInTheDocument();
    });

    it("asks first and stops tracking the project only when Stop Tracking is clicked", async () => {
      await chooseFromMenu("Stop tracking");

      expect(await screen.findByRole("alertdialog", { name: "Stop tracking “Website relaunch”?" })).toBeInTheDocument();
      expect(mocked.projectTrack).not.toHaveBeenCalled();
      await userEvent.click(screen.getByRole("button", { name: "Stop Tracking" }));

      await waitFor(() => expect(mocked.projectTrack).toHaveBeenCalledWith("10001", false));
    });

    it("keeps tracking the project when the question is cancelled", async () => {
      await chooseFromMenu("Stop tracking");

      await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

      expect(mocked.projectTrack).not.toHaveBeenCalled();
    });
  });

  describe("footer", () => {
    it("opens Settings → Projects from Add Space", async () => {
      await renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: "Add Space" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:projects");
    });

    it("opens Settings from the Settings button", async () => {
      await renderSidebar();

      await userEvent.click(screen.getByRole("button", { name: "Settings" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings");
    });

    it("shows the sync state and syncs when the chip is clicked", async () => {
      await renderSidebar();

      await userEvent.click(await screen.findByRole("button", { name: "Not synced yet" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:refresh");
    });
  });

  describe("drag to reorder", () => {
    let rects: { mockRestore: () => void };

    // Items are 40px tall, stacked from the top by their drag index.
    beforeEach(() => {
      rects = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        const index = this.getAttribute("data-drag-index");
        const top = index === null ? 0 : Number(index) * 40;
        const height = index === null ? 0 : 40;
        return { top, bottom: top + height, height, left: 0, right: 0, width: 0, x: 0, y: top, toJSON: () => ({}) };
      });
    });

    afterEach(() => rects.mockRestore());

    function drag(target: HTMLElement, from: number, to: number) {
      fireEvent.pointerDown(target, { button: 0, clientY: from });
      fireEvent.pointerMove(target, { buttons: 1, clientY: to });
      fireEvent.pointerUp(target);
    }

    it("moves a Space to the index it is dropped at", async () => {
      await renderSidebar();

      drag(spaceButton("Website relaunch"), 10, 70);

      expect(mocked.spaceMove).toHaveBeenCalledWith("10001", 1);
    });

    it("moves a filter list to the index it is dropped at", async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));

      drag(await screen.findByRole("button", { name: named("Current sprint") }), 10, 100);

      expect(mocked.listMove).toHaveBeenCalledWith(SPRINT.id, 1);
    });

    it("swallows the click that ends a drag", async () => {
      await renderSidebar();
      const button = spaceButton("Website relaunch");

      drag(button, 10, 70);
      fireEvent.click(button);

      expect(mocked.navGo).not.toHaveBeenCalled();
    });

    it("treats a press that barely moves as a click", async () => {
      await renderSidebar();
      const button = spaceButton("Website relaunch");

      drag(button, 10, 12);
      fireEvent.click(button);

      expect(mocked.spaceMove).not.toHaveBeenCalled();
      expect(mocked.navGo).toHaveBeenCalledWith(NW_ID, null);
    });

    it("moves nothing when dropped where it already is", async () => {
      await renderSidebar();

      drag(spaceButton("Website relaunch"), 10, 30);

      expect(mocked.spaceMove).not.toHaveBeenCalled();
    });

    it("ignores a press with a secondary button", async () => {
      await renderSidebar();
      const button = spaceButton("Website relaunch");

      fireEvent.pointerDown(button, { button: 2, clientY: 10 });
      fireEvent.pointerMove(button, { buttons: 1, clientY: 70 });
      fireEvent.pointerUp(button);

      expect(mocked.spaceMove).not.toHaveBeenCalled();
    });
  });
});
