import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Layout } from "../api";
import { ConfirmHost } from "../components/ConfirmSheet";
import { MenuHost } from "../components/menu";
import { installCommands, useCommandKeys } from "../commands";
import { runPageAction } from "../commands/pageActions";
import { getCommand } from "../commands/registry";
import { paletteMenus } from "../palette/menus";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { pinItem, pins, viewTarget } from "../test/pins";
import { settings } from "../test/settings";
import { view } from "../test/views";
import { ViewTabs } from "./ViewTabs";

const mocked = vi.mocked(api, true);

// jsdom may lack PointerEvent; the stub keeps the mouse fields (clientX, buttons) that the drag reads.
window.PointerEvent ??= class extends MouseEvent {
  pointerId = 1;
} as unknown as typeof PointerEvent;

const LIST = view({ id: 101, name: "List", layout: "list" });
const BOARD = view({ id: 102, name: "Sprint board", layout: "board", position: 1 });
const TABLE = view({ id: 103, name: "Overview", layout: "table", position: 2 });

function Harness({ viewId }: { viewId: number }) {
  useCommandKeys();
  return <ViewTabs listId={11} viewId={viewId} />;
}

async function renderTabs(views = [LIST, BOARD], viewId = 101) {
  mocked.viewsOf.mockResolvedValue(views);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness viewId={viewId} />
      <ConfirmHost />
      <MenuHost />
    </QueryClientProvider>,
  );
  await screen.findByRole("tab", { name: views[0].name });
}

const rightClick = (name: string) => userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name }) });

/** The row with this name in the open menu, whatever its kind. */
function row(text: string) {
  const menu = within(screen.getByRole("menu"));
  const found = (["menuitem", "menuitemradio", "menuitemcheckbox"] as const).flatMap((role) => menu.queryAllByRole(role, { name: text }));
  if (found.length !== 1) throw new Error(`Expected one menu row "${text}", found ${found.length}`);
  return found[0];
}
const choose = (text: string) => userEvent.click(row(text));
const tileOf = (name: string) => screen.getByRole("tab", { name }).querySelector<HTMLElement>(".view-tab__look")!;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
  mocked.pinsList.mockResolvedValue(pins());
  mocked.getSettings.mockResolvedValue(settings());
  mocked.viewSetLayout.mockResolvedValue(LIST);
  mocked.viewMove.mockResolvedValue([]);
});

