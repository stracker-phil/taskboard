import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type MenuEntry, type View } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { narrowingKey } from "../../state/slices/narrowing";
import { resetStore, useStore } from "../../state/store";
import { issueDetail } from "../../test/issues";
import { connectionView } from "../../test/jira";
import { startMotionTimers, stubMotion } from "../../test/motion";
import { space } from "../../test/spaces";
import { listNode, view } from "../../test/views";
import type { Level } from "../levels";
import { Toolbar } from "./Toolbar";

const mocked = vi.mocked(api, true);

const SPACE: Level = { kind: "space", space: space() };
const FILTER_LIST: Level = { kind: "list", space: space(), node: listNode({ id: 21, name: "My open issues", ownConditions: 2 }) };

function renderToolbar(level: Level = SPACE, current: View = view()) {
  const ui = (v: View) => (
    <QueryClientProvider client={client}>
      <Toolbar level={level} view={v} />
    </QueryClientProvider>
  );
  const client = createQueryClient();
  const result = render(ui(current));
  return { ...result, rerenderWith: (v: View) => result.rerender(ui(v)) };
}

type Entry = Exclude<MenuEntry, "separator">;
const entriesOf = (entries: MenuEntry[]) => entries.filter((e): e is Entry => e !== "separator");

function choose(text: string, submenu?: string) {
  const top = entriesOf(mocked.popupMenu.mock.lastCall![0]);
  const entries = submenu ? entriesOf(top.find((e) => e.text === submenu)!.items!) : top;
  const entry = entries.find((e) => e.text === text);
  if (!entry) throw new Error(`No menu entry "${text}"`);
  return act(async () => entry.run?.());
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.connectionGet.mockResolvedValue(connectionView());
  mocked.listMatchCount.mockResolvedValue({ open: 5, total: 7 });
});

/** Every collapsible item measures 100px; the bar is `barWidth` wide (its 48px padding included). */
function sizeBar(barWidth: number) {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(private readonly callback: () => void) {}
      observe() {
        this.callback();
      }
      unobserve() {}
      disconnect() {}
    },
  );
  spies.push(
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(barWidth),
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      x: 0, y: 0, top: 0, left: 0, right: 100, bottom: 20, width: 100, height: 20, toJSON: () => ({}),
    }),
  );
}

const MORE = { name: "More toolbar items" };
const spies: { mockRestore(): void }[] = [];

afterEach(() => {
  spies.splice(0).forEach((spy) => spy.mockRestore());
  vi.unstubAllGlobals();
});

