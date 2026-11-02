import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Nav } from "../api";
import { MenuHost } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { tab, tabbedNav } from "../test/nav";
import { lumenSpace, sidebarTree, space } from "../test/spaces";
import { listNode, view } from "../test/views";
import { TabStrip } from "./TabStrip";

const mocked = vi.mocked(api, true);

// jsdom may lack PointerEvent; the stub keeps the mouse fields (clientX, buttons) that the drag reads.
window.PointerEvent ??= class extends MouseEvent {
  pointerId = 1;
} as unknown as typeof PointerEvent;

const LIST_TAB = tab(1, { listId: 11, viewId: 101 });
const BOARD_TAB = tab(2, { listId: 11, viewId: 102 });
const LUMEN_TAB = tab(3, { listId: 12, viewId: 301 });
const THREE = [LIST_TAB, BOARD_TAB, LUMEN_TAB];

/** Renders the strip and waits until the tab named by `ready` has its label, i.e. the tree and the views have loaded. */
async function renderStrip(nav: Nav = tabbedNav(THREE, 1), ready: RegExp = /^Website relaunch · Board/) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TabStrip nav={nav} />
      <MenuHost />
    </QueryClientProvider>,
  );
  await screen.findByRole("tab", { name: ready });
}

const auxClick = (target: HTMLElement, button: number) => fireEvent(target, new MouseEvent("auxclick", { bubbles: true, button }));

const tabNamed = (label: string) => screen.getByRole("tab", { name: new RegExp(`^${label}`) });
const rightClick = (target: HTMLElement) => userEvent.pointer({ keys: "[MouseRight]", target });
const choose = (text: string) => userEvent.click(within(screen.getByRole("menu")).getByRole("menuitem", { name: text }));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space({ lists: [listNode({ id: 21, name: "My open issues" })] }), lumenSpace()]));
  mocked.viewsOf.mockImplementation(async (listId) => {
    if (listId === 11) return [view(), view({ id: 102, name: "Board", layout: "board" })];
    return [view({ id: 301, listId })];
  });
});

