import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type PinItem } from "../api";
import { MenuHost } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { nav } from "../test/nav";
import { commentTarget, listTarget, pinItem, pins, viewTarget } from "../test/pins";
import { sidebarTree } from "../test/spaces";
import { listNode } from "../test/views";
import { BookmarksBar } from "./BookmarksBar";

const mocked = vi.mocked(api, true);

// jsdom may lack PointerEvent; the stub keeps the mouse fields (clientX, buttons) that the drag reads.
window.PointerEvent ??= class extends MouseEvent {
  pointerId = 1;
} as unknown as typeof PointerEvent;

const ISSUE = pinItem({ id: 1 });
const LIST = pinItem({ id: 2, target: listTarget() });
const VIEW = pinItem({ id: 3, target: viewTarget() });

async function renderBar(active: PinItem[] = [ISSUE, LIST, VIEW]) {
  mocked.pinsList.mockResolvedValue(pins(active));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <BookmarksBar />
      <MenuHost />
    </QueryClientProvider>,
  );
  if (active.length > 0) await screen.findAllByRole("button", { name: /Redesign the pricing page|My open issues|Sprint board|Pricing work|postcode/ });
}

const bookmark = (name: RegExp | string) => screen.getByRole("button", { name });
const rightClick = (target: HTMLElement) => userEvent.pointer({ keys: "[MouseRight]", target });
const choose = (name: RegExp | string) => userEvent.click(within(screen.getByRole("menu")).getByRole("menuitem", { name }));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.navCurrent.mockResolvedValue(nav({ listId: 11, viewId: 101 }));
  mocked.sidebarTree.mockResolvedValue({ ...sidebarTree([]), home: [listNode({ id: 3, kind: "pinned", name: "Bookmarks", count: null })] });
});