describe("Toolbar", () => {
  describe("when the bar is narrow", () => {
    const KEY = narrowingKey(11, 101);
    /** Room for Group, Me and the text filter only. */
    const NO_COLUMNS_OR_FILTER = 48 + 250;
    /** Room for the text filter only. */
    const NOTHING_BUT_TEXT = 48 + 200;

    it("has no More button when the bar is wide", () => {
      sizeBar(1200);

      renderToolbar();

      expect(screen.queryByRole("button", MORE)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Columns" })).toBeInTheDocument();
    });

    it("moves Columns, Filter and Group into the More button and keeps Me and the text filter", () => {
      sizeBar(NO_COLUMNS_OR_FILTER);

      renderToolbar(FILTER_LIST);

      expect(screen.getByRole("button", MORE)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Columns" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^Filter/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Group: Status" })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Me" })).toBeInTheDocument();
      expect(screen.getByRole("textbox", { name: "Filter this view" })).toBeInTheDocument();
    });

    it("hides only Columns when the bar is just too narrow", () => {
      sizeBar(48 + 500);

      renderToolbar(FILTER_LIST);

      expect(screen.queryByRole("button", { name: "Columns" })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^Filter/ })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Group: Status" })).toBeInTheDocument();
    });

    it("lists the hidden items in the menu in the order they moved", async () => {
      sizeBar(NOTHING_BUT_TEXT);
      renderToolbar(FILTER_LIST);

      await userEvent.click(screen.getByRole("button", MORE));

      expect(entriesOf(mocked.popupMenu.mock.lastCall![0]).map((e) => e.text)).toEqual([
        "Columns…",
        "Filter · 2…",
        "Group: Status",
        "Only my issues",
      ]);
      expect(screen.queryByRole("button", { name: "Me" })).not.toBeInTheDocument();
    });

    it("leaves Filter out of the menu on a Space", async () => {
      sizeBar(NOTHING_BUT_TEXT);
      renderToolbar(SPACE);

      await userEvent.click(screen.getByRole("button", MORE));

      expect(entriesOf(mocked.popupMenu.mock.lastCall![0]).map((e) => e.text)).toEqual(["Columns…", "Group: Status", "Only my issues"]);
    });

    it("names the Columns entry Card fields on a board", async () => {
      sizeBar(NO_COLUMNS_OR_FILTER);
      renderToolbar(SPACE, view({ layout: "board" }));

      await userEvent.click(screen.getByRole("button", MORE));

      expect(entriesOf(mocked.popupMenu.mock.lastCall![0]).map((e) => e.text)).toContain("Card fields…");
    });

    it("opens the Columns popover when its menu entry is chosen", async () => {
      sizeBar(NO_COLUMNS_OR_FILTER);
      renderToolbar();
      await userEvent.click(screen.getByRole("button", MORE));

      await choose("Columns…");

      expect(await screen.findByRole("dialog", { name: "Columns for this view" })).toBeInTheDocument();
    });

    it("shows the own condition count in the Filter entry and opens the Filter popover", async () => {
      sizeBar(NO_COLUMNS_OR_FILTER);
      renderToolbar(FILTER_LIST);
      await userEvent.click(screen.getByRole("button", MORE));

      await choose("Filter · 2…");

      expect(await screen.findByRole("dialog", { name: "Filter" })).toBeInTheDocument();
    });

    it("turns the Me narrowing on from the menu and shows it checked afterwards", async () => {
      sizeBar(NOTHING_BUT_TEXT);
      renderToolbar();
      await userEvent.click(screen.getByRole("button", MORE));
      expect(entriesOf(mocked.popupMenu.mock.lastCall![0]).find((e) => e.text === "Only my issues")?.checked).toBe(false);

      await choose("Only my issues");
      await userEvent.click(screen.getByRole("button", MORE));

      expect(useStore.getState().narrowings[KEY]).toMatchObject({ me: true });
      expect(entriesOf(mocked.popupMenu.mock.lastCall![0]).find((e) => e.text === "Only my issues")?.checked).toBe(true);
    });

    it("saves the grouping chosen from the Group submenu", async () => {
      mocked.viewSetGroup.mockResolvedValue(view({ groupBy: "epic" }));
      sizeBar(NO_COLUMNS_OR_FILTER);
      renderToolbar();
      await userEvent.click(screen.getByRole("button", MORE));

      await choose("Epic", "Group: Status");

      expect(mocked.viewSetGroup).toHaveBeenCalledWith(101, "epic");
    });
  });

  describe("Group button", () => {
    it("names the view's current grouping", () => {
      renderToolbar(SPACE, view({ groupBy: "status" }));

      expect(screen.getByRole("button", { name: "Group: Status" })).toBeInTheDocument();
    });

    it("opens a menu with the groupings, the current one checked, and a Sort by submenu", async () => {
      renderToolbar(SPACE, view({ groupBy: "status" }));

      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      const entries = entriesOf(mocked.popupMenu.mock.lastCall![0]);
      expect(entries.filter((e) => e.checked).map((e) => e.text)).toEqual(["Status"]);
      expect(entries.map((e) => e.text)).toContain("Epic");
      expect(entries.find((e) => e.text === "Sort by")?.items).toBeDefined();
    });

    it("saves the grouping chosen from the menu to the view", async () => {
      mocked.viewSetGroup.mockResolvedValue(view({ groupBy: "epic" }));
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      await choose("Epic");

      expect(mocked.viewSetGroup).toHaveBeenCalledWith(101, "epic");
    });

    it("saves an ascending sort chosen from the Sort by submenu", async () => {
      mocked.viewSetSort.mockResolvedValue(view());
      renderToolbar(SPACE, view({ sort: { by: "key", descending: true } }));
      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      await choose("Ascending", "Sort by");

      expect(mocked.viewSetSort).toHaveBeenCalledWith(101, { by: "key", descending: false });
    });

    it("shows a toast when the view cannot be changed", async () => {
      mocked.viewSetGroup.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      await choose("Epic");

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change the grouping: Disk full"));
    });
  });

  describe("Filter button", () => {
    it("shows the number of the filter list's own conditions", () => {
      renderToolbar(FILTER_LIST);

      expect(screen.getByLabelText("2 conditions")).toHaveTextContent("2");
    });

    it("uses the singular for one condition", () => {
      renderToolbar({ kind: "list", space: space(), node: listNode({ ownConditions: 1 }) });

      expect(screen.getByLabelText("1 condition")).toBeInTheDocument();
    });

    it("is not offered on a Space, whose filters live in its filter lists", () => {
      renderToolbar(SPACE);

      expect(screen.queryByRole("button", { name: /^Filter/ })).not.toBeInTheDocument();
      expect(screen.queryByLabelText(/condition/)).not.toBeInTheDocument();
    });

    it("is offered on Everything", () => {
      renderToolbar({ kind: "home", node: listNode({ id: 1, name: "Everything" }) });

      expect(screen.getByRole("button", { name: /^Filter/ })).toBeInTheDocument();
    });
  });

  describe("popovers", () => {
    it("labels the columns button Card fields on a board", () => {
      renderToolbar(SPACE, view({ layout: "board" }));

      expect(screen.getByRole("button", { name: "Card fields" })).toBeInTheDocument();
    });

    it("opens the Columns popover and marks its button expanded", async () => {
      renderToolbar();
      const button = screen.getByRole("button", { name: "Columns" });
      expect(button).toHaveAttribute("aria-expanded", "false");

      await userEvent.click(button);

      expect(await screen.findByRole("dialog", { name: "Columns for this view" })).toBeInTheDocument();
      expect(button).toHaveAttribute("aria-expanded", "true");
    });

    it("closes the Columns popover when its button is pressed again", async () => {
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Columns" }));

      await userEvent.click(screen.getByRole("button", { name: "Columns" }));

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Columns" })).toHaveAttribute("aria-expanded", "false");
    });

    it("keeps only one popover open: Filter replaces Columns", async () => {
      renderToolbar(FILTER_LIST);
      await userEvent.click(screen.getByRole("button", { name: "Columns" }));

      await userEvent.click(screen.getByRole("button", { name: /^Filter/ }));

      expect(await screen.findByRole("dialog", { name: "Filter" })).toBeInTheDocument();
      expect(screen.queryByRole("dialog", { name: "Columns for this view" })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^Filter/ })).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByRole("button", { name: "Columns" })).toHaveAttribute("aria-expanded", "false");
    });
  });

  describe("popovers with motion", () => {
    afterEach(() => {
      vi.useRealTimers();
    });

    it("keeps the Columns popover on Escape inside an inert wrapper, and removes it once the time has passed", async () => {
      spies.push(stubMotion({ "--motion-base": "200ms" }));
      startMotionTimers();
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Columns" }));
      const dialog = await screen.findByRole("dialog", { name: "Columns for this view" });
      expect(dialog.closest("[inert]")).toBeNull();

      await userEvent.keyboard("{Escape}");

      const leaving = screen.getByRole("dialog", { name: "Columns for this view", hidden: true });
      expect(leaving.closest("[inert]")).not.toBeNull();
      expect(screen.getByRole("button", { name: "Columns" })).toHaveAttribute("aria-expanded", "false");
      await act(() => vi.advanceTimersByTimeAsync(400));
      expect(screen.queryByRole("dialog", { hidden: true })).not.toBeInTheDocument();
    });
  });

  describe("Me chip", () => {
    const KEY = narrowingKey(11, 101);

    it("starts off as a plain Me button", () => {
      renderToolbar();

      expect(screen.getByRole("button", { name: "Me" })).toHaveAttribute("aria-pressed", "false");
      expect(useStore.getState().narrowings[KEY]).toBeUndefined();
    });

    it("turns on with the connected account's initials and narrows the view to me", async () => {
      renderToolbar();

      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      const chip = screen.getByRole("button", { name: "Only my issues, click to show all" });
      expect(chip).toHaveAttribute("aria-pressed", "true");
      expect(await screen.findByText("AM")).toBeInTheDocument();
      expect(useStore.getState().narrowings[KEY]).toMatchObject({ me: true });
    });

    it("turns off again when the chip is clicked", async () => {
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      await userEvent.click(screen.getByRole("button", { name: "Only my issues, click to show all" }));

      expect(screen.getByRole("button", { name: "Me" })).toHaveAttribute("aria-pressed", "false");
      expect(useStore.getState().narrowings[KEY]?.me).toBe(false);
    });

    it("is remembered per view", async () => {
      const { rerenderWith } = renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      rerenderWith(view({ id: 102, name: "Board" }));

      expect(screen.getByRole("button", { name: "Me" })).toHaveAttribute("aria-pressed", "false");
    });
  });

  describe("epic chip", () => {
    const KEY = narrowingKey(11, 101);
    const CHIP = { name: "Only issues of NW-1001 Checkout, click to show all" };

    beforeEach(() => {
      mocked.issueGet.mockResolvedValue(issueDetail({ id: "10001", key: "NW-1001", summary: "Checkout", isEpic: true, epic: null }));
    });

    it("is not shown without an epic narrowing", () => {
      renderToolbar();

      expect(screen.queryByRole("button", { name: /^Only issues of/ })).not.toBeInTheDocument();
    });

    it("shows the epic's summary while the view is narrowed to the epic", async () => {
      useStore.getState().setNarrowing(KEY, { epic: "10001" });

      renderToolbar();

      expect(await screen.findByRole("button", CHIP)).toHaveTextContent("Checkout");
      expect(mocked.issueGet).toHaveBeenCalledWith("10001");
    });

    it("clears the epic narrowing when clicked", async () => {
      useStore.getState().setNarrowing(KEY, { me: true, epic: "10001" });
      renderToolbar();

      await userEvent.click(await screen.findByRole("button", CHIP));

      expect(useStore.getState().narrowings[KEY]).toMatchObject({ me: true, epic: null });
      expect(screen.queryByRole("button", { name: /^Only issues of/ })).not.toBeInTheDocument();
    });
  });

  describe("text filter", () => {
    it("narrows the view after typing and keeps the Me setting", async () => {
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Filter this view" }), "pricing");

      await waitFor(() => expect(useStore.getState().narrowings[narrowingKey(11, 101)]).toMatchObject({ me: true, text: "pricing" }));
    });

    it("shows the text already set for the view", () => {
      useStore.getState().setNarrowing(narrowingKey(11, 101), { text: "footer" });

      renderToolbar();

      expect(screen.getByRole("textbox", { name: "Filter this view" })).toHaveValue("footer");
    });

    it("starts empty on a different view", () => {
      useStore.getState().setNarrowing(narrowingKey(11, 101), { text: "footer" });
      const { rerenderWith } = renderToolbar();

      rerenderWith(view({ id: 102, name: "Board" }));

      expect(screen.getByRole("textbox", { name: "Filter this view" })).toHaveValue("");
    });
  });
});
