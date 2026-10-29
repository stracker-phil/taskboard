import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ListNode, type MenuEntry, type Nav, type SidebarTree, type SpaceNode } from "../../api";
import { installCommands, useCommandKeys } from "../../commands";
import { runPageAction } from "../../commands/pageActions";
import { createQueryClient } from "../../queries/queryClient";
import { SIDEBAR_KEY } from "../../queries/sidebar";
import { resetStore, useStore } from "../../state/store";
import { listTarget, pinItem, pins } from "../../test/pins";
import { lumenSpace, space } from "../../test/spaces";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { Toasts } from "../../components/Toasts";
import { Sidebar } from "./Sidebar";

const mocked = vi.mocked(api, true);

function Keys() {
  useCommandKeys();
  return null;
}

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
  icon: null,
  color: null,
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
// Recently changed shows its count as a badge labelled "<n> unread".
const named = (label: string) => new RegExp(`^${label}\\s*(\\d+( unread)?)?$`);
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

    it("show Recently changed's count as an unread badge", async () => {
      await renderSidebar();

      expect(within(screen.getByRole("navigation", { name: "Home" })).getByLabelText("8 unread")).toHaveTextContent("8");
    });

    it.each([0, null])("show no badge and no count on Recently changed when its count is %s", async (count) => {
      const tree = treeWith();
      tree.home = [list(2, "Recently changed", { kind: "recent", count }), list(3, "Pinned", { kind: "pinned", count: null })];
      await renderSidebar(tree);

      const row = within(screen.getByRole("navigation", { name: "Home" })).getByRole("button", { name: "Recently changed" });
      expect(row).toHaveTextContent(/^Recently changed$/);
      expect(screen.queryByLabelText(/unread/)).not.toBeInTheDocument();
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
    it("show their tile letter and name, without an open count", async () => {
      await renderSidebar();

      for (const [name, count] of [["Website relaunch", "21"], ["Northwind support", "5"], ["Lumen iOS app", "9"]]) {
        expect(screen.getByRole("button", { name })).not.toHaveTextContent(count);
      }
      expect(screen.getByText("R")).toBeInTheDocument();
      expect(screen.getByText("N")).toBeInTheDocument();
      expect(screen.getByText("L")).toBeInTheDocument();
    });

    it("have no Add Space button in the header", async () => {
      await renderSidebar();

      expect(within(screen.getByRole("navigation", { name: "Spaces" })).queryByRole("button", { name: "Add Space" })).not.toBeInTheDocument();
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

    it("show each filter list's count", async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));

      expect(within(await screen.findByRole("button", { name: named("Current sprint") })).getByText("7")).toBeInTheDocument();
      expect(within(screen.getByRole("button", { name: named("Epics") })).getByText("3")).toBeInTheDocument();
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

      await userEvent.click(await screen.findByRole("button", { name: "New filter list in Website relaunch" }));

      const field = await screen.findByRole("textbox", { name: "Filter list name" });
      expect(mocked.listCreate).toHaveBeenCalledWith(NW_ID, "New filter list", null);
      expect(mocked.navGo).toHaveBeenCalledWith(30, null);
      await userEvent.clear(field);
      await userEvent.type(field, "Roadmap{Enter}");
      expect(mocked.listRename).toHaveBeenCalledWith(30, "Roadmap");
    });

    it("shows the plain button only for a Space without filter lists", async () => {
      await renderSidebar(treeWith({ nw: { lists: [] } }), navAt(null, [NW_ID, LUM_ID]));

      expect(await screen.findByRole("button", { name: "New filter list" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "New filter list in Website relaunch" })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "New filter list in Lumen iOS app" })).toBeInTheDocument();
      expect(screen.getAllByRole("button", { name: "New filter list" })).toHaveLength(1);
    });

    it("creates the first list of an empty Space from the plain button", async () => {
      await renderSidebar(treeWith({ nw: { lists: [] } }), navAt(null, [NW_ID]));

      await userEvent.click(await screen.findByRole("button", { name: "New filter list" }));

      await waitFor(() => expect(mocked.listCreate).toHaveBeenCalledWith(NW_ID, "New filter list", null));
      expect(mocked.navGo).toHaveBeenCalledWith(30, null);
    });
  });

  describe("page actions", () => {
    const run = (id: string) => {
      let offered = false;
      act(() => {
        offered = runPageAction(id);
      });
      return offered;
    };

    /** Makes the created list appear in the refetched tree, as Rust's answer would. */
    function createsListInto(client: Awaited<ReturnType<typeof renderSidebar>>, patch: Parameters<typeof treeWith>[0]) {
      mocked.listCreate.mockImplementation(async () => {
        mocked.sidebarTree.mockResolvedValue(treeWith(patch));
        await client.invalidateQueries({ queryKey: SIDEBAR_KEY });
        return 30;
      });
    }

    it.each([
      ["go.everything", 1],
      ["go.recent", 2],
      ["go.pinned", 3],
    ])("%s opens its Home level", async (id, listId) => {
      await renderSidebar();

      run(id);

      expect(mocked.navGo).toHaveBeenCalledWith(listId, null);
    });

    it("offers no Go to Recently changed or Pinned when the tree has no such Home rows", async () => {
      const tree = treeWith();
      tree.home = [];
      await renderSidebar(tree);

      expect(run("go.recent")).toBe(false);
      expect(run("go.pinned")).toBe(false);
      expect(run("go.everything")).toBe(true);
    });

    it("opens Pinned on ⌘⇧P", async () => {
      installCommands("main");
      await renderSidebar();
      render(<Keys />);

      await userEvent.keyboard("{Meta>}{Shift>}p{/Shift}{/Meta}");

      expect(mocked.navGo).toHaveBeenCalledWith(3, null);
    });

    describe("list.new", () => {
      it("creates the list under the current Space, opens it and lets it be named in place", async () => {
        const client = await renderSidebar(treeWith(), navAt(SPRINT.id, [NW_ID]));
        createsListInto(client, { nw: { lists: [SPRINT, EPICS, list(30, "New filter list")] } });

        run("list.new");

        const field = await screen.findByRole("textbox", { name: "Filter list name" });
        expect(mocked.listCreate).toHaveBeenCalledWith(NW_ID, "New filter list", null);
        expect(mocked.navGo).toHaveBeenCalledWith(30, null);
        expect(mocked.navSetExpanded).not.toHaveBeenCalled();
        await userEvent.clear(field);
        await userEvent.type(field, "Roadmap{Enter}");
        expect(mocked.listRename).toHaveBeenCalledWith(30, "Roadmap");
      });

      it("expands the current Space first when it is collapsed", async () => {
        const client = await renderSidebar(treeWith(), navAt(SPRINT.id, [LUM_ID]));
        createsListInto(client, { nw: { lists: [SPRINT, EPICS, list(30, "New filter list")] } });

        run("list.new");

        await screen.findByRole("textbox", { name: "Filter list name" });
        expect(mocked.listCreate).toHaveBeenCalledWith(NW_ID, "New filter list", null);
        expect(mocked.navSetExpanded).toHaveBeenCalledWith([LUM_ID, NW_ID]);
      });

      it("creates the list under Everything outside a Space, and opens it without renaming", async () => {
        await renderSidebar(treeWith(), navAt(1));

        run("list.new");

        await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(30, null));
        expect(mocked.listCreate).toHaveBeenCalledWith(1, "New filter list", null);
        expect(mocked.navSetExpanded).not.toHaveBeenCalled();
        expect(screen.queryByRole("textbox", { name: "Filter list name" })).not.toBeInTheDocument();
      });

      it("shows an error toast and opens nothing when the list cannot be created", async () => {
        mocked.listCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
        await renderSidebar(treeWith(), navAt(1));

        run("list.new");

        expect(await screen.findByText("Could not create the filter list: Disk full")).toBeInTheDocument();
        expect(mocked.navGo).not.toHaveBeenCalled();
      });

      it("runs on ⌘N", async () => {
        installCommands("main");
        await renderSidebar(treeWith(), navAt(1));
        render(<Keys />);

        await userEvent.keyboard("{Meta>}n{/Meta}");

        await waitFor(() => expect(mocked.listCreate).toHaveBeenCalledWith(1, "New filter list", null));
      });
    });

    it("withdraws every sidebar action when the sidebar unmounts", async () => {
      await renderSidebar();

      cleanup();

      expect(["go.everything", "go.recent", "go.pinned", "list.new"].map(run)).toEqual([false, false, false, false]);
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

    it("offers Remove from Home instead of Add to Home and ends the pin", async () => {
      await waitFor(() => {
        mocked.popupMenu.mockClear();
        fireEvent.contextMenu(screen.getByRole("button", { name: named("Epics") }));
        expect(menuEntry("Remove from Home")).toBeDefined();
      });
      expect(menuEntry("Add to Home")).toBeUndefined();

      await chooseFromMenu("Remove from Home");

      expect(mocked.pinRemove).toHaveBeenCalledWith(9);
    });
  });

  describe("filter list context menu", () => {
    beforeEach(async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));
      fireEvent.contextMenu(await screen.findByRole("button", { name: named("Epics") }));
    });

    it("offers Rename, Duplicate, Add to Home and Delete", () => {
      const entries: MenuEntry[] = mocked.popupMenu.mock.calls[0][0];

      expect(entries.map((e) => (e === "separator" ? e : e.text))).toEqual(["Rename", "Duplicate", "Add to Home", "separator", "Delete"]);
      expect(menuEntry("Add to Home")).not.toMatchObject({ enabled: false });
    });

    it("adds the list to Home until removed, without a reason", async () => {
      await chooseFromMenu("Add to Home");

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

  describe("filter list look", () => {
    const changeLook = (name: string) => screen.findByRole("button", { name: `Change the icon and color of ${name}` });
    const withList = (patch: Partial<ListNode>) => treeWith({ nw: { lists: [list(21, "Current sprint", patch), EPICS] } });

    beforeEach(() => {
      mocked.listSetLook.mockResolvedValue(undefined);
    });

    it("opens the Icon and color dialog from the icon button without opening the list", async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));

      await userEvent.click(await changeLook("Epics"));

      const dialog = screen.getByRole("dialog", { name: "Icon and color" });
      expect(within(dialog).getByRole("radiogroup", { name: "Icon" })).toBeInTheDocument();
      expect(within(dialog).getByRole("radiogroup", { name: "Color" })).toBeInTheDocument();
      expect(mocked.navGo).not.toHaveBeenCalled();
    });

    it("marks the list's own icon and color as the current ones", async () => {
      await renderSidebar(withList({ icon: "rocket", color: "4" }), navAt(null, [NW_ID]));

      await userEvent.click(await changeLook("Current sprint"));

      expect(screen.getByRole("radio", { name: "rocket", checked: true })).toBeInTheDocument();
      expect(screen.getByRole("radio", { name: "Teal", checked: true })).toBeInTheDocument();
    });

    it("checks No color for a list without a color", async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));

      await userEvent.click(await changeLook("Epics"));

      expect(screen.getByRole("radio", { name: "No color", checked: true })).toBeInTheDocument();
      expect(within(screen.getByRole("radiogroup", { name: "Icon" })).queryAllByRole("radio", { checked: true })).toHaveLength(0);
    });

    it("lists No color first among the colors", async () => {
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));

      await userEvent.click(await changeLook("Epics"));

      const names = within(screen.getByRole("radiogroup", { name: "Color" })).getAllByRole("radio").map((r) => r.getAttribute("aria-label"));
      expect(names.slice(0, 2)).toEqual(["No color", "Blue"]);
      expect(names).toHaveLength(13);
    });

    it("saves the picked icon, keeping the color, and leaves the dialog open", async () => {
      await renderSidebar(withList({ color: "4" }), navAt(null, [NW_ID]));
      await userEvent.click(await changeLook("Current sprint"));

      await userEvent.click(screen.getByRole("radio", { name: "check circle" }));

      expect(mocked.listSetLook).toHaveBeenCalledWith(21, "check-circle", "4");
      expect(screen.getByRole("dialog", { name: "Icon and color" })).toBeInTheDocument();
    });

    it("saves the picked color, keeping the icon", async () => {
      await renderSidebar(withList({ icon: "rocket" }), navAt(null, [NW_ID]));
      await userEvent.click(await changeLook("Current sprint"));

      await userEvent.click(screen.getByRole("radio", { name: "Red" }));

      expect(mocked.listSetLook).toHaveBeenCalledWith(21, "rocket", "6");
    });

    it("saves no color when No color is picked, keeping the icon", async () => {
      await renderSidebar(withList({ icon: "rocket", color: "6" }), navAt(null, [NW_ID]));
      await userEvent.click(await changeLook("Current sprint"));

      await userEvent.click(screen.getByRole("radio", { name: "No color" }));

      expect(mocked.listSetLook).toHaveBeenCalledWith(21, "rocket", null);
    });

    it("saves no icon on Reset, keeping the color", async () => {
      await renderSidebar(withList({ icon: "rocket", color: "6" }), navAt(null, [NW_ID]));
      await userEvent.click(await changeLook("Current sprint"));

      await userEvent.click(screen.getByRole("button", { name: "Reset" }));

      expect(mocked.listSetLook).toHaveBeenCalledWith(21, null, "6");
    });

    it("saves a typed letter or emoji, keeping the color", async () => {
      await renderSidebar(withList({ color: "2" }), navAt(null, [NW_ID]));
      await userEvent.click(await changeLook("Current sprint"));

      await userEvent.click(screen.getByRole("button", { name: "Letter or emoji" }));
      await userEvent.type(screen.getByRole("textbox", { name: "Letter or emoji" }), "q{Enter}");

      expect(mocked.listSetLook).toHaveBeenCalledWith(21, "q", "2");
    });

    it("shows an error toast when the look cannot be saved", async () => {
      mocked.listSetLook.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderSidebar(treeWith(), navAt(null, [NW_ID]));
      await userEvent.click(await changeLook("Epics"));

      await userEvent.click(screen.getByRole("radio", { name: "star" }));

      expect(await screen.findByText("Could not change the icon: Disk full")).toBeInTheDocument();
    });
  });

  describe("Space context menu", () => {
    beforeEach(async () => {
      await renderSidebar();
      fireEvent.contextMenu(spaceButton("Website relaunch"));
    });

    it("offers Rename, Icon and Color, Client label, Space Settings and Stop tracking", () => {
      const entries: MenuEntry[] = mocked.popupMenu.mock.calls[0][0];

      expect(entries.map((e) => (e === "separator" ? e : e.text))).toEqual([
        "Rename",
        "Icon and Color…",
        "Client label…",
        "Space Settings…",
        "separator",
        "Stop tracking",
      ]);
    });

    it("opens the Space's settings through the bus command", async () => {
      await chooseFromMenu("Space Settings…");

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:space:10001");
    });

    it("shows an error toast when the settings cannot be opened", async () => {
      mocked.runCommand.mockRejectedValue({ kind: "io", message: "No window", code: null });

      await chooseFromMenu("Space Settings…");

      expect(await screen.findByText("Could not open the Space's settings: No window")).toBeInTheDocument();
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
      ["Icon and Color…", "Icon and color"],
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