describe("TabStrip", () => {
  describe("tabs", () => {
    it("names each tab after its level and view, or the level alone when it has one view", async () => {
      await renderStrip();

      expect(tabNamed("Website relaunch · List")).toBeInTheDocument();
      expect(tabNamed("Website relaunch · Board")).toBeInTheDocument();
      expect(tabNamed("Lumen iOS app")).not.toHaveAccessibleName(/·/);
    });

    it("names a filter list's tab after the list", async () => {
      await renderStrip(tabbedNav([LIST_TAB, tab(4, { listId: 21, viewId: 201 })], 1), /^My open issues/);

      expect(tabNamed("My open issues")).toBeInTheDocument();
    });

    it("marks the shown tab selected and no other", async () => {
      await renderStrip(tabbedNav(THREE, 2));

      expect(tabNamed("Website relaunch · Board")).toHaveAttribute("aria-selected", "true");
      expect(tabNamed("Website relaunch · List")).toHaveAttribute("aria-selected", "false");
      expect(tabNamed("Lumen iOS app")).toHaveAttribute("aria-selected", "false");
    });
  });

  describe("showing a tab", () => {
    it("asks Rust to show an inactive tab when it is clicked", async () => {
      await renderStrip();

      await userEvent.click(tabNamed("Lumen iOS app"));

      expect(mocked.navTabActivate).toHaveBeenCalledWith(3);
    });

    it("does nothing when the shown tab is clicked", async () => {
      await renderStrip();

      await userEvent.click(tabNamed("Website relaunch · List"));

      expect(mocked.navTabActivate).not.toHaveBeenCalled();
    });

    it("shows an error toast when the tab cannot be shown", async () => {
      mocked.navTabActivate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderStrip();

      await userEvent.click(tabNamed("Lumen iOS app"));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not show the tab: Disk full"));
    });
  });

  describe("closing a tab", () => {
    it("closes it from its × without showing it first", async () => {
      await renderStrip();

      await userEvent.click(within(tabNamed("Lumen iOS app")).getByRole("button", { name: "Close tab" }));

      expect(mocked.navTabClose).toHaveBeenCalledWith(3);
      expect(mocked.navTabActivate).not.toHaveBeenCalled();
    });

    it("closes it on a middle click", async () => {
      await renderStrip();

      auxClick(tabNamed("Lumen iOS app"), 1);

      expect(mocked.navTabClose).toHaveBeenCalledWith(3);
    });

    it("keeps it on an auxiliary click of another button", async () => {
      await renderStrip();

      auxClick(tabNamed("Lumen iOS app"), 2);

      expect(mocked.navTabClose).not.toHaveBeenCalled();
    });
  });

  describe("context menu", () => {
    it("offers New Tab, Close Tab and Close Other Tabs", async () => {
      await renderStrip();

      await rightClick(tabNamed("Lumen iOS app"));

      expect(within(screen.getByRole("menu", { name: "Tab" })).getAllByRole("menuitem").map((r) => r.textContent)).toEqual(["New Tab", "Close Tab", "Close Other Tabs"]);
    });

    it("opens the tab's place in a new tab and shows it", async () => {
      await renderStrip();
      await rightClick(tabNamed("Website relaunch · Board"));

      await choose("New Tab");

      expect(mocked.navTabNew).toHaveBeenCalledWith(11, 102, true);
    });

    it("closes the tab it was opened on", async () => {
      await renderStrip();
      await rightClick(tabNamed("Website relaunch · Board"));

      await choose("Close Tab");

      expect(mocked.navTabClose).toHaveBeenCalledWith(2);
    });

    it("closes every other tab and shows the one it was opened on", async () => {
      await renderStrip();
      await rightClick(tabNamed("Website relaunch · Board"));

      await choose("Close Other Tabs");

      await waitFor(() => expect(mocked.navTabActivate).toHaveBeenCalledWith(2));
      expect(mocked.navTabClose).toHaveBeenCalledWith(1);
      expect(mocked.navTabClose).toHaveBeenCalledWith(3);
      expect(mocked.navTabClose).not.toHaveBeenCalledWith(2);
    });

    it("greys out Close Other Tabs for the only tab", async () => {
      await renderStrip(tabbedNav([BOARD_TAB], 2));

      await rightClick(tabNamed("Website relaunch · Board"));

      expect(within(screen.getByRole("menu")).getByRole("menuitem", { name: "Close Other Tabs" })).toHaveAttribute("aria-disabled", "true");
    });

    it("greys out New Tab for a tab without a place", async () => {
      await renderStrip(tabbedNav([LIST_TAB, tab(5, null)], 1), /^Website relaunch · List/);

      await rightClick(screen.getAllByRole("tab")[1]);

      expect(within(screen.getByRole("menu")).getByRole("menuitem", { name: "New Tab" })).toHaveAttribute("aria-disabled", "true");
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
      ["the first tab past the second", "Website relaunch · List", 10, 160, 1, 3],
      ["the first tab to the end", "Website relaunch · List", 10, 290, 1, null],
      ["the last tab to the front", "Lumen iOS app", 210, 10, 3, 1],
    ])("moves %s before the tab it is dropped in front of", async (_name, label, from, to, id, before) => {
      await renderStrip();

      drag(tabNamed(label), from, to);

      expect(mocked.navTabMove).toHaveBeenCalledWith(id, before);
    });

    it("moves nothing when dropped where it already is", async () => {
      await renderStrip();

      drag(tabNamed("Website relaunch · Board"), 110, 130);

      expect(mocked.navTabMove).not.toHaveBeenCalled();
    });

    it("swallows the click that ends a drag", async () => {
      await renderStrip();
      const dragged = tabNamed("Lumen iOS app");

      drag(dragged, 210, 10);
      fireEvent.click(dragged);

      expect(mocked.navTabActivate).not.toHaveBeenCalled();
    });
  });
});