describe("ViewTabs", () => {
  describe("tabs", () => {
    it("lists the level's views as tabs and marks the current one selected", async () => {
      await renderTabs();

      expect(screen.getByRole("tab", { name: "List" })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("tab", { name: "Sprint board" })).toHaveAttribute("aria-selected", "false");
    });

    it("names each tab after its view alone", async () => {
      await renderTabs();

      expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["List", "Sprint board"]);
      expect(screen.getByRole("tab", { name: "Sprint board" })).toBeInTheDocument();
    });

    it("opens a view when its tab is clicked", async () => {
      await renderTabs();

      await userEvent.click(screen.getByRole("tab", { name: "Sprint board" }));

      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
    });
  });

  describe("tab tile", () => {
    it("shows the layout's icon for a view without a look of its own", async () => {
      await renderTabs();

      const tab = screen.getByRole("tab", { name: "List" });
      expect(tab.querySelector("svg")).toBeInTheDocument();
    });

    it("shows the view's own letter in place of the layout's icon", async () => {
      await renderTabs([view({ id: 101, name: "List", icon: "q", color: "3" }), BOARD]);

      const tab = screen.getByRole("tab", { name: "List" });
      expect(tab).toHaveTextContent("QList");
      expect(tab.querySelector("svg")).not.toBeInTheDocument();
    });
  });

  describe("keys", () => {
    it("opens the second view on ⌘2", async () => {
      await renderTabs();

      await userEvent.keyboard("{Meta>}2{/Meta}");

      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
    });

    it("does nothing on a number key beyond the last tab", async () => {
      await renderTabs();

      await userEvent.keyboard("{Meta>}3{/Meta}");

      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });

  describe("+ View", () => {
    it("opens a New view menu offering the three layouts", async () => {
      await renderTabs();

      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      const menu = within(screen.getByRole("menu", { name: "New view" }));
      expect(menu.getAllByRole("menuitem").map((b) => b.textContent)).toEqual(["ListGroups of rows", "BoardStatus columns", "TableA dense grid"]);
    });

    it("offers only the layouts the Space offers", async () => {
      await renderTabs([view({ id: 101, name: "List", layouts: ["list", "table"] }), view({ id: 103, name: "Overview", layout: "table", layouts: ["list", "table"] })]);

      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      const menu = within(screen.getByRole("menu", { name: "New view" }));
      expect(menu.getByRole("menuitem", { name: /^List/ })).toBeInTheDocument();
      expect(menu.getByRole("menuitem", { name: /^Table/ })).toBeInTheDocument();
      expect(menu.queryByRole("menuitem", { name: /^Board/ })).not.toBeInTheDocument();
    });

    it("closes the menu on Escape", async () => {
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("menu", { name: "New view" })).not.toBeInTheDocument();
    });

    it.each([
      ["List", "list"],
      ["Board", "board"],
      ["Table", "table"],
    ] as const)("adds a %s view and opens it", async (name, layout) => {
      mocked.viewCreate.mockResolvedValue(view({ id: 200, name, layout }));
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await userEvent.click(within(screen.getByRole("menu", { name: "New view" })).getByRole("menuitem", { name: new RegExp(`^${name}`) }));

      expect(mocked.viewCreate).toHaveBeenCalledWith(11, name, layout);
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(11, 200));
      expect(screen.queryByRole("menu", { name: "New view" })).not.toBeInTheDocument();
    });

    it("shows an error toast and stays put when the view cannot be added", async () => {
      mocked.viewCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await userEvent.click(within(screen.getByRole("menu", { name: "New view" })).getByRole("menuitem", { name: /^Board/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not add the view: Disk full"));
      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });

  describe("click on a selected tab", () => {
    it("starts the inline rename and saves the new name", async () => {
      mocked.viewRename.mockResolvedValue({ ...LIST, name: "Backlog" });
      await renderTabs();

      await userEvent.click(screen.getByRole("tab", { name: "List" }));
      const field = await screen.findByRole("textbox", { name: "View name" });
      expect(field).toHaveValue("List");
      await userEvent.clear(field);
      await userEvent.type(field, "Backlog{Enter}");

      expect(mocked.viewRename).toHaveBeenCalledWith(101, "Backlog");
      expect(mocked.navGo).not.toHaveBeenCalled();
    });

    it("opens an unselected tab without renaming it", async () => {
      await renderTabs();

      await userEvent.click(screen.getByRole("tab", { name: "Sprint board" }));

      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
      expect(screen.queryByRole("textbox", { name: "View name" })).not.toBeInTheDocument();
    });

    it("opens the Icon and color popover from the selected tab's tile, without renaming", async () => {
      await renderTabs();

      await userEvent.click(tileOf("List"));

      expect(await screen.findByRole("dialog", { name: "Icon and color" })).toBeInTheDocument();
      expect(screen.queryByRole("textbox", { name: "View name" })).not.toBeInTheDocument();
    });

    it("only opens an unselected tab from its tile", async () => {
      await renderTabs();

      await userEvent.click(tileOf("Sprint board"));

      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
      expect(screen.queryByRole("dialog", { name: "Icon and color" })).not.toBeInTheDocument();
    });
  });

  describe("double-click", () => {
    it("shows the inline rename field and saves the new name", async () => {
      mocked.viewRename.mockResolvedValue({ ...BOARD, name: "Release board" });
      await renderTabs();

      await userEvent.dblClick(screen.getByRole("tab", { name: "Sprint board" }));
      const field = await screen.findByRole("textbox", { name: "View name" });
      expect(field).toHaveValue("Sprint board");
      await userEvent.clear(field);
      await userEvent.type(field, "Release board{Enter}");

      expect(mocked.viewRename).toHaveBeenCalledWith(102, "Release board");
      expect(screen.queryByRole("textbox", { name: "View name" })).not.toBeInTheDocument();
    });
  });

  describe("drag to reorder", () => {
    let rects: { mockRestore: () => void };

    // Tabs are 100px wide, side by side from the left by their drag index.
    beforeEach(() => {
      rects = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        const index = this.getAttribute("data-drag-index");
        const left = index === null ? 0 : Number(index) * 100;
        const width = index === null ? 0 : 100;
        return { left, right: left + width, width, top: 0, bottom: 30, height: 30, x: left, y: 0, toJSON: () => ({}) };
      });
    });

    afterEach(() => rects.mockRestore());

    function drag(target: HTMLElement, from: number, to: number) {
      fireEvent.pointerDown(target, { button: 0, clientX: from });
      fireEvent.pointerMove(target, { buttons: 1, clientX: to });
      fireEvent.pointerUp(target);
    }

    it.each([
      ["the first tab past the second", "List", 10, 160, 101, 1],
      ["the first tab to the end", "List", 10, 290, 101, 2],
      ["the last tab to the front", "Overview", 210, 10, 103, 0],
    ])("moves %s", async (_name, tab, from, to, id, index) => {
      await renderTabs([LIST, BOARD, TABLE]);

      drag(screen.getByRole("tab", { name: tab }), from, to);

      expect(mocked.viewMove).toHaveBeenCalledWith(id, index);
    });

    it("moves nothing when dropped where it already is", async () => {
      await renderTabs([LIST, BOARD, TABLE]);

      drag(screen.getByRole("tab", { name: "Sprint board" }), 110, 130);

      expect(mocked.viewMove).not.toHaveBeenCalled();
    });

    it("treats a press that barely moves as a click", async () => {
      await renderTabs([LIST, BOARD]);
      const tab = screen.getByRole("tab", { name: "Sprint board" });

      drag(tab, 110, 112);
      fireEvent.click(tab);

      expect(mocked.viewMove).not.toHaveBeenCalled();
      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
    });

    it("swallows the click that ends a drag", async () => {
      await renderTabs([LIST, BOARD, TABLE]);
      const tab = screen.getByRole("tab", { name: "List" });

      drag(tab, 10, 290);
      fireEvent.click(tab);

      expect(mocked.navGo).not.toHaveBeenCalled();
    });

    it("shows an error toast when the view cannot be moved", async () => {
      mocked.viewMove.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderTabs([LIST, BOARD, TABLE]);

      drag(screen.getByRole("tab", { name: "List" }), 10, 290);

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not move the view: Disk full"));
    });
  });

  describe("context menu", () => {
    it("renames a view in place", async () => {
      mocked.viewRename.mockResolvedValue({ ...BOARD, name: "Release board" });
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      await choose("Rename");
      const field = await screen.findByRole("textbox", { name: "View name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Release board{Enter}");

      expect(mocked.viewRename).toHaveBeenCalledWith(102, "Release board");
      expect(screen.queryByRole("textbox", { name: "View name" })).not.toBeInTheDocument();
    });

    it("leaves a view's name alone when the rename is canceled", async () => {
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      await choose("Rename");
      await userEvent.type(await screen.findByRole("textbox", { name: "View name" }), "Other{Escape}");

      expect(mocked.viewRename).not.toHaveBeenCalled();
      expect(screen.getByRole("tab", { name: "Sprint board" })).toBeInTheDocument();
    });

    it("duplicates a view", async () => {
      mocked.viewDuplicate.mockResolvedValue(view({ id: 300, name: "Sprint board copy", layout: "board" }));
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      await choose("Duplicate");

      expect(mocked.viewDuplicate).toHaveBeenCalledWith(102);
    });

    describe("Icon and Color…", () => {
      async function openLook(views = [LIST, BOARD], tab = "Sprint board") {
        mocked.viewSetLook.mockResolvedValue(views.find((v) => v.name === tab)!);
        await renderTabs(views);
        await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: tab }) });
        await choose("Icon and Color…");
        return screen.findByRole("dialog", { name: "Icon and color" });
      }

      it("opens the Icon and color dialog, with No color for a view without one", async () => {
        const dialog = await openLook();

        expect(within(dialog).getByRole("radio", { name: "No color", checked: true })).toBeInTheDocument();
        expect(within(dialog).getByRole("radiogroup", { name: "Icon" })).toBeInTheDocument();
      });

      it("shows the view's own icon and color as the current ones", async () => {
        const dialog = await openLook([LIST, view({ ...BOARD, icon: "rocket", color: "6" })]);

        expect(within(dialog).getByRole("radio", { name: "rocket", checked: true })).toBeInTheDocument();
        expect(within(dialog).getByRole("radio", { name: "Red", checked: true })).toBeInTheDocument();
      });

      it("saves the picked icon for that view, keeping its color", async () => {
        const dialog = await openLook([LIST, view({ ...BOARD, color: "6" })]);

        await userEvent.click(within(dialog).getByRole("radio", { name: "bug" }));

        expect(mocked.viewSetLook).toHaveBeenCalledWith(102, "bug", "6");
      });

      it("saves the picked color, keeping the icon", async () => {
        const dialog = await openLook([LIST, view({ ...BOARD, icon: "rocket" })]);

        await userEvent.click(within(dialog).getByRole("radio", { name: "Green" }));

        expect(mocked.viewSetLook).toHaveBeenCalledWith(102, "rocket", "8");
      });

      it("saves no color for No color, and no icon for Reset", async () => {
        const dialog = await openLook([LIST, view({ ...BOARD, icon: "rocket", color: "6" })]);

        await userEvent.click(within(dialog).getByRole("radio", { name: "No color" }));
        expect(mocked.viewSetLook).toHaveBeenLastCalledWith(102, "rocket", null);
        await userEvent.click(within(dialog).getByRole("button", { name: "Reset" }));

        expect(mocked.viewSetLook).toHaveBeenLastCalledWith(102, null, "6");
      });

      it("closes on Escape", async () => {
        await openLook();

        await userEvent.keyboard("{Escape}");

        expect(screen.queryByRole("dialog", { name: "Icon and color" })).not.toBeInTheDocument();
      });

      it("shows an error toast when the look cannot be saved", async () => {
        const dialog = await openLook();
        mocked.viewSetLook.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

        await userEvent.click(within(dialog).getByRole("radio", { name: "Red" }));

        await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change the view's icon: Disk full"));
      });
    });

    it("opens a menu named after the view with its actions", async () => {
      await renderTabs();

      await rightClick("Sprint board");

      const menu = within(screen.getByRole("menu", { name: "Sprint board" }));
      expect(menu.getAllByRole("menuitem").map((r) => r.textContent)).toEqual(["Rename", "Icon and Color…", "Duplicate", "Bookmark View", "Delete"]);
      expect(menu.getByRole("group", { name: "Layout" })).toBeInTheDocument();
    });

    describe("bookmarking", () => {
      it("bookmarks the view until removed by default", async () => {
        await renderTabs();
        await rightClick("Sprint board");

        await choose("Bookmark View");

        expect(mocked.pinCreate).toHaveBeenCalledWith("view", "102", "untilUnpinned", null);
      });

      it("bookmarks the view for the duration set in the settings", async () => {
        mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "week", openIssuesIn: "preview" } }));
        await renderTabs();
        await act(async () => void (await mocked.getSettings.mock.results[0]?.value));
        await rightClick("Sprint board");

        await choose("Bookmark View");

        expect(mocked.pinCreate).toHaveBeenCalledWith("view", "102", "week", null);
      });

      it("offers Remove Bookmark instead for a bookmarked view and removes its pin", async () => {
        mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 8, target: viewTarget({ id: 102 }) })]));
        await renderTabs([LIST, BOARD], 102);
        await waitFor(() => expect(paletteMenus().find((menu) => menu.title === "View")?.entries()).toContainEqual(expect.objectContaining({ text: "Remove Bookmark" })));
        await rightClick("Sprint board");

        expect(screen.queryByRole("menuitem", { name: "Bookmark View" })).not.toBeInTheDocument();
        await choose("Remove Bookmark");

        expect(mocked.pinRemove).toHaveBeenCalledWith(8);
      });

      it("offers Bookmark View on another tab than the bookmarked one", async () => {
        mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 8, target: viewTarget({ id: 102 }) })]));
        await renderTabs([LIST, BOARD], 102);
        await waitFor(() => expect(paletteMenus().find((menu) => menu.title === "View")?.entries()).toContainEqual(expect.objectContaining({ text: "Remove Bookmark" })));
        await rightClick("List");

        expect(row("Bookmark View")).toBeInTheDocument();
      });

      it("shows an error toast when the bookmark cannot be made", async () => {
        mocked.pinCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
        await renderTabs();
        await rightClick("Sprint board");

        await choose("Bookmark View");

        await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not bookmark the view: Disk full"));
      });
    });

    it("checks the view's current layout in the Layout group", async () => {
      await renderTabs();

      await rightClick("Sprint board");

      const group = within(screen.getByRole("group", { name: "Layout" }));
      expect(group.getAllByRole("menuitemradio").map((r) => [r.textContent, r.getAttribute("aria-checked")])).toEqual([
        ["List", "false"],
        ["Board", "true"],
        ["Table", "false"],
      ]);
    });

    it("changes a view's layout", async () => {
      mocked.viewSetLayout.mockResolvedValue({ ...BOARD, layout: "table" });
      await renderTabs();
      await rightClick("Sprint board");

      await choose("Table");

      expect(mocked.viewSetLayout).toHaveBeenCalledWith(102, "table");
    });

    it("keeps the layout when the checked one is chosen", async () => {
      await renderTabs();
      await rightClick("Sprint board");

      await choose("Board");

      expect(mocked.viewSetLayout).not.toHaveBeenCalled();
    });

    it("lists only the layouts the Space offers in the Layout group", async () => {
      await renderTabs([view({ id: 101, name: "List", layouts: ["list", "table"] })]);
      await rightClick("List");

      const layouts = within(screen.getByRole("group", { name: "Layout" })).getAllByRole("menuitemradio");

      expect(layouts.map((r) => r.textContent)).toEqual(["List", "Table"]);
    });

    it("disables Delete for the only view", async () => {
      await renderTabs([LIST]);

      await rightClick("List");

      expect(row("Delete")).toHaveAttribute("aria-disabled", "true");
    });

    it("asks first, then deletes the current view and opens the level's default view", async () => {
      mocked.viewDelete.mockResolvedValue([BOARD]);
      await renderTabs([LIST, BOARD], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      expect(row("Delete")).not.toHaveAttribute("aria-disabled");
      await choose("Delete");

      expect(await screen.findByRole("alertdialog", { name: "Delete the view “List”?" })).toBeInTheDocument();
      expect(mocked.viewDelete).not.toHaveBeenCalled();
      await userEvent.click(screen.getByRole("button", { name: "Delete View" }));

      await waitFor(() => expect(mocked.viewDelete).toHaveBeenCalledWith(101));
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(11, null));
    });

    it("keeps the view when the deletion is canceled", async () => {
      await renderTabs([LIST, BOARD], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });
      await choose("Delete");

      await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

      expect(mocked.viewDelete).not.toHaveBeenCalled();
      expect(mocked.navGo).not.toHaveBeenCalled();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    });

    it("stays on the current view when another view is deleted", async () => {
      mocked.viewDelete.mockResolvedValue([LIST]);
      await renderTabs([LIST, BOARD, TABLE], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Overview" }) });

      await choose("Delete");
      await userEvent.click(await screen.findByRole("button", { name: "Delete View" }));

      await waitFor(() => expect(mocked.viewDelete).toHaveBeenCalledWith(103));
      expect(mocked.navGo).not.toHaveBeenCalled();
    });

    it("shows an error toast when a view cannot be deleted", async () => {
      mocked.viewDelete.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderTabs([LIST, BOARD], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      await choose("Delete");
      await userEvent.click(await screen.findByRole("button", { name: "Delete View" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not delete the view: Disk full"));
      expect(mocked.navGo).not.toHaveBeenCalled();
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

    describe("view.new", () => {
      it("opens the New view menu", async () => {
        await renderTabs();

        run("view.new");

        expect(await screen.findByRole("menu", { name: "New view" })).toBeInTheDocument();
      });

      it("opens the New view menu on ⌘T", async () => {
        await renderTabs();

        await userEvent.keyboard("{Meta>}t{/Meta}");

        expect(await screen.findByRole("menu", { name: "New view" })).toBeInTheDocument();
      });
    });

    describe("view.next and view.previous", () => {
      it.each([
        ["view.next", 101, 102],
        ["view.next", 103, 101],
        ["view.previous", 102, 101],
        ["view.previous", 101, 103],
      ])("%s from view %i opens view %i", async (id, from, to) => {
        await renderTabs([LIST, BOARD, TABLE], from);

        run(id);

        expect(mocked.navGo).toHaveBeenCalledWith(11, to);
      });

      it("moves to the next view on ⌃⇥ and the previous on ⌃⇧⇥", async () => {
        await renderTabs([LIST, BOARD, TABLE], 102);

        await userEvent.keyboard("{Control>}{Tab}{/Control}");
        expect(mocked.navGo).toHaveBeenLastCalledWith(11, 103);

        await userEvent.keyboard("{Control>}{Shift>}{Tab}{/Shift}{/Control}");
        expect(mocked.navGo).toHaveBeenLastCalledWith(11, 101);
      });

      it.each(["view.next", "view.previous"])("%s is not offered with a single view", async (id) => {
        await renderTabs([LIST]);

        expect(run(id)).toBe(false);
        expect(getCommand(id)?.when?.()).toBe(false);
        expect(mocked.navGo).not.toHaveBeenCalled();
      });
    });

    describe("view.layout", () => {
      it.each([
        ["view.layout.board", "board"],
        ["view.layout.table", "table"],
      ])("%s changes the current list view to %s", async (id, layout) => {
        mocked.viewSetLayout.mockResolvedValue({ ...LIST, layout: layout as Layout });
        await renderTabs([LIST, BOARD], 101);

        run(id);

        expect(mocked.viewSetLayout).toHaveBeenCalledWith(101, layout);
      });

      it("changes the layout of the current view, not of the first", async () => {
        mocked.viewSetLayout.mockResolvedValue({ ...BOARD, layout: "list" });
        await renderTabs([LIST, BOARD], 102);

        run("view.layout.list");

        expect(mocked.viewSetLayout).toHaveBeenCalledWith(102, "list");
      });

      it("does nothing when the view already has that layout", async () => {
        await renderTabs([LIST, BOARD], 102);

        run("view.layout.board");

        expect(mocked.viewSetLayout).not.toHaveBeenCalled();
      });

      it("changes the layout on ⌃⌘2", async () => {
        mocked.viewSetLayout.mockResolvedValue({ ...LIST, layout: "board" });
        await renderTabs([LIST, BOARD], 101);

        await userEvent.keyboard("{Meta>}{Control>}2{/Control}{/Meta}");

        expect(mocked.viewSetLayout).toHaveBeenCalledWith(101, "board");
      });

      it("shows an error toast when the layout cannot be changed", async () => {
        mocked.viewSetLayout.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
        await renderTabs([LIST, BOARD], 101);

        run("view.layout.table");

        await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change the layout: Disk full"));
      });

      it("offers the actions of the Space's layouts only", async () => {
        const restricted = (patch: Parameters<typeof view>[0]) => view({ layouts: ["list", "table"], ...patch });
        await renderTabs([restricted({ id: 101, name: "List" }), restricted({ id: 103, name: "Overview", layout: "table" })], 101);

        expect(run("view.layout.board")).toBe(false);
        expect(run("view.layout.table")).toBe(true);
        expect(mocked.viewSetLayout).toHaveBeenCalledWith(101, "table");
      });

      it("switches a table view back to list when the Space offers list", async () => {
        await renderTabs([view({ id: 101, name: "List", layout: "table", layouts: ["list", "table"] })], 101);

        expect(run("view.layout.list")).toBe(true);
        expect(mocked.viewSetLayout).toHaveBeenCalledWith(101, "list");
      });

      it.each(["view.layout.list", "view.layout.board", "view.layout.table"])("%s is not offered when the current view is not among the tabs", async (id) => {
        await renderTabs([LIST, BOARD], 999);

        expect(run(id)).toBe(false);
      });
    });

    it("withdraws every view action when the tabs unmount", async () => {
      await renderTabs([LIST, BOARD]);

      cleanup();

      expect(["view.new", "view.next", "view.previous", "view.layout.list"].map(run)).toEqual([false, false, false, false]);
    });
  });

  describe("palette entries", () => {
    const entriesOf = (title: string) =>
      (paletteMenus().find((menu) => menu.title === title)?.entries() ?? []).flatMap((entry) => (typeof entry === "string" || "caption" in entry ? [] : [entry]));

    it("offers every view to open under “Views”, the current one checked", async () => {
      await renderTabs([LIST, BOARD]);

      expect(paletteMenus().find((menu) => menu.title === "Open view")?.section).toBe("Views");
      expect(entriesOf("Open view").map((e) => [e.text, e.checked])).toEqual([
        ["List", true],
        ["Sprint board", false],
      ]);
    });

    it("offers renaming, styling, duplicating, bookmarking and deleting the current view", async () => {
      await renderTabs([LIST, BOARD]);

      expect(entriesOf("View").map((e) => e.text)).toEqual(["Rename", "Icon and Color…", "Duplicate", "Bookmark View", "Delete"]);
    });

    it("offers Remove Bookmark for the current view once it is bookmarked", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 8, target: viewTarget({ id: 101 }) })]));
      await renderTabs([LIST, BOARD]);

      await waitFor(() => expect(entriesOf("View").map((e) => e.text)).toEqual(["Rename", "Icon and Color…", "Duplicate", "Remove Bookmark", "Delete"]));
    });

    it("greys out Delete while the view is the only one", async () => {
      await renderTabs([LIST]);

      expect(entriesOf("View").find((e) => e.text === "Delete")).toMatchObject({ enabled: false });
    });

    it("offers nothing for a current view that is not among the tabs", async () => {
      await renderTabs([LIST, BOARD], 999);

      expect(entriesOf("View")).toEqual([]);
    });
  });
});
