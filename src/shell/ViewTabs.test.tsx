import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Layout, type MenuEntry } from "../api";
import { ConfirmHost } from "../components/ConfirmSheet";
import { installCommands, useCommandKeys } from "../commands";
import { runPageAction } from "../commands/pageActions";
import { getCommand } from "../commands/registry";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
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
    </QueryClientProvider>,
  );
  await screen.findByRole("tab", { name: views[0].name });
}

type Entry = Exclude<MenuEntry, "separator">;
const entriesOf = (entries: MenuEntry[]) => entries.filter((e): e is Entry => e !== "separator");

/** Runs the entry with this text from the native menu that was last popped up (or from one of its submenus). */
function choose(text: string, submenu?: string) {
  const top = entriesOf(mocked.popupMenu.mock.lastCall![0]);
  const entries = submenu ? entriesOf(top.find((e) => e.text === submenu)!.items!) : top;
  const entry = entries.find((e) => e.text === text);
  if (!entry) throw new Error(`No menu entry "${text}"`);
  return act(async () => entry.run?.());
}

const entry = (text: string, submenu?: string) => {
  const top = entriesOf(mocked.popupMenu.mock.lastCall![0]);
  return entriesOf(submenu ? top.find((e) => e.text === submenu)!.items! : top).find((e) => e.text === text)!;
};

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
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

    it("opens a view when its tab is clicked", async () => {
      await renderTabs();

      await userEvent.click(screen.getByRole("tab", { name: "Sprint board" }));

      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
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
    it("opens a New view dialog offering the three layouts, without a native menu", async () => {
      await renderTabs();

      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      const dialog = screen.getByRole("dialog", { name: "New view" });
      expect(within(dialog).getAllByRole("button").map((b) => b.textContent)).toEqual([
        "ListGroups of rows",
        "BoardStatus columns",
        "TableA dense grid",
      ]);
      expect(mocked.popupMenu).not.toHaveBeenCalled();
    });

    it("offers only the layouts the Space offers", async () => {
      await renderTabs([view({ id: 101, name: "List", layouts: ["list", "table"] }), view({ id: 103, name: "Overview", layout: "table", layouts: ["list", "table"] })]);

      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      const dialog = screen.getByRole("dialog", { name: "New view" });
      expect(within(dialog).getByRole("button", { name: /^List/ })).toBeInTheDocument();
      expect(within(dialog).getByRole("button", { name: /^Table/ })).toBeInTheDocument();
      expect(within(dialog).queryByRole("button", { name: /^Board/ })).not.toBeInTheDocument();
    });

    it("closes the dialog on Escape", async () => {
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("dialog", { name: "New view" })).not.toBeInTheDocument();
    });

    it.each([
      ["List", "list"],
      ["Board", "board"],
      ["Table", "table"],
    ] as const)("adds a %s view and opens it", async (name, layout) => {
      mocked.viewCreate.mockResolvedValue(view({ id: 200, name, layout }));
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await userEvent.click(within(screen.getByRole("dialog", { name: "New view" })).getByRole("button", { name: new RegExp(`^${name}`) }));

      expect(mocked.viewCreate).toHaveBeenCalledWith(11, name, layout);
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(11, 200));
      expect(screen.queryByRole("dialog", { name: "New view" })).not.toBeInTheDocument();
    });

    it("shows an error toast and stays put when the view cannot be added", async () => {
      mocked.viewCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await userEvent.click(within(screen.getByRole("dialog", { name: "New view" })).getByRole("button", { name: /^Board/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not add the view: Disk full"));
      expect(mocked.navGo).not.toHaveBeenCalled();
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

    it("leaves a view's name alone when the rename is cancelled", async () => {
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

    it("changes a view's layout, with its current layout disabled", async () => {
      mocked.viewSetLayout.mockResolvedValue({ ...BOARD, layout: "table" });
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      expect(entry("Board", "Change layout").enabled).toBe(false);
      expect(entry("List", "Change layout").enabled).toBe(true);
      expect(entry("Table", "Change layout").enabled).toBe(true);
      await choose("Table", "Change layout");

      expect(mocked.viewSetLayout).toHaveBeenCalledWith(102, "table");
    });

    it("lists only the layouts the Space offers under Change layout", async () => {
      await renderTabs([view({ id: 101, name: "List", layouts: ["list", "table"] })]);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      const layouts = entriesOf(entry("Change layout").items!).map((e) => e.text);

      expect(layouts).toEqual(["List", "Table"]);
    });

    it("disables Delete for the only view", async () => {
      await renderTabs([LIST]);

      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      expect(entry("Delete").enabled).toBe(false);
    });

    it("asks first, then deletes the current view and opens the level's default view", async () => {
      mocked.viewDelete.mockResolvedValue([BOARD]);
      await renderTabs([LIST, BOARD], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      expect(entry("Delete").enabled).toBe(true);
      await choose("Delete");

      expect(await screen.findByRole("alertdialog", { name: "Delete the view “List”?" })).toBeInTheDocument();
      expect(mocked.viewDelete).not.toHaveBeenCalled();
      await userEvent.click(screen.getByRole("button", { name: "Delete View" }));

      await waitFor(() => expect(mocked.viewDelete).toHaveBeenCalledWith(101));
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(11, null));
    });

    it("keeps the view when the deletion is cancelled", async () => {
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
      it("opens the New view dialog", async () => {
        await renderTabs();

        run("view.new");

        expect(await screen.findByRole("dialog", { name: "New view" })).toBeInTheDocument();
      });

      it("opens the New view dialog on ⌘T", async () => {
        await renderTabs();

        await userEvent.keyboard("{Meta>}t{/Meta}");

        expect(await screen.findByRole("dialog", { name: "New view" })).toBeInTheDocument();
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
});