describe("BookmarksBar", () => {
  describe("the row", () => {
    it("shows one button per bookmark, named after what it points at", async () => {
      await renderBar([ISSUE, LIST, VIEW, pinItem({ id: 4, target: commentTarget() })]);

      const bar = screen.getByRole("navigation", { name: "Bookmarks" });
      const names = ["Redesign the pricing page", "My open issues", "Sprint board", "Please check the postcode rules first."];
      const buttons = within(bar).getAllByRole("button");
      names.forEach((name, index) => expect(buttons[index]).toHaveTextContent(name));
      expect(buttons[names.length]).toHaveAccessibleName("More bookmarks");
    });

    it("shows the owner's own name instead of the item's", async () => {
      await renderBar([pinItem({ title: "Pricing work" })]);

      expect(bookmark("Pricing work")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Redesign the pricing page" })).not.toBeInTheDocument();
    });

    it("explains how to add one when there are none", async () => {
      await renderBar([]);

      expect(await screen.findByText("Bookmark an issue, a list or a view to see it here.")).toBeInTheDocument();
    });
  });

  describe("opening", () => {
    it("opens an issue over the view the owner is on", async () => {
      await renderBar();

      await waitFor(async () => {
        await userEvent.click(bookmark("Redesign the pricing page"));
        expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["10042"] });
      });
    });

    it("goes to a list", async () => {
      await renderBar();

      await userEvent.click(bookmark("My open issues"));

      expect(mocked.navGo).toHaveBeenCalledWith(21, null);
    });

    it("goes to a view of its list", async () => {
      await renderBar();

      await userEvent.click(bookmark("Sprint board"));

      expect(mocked.navGo).toHaveBeenCalledWith(21, 7);
    });
  });

  describe("opening in a tab", () => {
    it.each([
      ["a list", "My open issues", 21, null],
      ["a view", "Sprint board", 21, 7],
    ])("opens %s in a tab behind the shown one on ⌘-click, leaving the place alone", async (_kind, name, listId, viewId) => {
      await renderBar();

      fireEvent.click(bookmark(name), { metaKey: true });

      expect(mocked.navTabNew).toHaveBeenCalledWith(listId, viewId, false);
      expect(mocked.navGo).not.toHaveBeenCalled();
    });

    it("opens a view in a tab and shows it on ⌘⇧-click", async () => {
      await renderBar();

      fireEvent.click(bookmark("Sprint board"), { metaKey: true, shiftKey: true });

      expect(mocked.navTabNew).toHaveBeenCalledWith(21, 7, true);
    });

    it("still opens an issue over the view on ⌘-click", async () => {
      await renderBar();

      await waitFor(() => {
        fireEvent.click(bookmark("Redesign the pricing page"), { metaKey: true });
        expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["10042"] });
      });
      expect(mocked.navTabNew).not.toHaveBeenCalled();
    });

    it.each([
      ["a list", "My open issues", 21, null],
      ["a view", "Sprint board", 21, 7],
    ])("opens %s in a tab and shows it from the context menu", async (_kind, name, listId, viewId) => {
      await renderBar();
      await rightClick(bookmark(name));

      await choose("Open in New Tab");

      expect(mocked.navTabNew).toHaveBeenCalledWith(listId, viewId, true);
    });

    it("offers no Open in New Tab on an issue bookmark", async () => {
      await renderBar();

      await rightClick(bookmark("Redesign the pricing page"));

      expect(within(screen.getByRole("menu")).queryByRole("menuitem", { name: "Open in New Tab" })).not.toBeInTheDocument();
    });
  });

  describe("context menu", () => {
    it("opens a bookmark from the menu", async () => {
      await renderBar();
      await rightClick(bookmark("My open issues"));

      await choose("Open");

      expect(mocked.navGo).toHaveBeenCalledWith(21, null);
    });

    it("renames a bookmark in place", async () => {
      await renderBar();
      await rightClick(bookmark("Sprint board"));

      await choose(/^Rename Bookmark/);
      const field = await screen.findByRole("textbox", { name: "Bookmark name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Standup{Enter}");

      expect(mocked.pinRename).toHaveBeenCalledWith(3, "Standup");
      expect(screen.queryByRole("textbox", { name: "Bookmark name" })).not.toBeInTheDocument();
    });

    it("leaves the name alone when the rename is canceled", async () => {
      await renderBar();
      await rightClick(bookmark("Sprint board"));

      await choose(/^Rename Bookmark/);
      await userEvent.type(await screen.findByRole("textbox", { name: "Bookmark name" }), "Other{Escape}");

      expect(mocked.pinRename).not.toHaveBeenCalled();
      expect(bookmark("Sprint board")).toBeInTheDocument();
    });

    it("offers no Use Original Name on a bookmark without an own name", async () => {
      await renderBar();

      await rightClick(bookmark("Sprint board"));

      expect(within(screen.getByRole("menu")).queryByRole("menuitem", { name: /^Use Original Name/ })).not.toBeInTheDocument();
    });

    it("goes back to the original name of a renamed bookmark", async () => {
      await renderBar([pinItem({ id: 5, title: "Pricing work" })]);
      await rightClick(bookmark("Pricing work"));

      await choose(/^Use Original Name/);

      expect(mocked.pinRename).toHaveBeenCalledWith(5, null);
    });

    it("removes a bookmark", async () => {
      await renderBar();
      await rightClick(bookmark("My open issues"));

      await choose("Remove Bookmark");

      expect(mocked.pinRemove).toHaveBeenCalledWith(2);
    });

    it("moves the bookmarks back to the sidebar", async () => {
      await renderBar();
      await rightClick(bookmark("My open issues"));

      await choose("Show Bookmarks in Sidebar");

      expect(mocked.modeSet).toHaveBeenCalledWith("bookmarksBar", false);
    });

    it("offers the same switch on the bar's empty space", async () => {
      await renderBar([]);

      await rightClick(screen.getByRole("navigation", { name: "Bookmarks" }));
      await choose("Show Bookmarks in Sidebar");

      expect(mocked.modeSet).toHaveBeenCalledWith("bookmarksBar", false);
    });

    it("shows an error toast when a bookmark cannot be removed", async () => {
      mocked.pinRemove.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderBar();
      await rightClick(bookmark("My open issues"));

      await choose("Remove Bookmark");

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not remove the bookmark: Disk full"));
    });
  });

  describe("drag to reorder", () => {
    let rects: { mockRestore: () => void };

    // Bookmarks are 100px wide, side by side from the left by their drag index.
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

    it("moves a bookmark before the one it is dropped in front of", async () => {
      await renderBar();

      drag(bookmark("Sprint board"), 250, 10);

      expect(mocked.pinMove).toHaveBeenCalledWith(3, 1);
    });

    it("moves a bookmark to the end when dropped past the last", async () => {
      await renderBar();

      drag(bookmark("Redesign the pricing page"), 10, 290);

      expect(mocked.pinMove).toHaveBeenCalledWith(1, null);
    });

    it("moves nothing when dropped where it already is", async () => {
      await renderBar();

      drag(bookmark("My open issues"), 110, 130);

      expect(mocked.pinMove).not.toHaveBeenCalled();
    });

    it("swallows the click that ends a drag", async () => {
      await renderBar();
      const item = bookmark("Redesign the pricing page");

      drag(item, 10, 290);
      fireEvent.click(item);

      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("More bookmarks", () => {
    it("lists All Bookmarks and opens the Bookmarks page", async () => {
      await renderBar();
      await waitFor(() => expect(mocked.sidebarTree).toHaveBeenCalled());
      await act(async () => {});

      await userEvent.click(screen.getByRole("button", { name: "More bookmarks" }));
      expect(within(screen.getByRole("menu")).getAllByRole("menuitem")).toHaveLength(1);
      await choose("All Bookmarks");

      expect(mocked.navGo).toHaveBeenCalledWith(3, null);
    });

    describe("in a narrow row", () => {
      beforeEach(() => {
        // Each bookmark is 100px wide and the row 250px: two fit, the gap included.
        vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(250);
        vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
          const width = this.classList.contains("bookmarks-bar__item") ? 100 : 0;
          return { left: 0, right: width, width, top: 0, bottom: 30, height: 30, x: 0, y: 0, toJSON: () => ({}) };
        });
        vi.stubGlobal(
          "ResizeObserver",
          class {
            constructor(private callback: ResizeObserverCallback) {}
            observe() {
              this.callback([], this);
            }
            unobserve() {}
            disconnect() {}
          },
        );
      });

      afterEach(() => {
        vi.restoreAllMocks();
        vi.unstubAllGlobals();
      });

      it("folds the bookmarks that do not fit into the menu", async () => {
        await renderBar();
        await waitFor(() => expect(screen.queryByRole("button", { name: "Sprint board" })).not.toBeInTheDocument());

        expect(bookmark("Redesign the pricing page")).toBeInTheDocument();
        expect(bookmark("My open issues")).toBeInTheDocument();

        await userEvent.click(screen.getByRole("button", { name: "More bookmarks" }));
        const rows = within(screen.getByRole("menu")).getAllByRole("menuitem");
        expect(rows).toHaveLength(2);
        expect(rows[0]).toHaveTextContent("Sprint board");
        await choose("Sprint board");

        expect(mocked.navGo).toHaveBeenCalledWith(21, 7);
      });
    });
  });
});
