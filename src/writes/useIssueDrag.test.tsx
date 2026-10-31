import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type DropTargets, type TransitionOption, type View } from "../api";
import { MenuHost } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { NO_NARROWING } from "../queries/views";
import { resetStore, useStore } from "../state/store";
import { boardColumn, boardData } from "../test/board";
import { pointer } from "../test/drag";
import { groupOf, row, viewData } from "../test/issues";
import { connectionView } from "../test/jira";
import { pins } from "../test/pins";
import { behaviour, settings } from "../test/settings";
import { sidebarTree } from "../test/spaces";
import { view } from "../test/views";
import { BoardView } from "../views/board/BoardView";
import { ListView } from "../views/list/ListView";

const mocked = vi.mocked(api, true);

const PRICING = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page" });
const FOOTER = row({ id: "3", key: "NW-1003", summary: "Fix the footer links" });
const MIGRATION = row({ id: "4", key: "NW-1004", summary: "Plan the content migration" });

const REVIEW: TransitionOption = { id: "31", statusId: "3", statusName: "In Review", style: "review", opensJira: false };
const TESTING: TransitionOption = { id: "32", statusId: "3", statusName: "In Testing", style: "review", opensJira: false };

const allowChanges = (on = true) => mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: on }) }));
const answer = (targets: Partial<DropTargets>) => mocked.issueDropTargets.mockResolvedValue({ any: false, keys: [], ...targets });
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);
const zoneOf = (el: Element) => el.closest("[data-drop-key]")!;
/** The ghost repeats the dragged issue's summary next to the row's own. */
const ghostShowing = (summary: string) => screen.queryAllByText(summary).length > 1;

let spies: { mockRestore: () => void }[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
  pointer.over(null);
  mocked.sidebarTree.mockResolvedValue(sidebarTree([]));
  mocked.pinsList.mockResolvedValue(pins());
  mocked.placeGet.mockResolvedValue(null);
  mocked.placeSet.mockResolvedValue(undefined);
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));
  spies = [
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(1000),
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(800),
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) {
      return this.classList.contains("board-column__cards") ? 1000 : 110;
    }),
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(800),
  ];
});

afterEach(() => {
  spies.forEach((s) => s.mockRestore());
  pointer.reset();
});

describe("dragging in the List", () => {
  const GROUPS = [
    groupOf([PRICING], { key: "a", label: "Alpha" }),
    groupOf([FOOTER], { key: "b", label: "Beta" }),
    groupOf([MIGRATION], { key: "c", label: "Gamma" }),
  ];

  async function renderList(withChanges = true) {
    allowChanges(withChanges);
    mocked.viewRows.mockResolvedValue(viewData(GROUPS, [PRICING, FOOTER, MIGRATION]));
    render(
      <QueryClientProvider client={createQueryClient()}>
        <ListView listId={11} viewId={101} openWith={null} narrowing={NO_NARROWING} />
        <MenuHost />
      </QueryClientProvider>,
    );
    await screen.findByRole("grid");
    if (withChanges) await screen.findByRole("button", { name: "Assign NW-1042 to me or unassign it" });
    else await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
    await act(async () => {});
  }

  const rowOf = (key: string) => screen.getByRole("row", { name: new RegExp(key) });
  /** Presses the pricing row and moves far enough to start dragging, with the pointer over `over`. */
  const dragPricing = (over: Element | null = null) => {
    pointer.press(rowOf("NW-1042"));
    pointer.over(over);
    pointer.moveTo(40, 40);
  };

  describe("starting", () => {
    it("shows a ghost of the issue and asks Rust where it may go once the pointer moved past a few pixels", async () => {
      await renderList();

      dragPricing();

      expect(ghostShowing("Redesign the pricing page")).toBe(true);
      expect(screen.getAllByText("NW-1042").length).toBeGreaterThan(1);
      expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "status");
    });

    it("stays a click when the pointer barely moves", async () => {
      await renderList();

      pointer.press(rowOf("NW-1042"));
      pointer.moveTo(12, 12);
      pointer.release(12, 12);

      expect(ghostShowing("Redesign the pricing page")).toBe(false);
      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });

    it("selects the row on a plain click", async () => {
      await renderList();

      await userEvent.click(rowOf("NW-1042"));

      expect(rowOf("NW-1042")).toHaveAttribute("aria-selected", "true");
      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });

    it("does not start without changes to Jira", async () => {
      await renderList(false);

      dragPricing();

      expect(ghostShowing("Redesign the pricing page")).toBe(false);
      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });

    it.each([["⌘", { metaKey: true }], ["⌃", { ctrlKey: true }], ["⌥", { altKey: true }], ["⇧", { shiftKey: true }]])(
      "does not start while %s is held",
      async (_name, modifier) => {
        await renderList();

        pointer.press(rowOf("NW-1042"), modifier);
        pointer.moveTo(40, 40);

        expect(mocked.issueDropTargets).not.toHaveBeenCalled();
      },
    );

    it("does not start from a button inside the row", async () => {
      await renderList();

      pointer.press(within(rowOf("NW-1042")).getByRole("button", { name: "Copy NW-1042" }));
      pointer.moveTo(40, 40);

      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });
  });

  describe("drop zones", () => {
    it("marks the issue's own group as home and every other group as unavailable until Rust answers", async () => {
      mocked.issueDropTargets.mockReturnValue(new Promise(() => {}));
      await renderList();

      dragPricing();

      expect(zoneOf(rowOf("NW-1042"))).toHaveClass("drop-zone--home");
      expect(zoneOf(rowOf("NW-1003"))).toHaveClass("drop-zone--no");
      expect(zoneOf(rowOf("NW-1004"))).toHaveClass("drop-zone--no");
    });

    it("marks the groups in the answer as ok and the others as unavailable", async () => {
      answer({ keys: ["b"] });
      await renderList();

      dragPricing();

      await waitFor(() => expect(zoneOf(rowOf("NW-1003"))).toHaveClass("drop-zone--ok"));
      expect(zoneOf(rowOf("NW-1042"))).toHaveClass("drop-zone--home");
      expect(zoneOf(rowOf("NW-1004"))).toHaveClass("drop-zone--no");
    });

    it("marks every other group as ok when any group takes the issue", async () => {
      answer({ any: true });
      await renderList();

      dragPricing();

      await waitFor(() => expect(zoneOf(rowOf("NW-1004"))).toHaveClass("drop-zone--ok"));
      expect(zoneOf(rowOf("NW-1003"))).toHaveClass("drop-zone--ok");
      expect(zoneOf(rowOf("NW-1042"))).toHaveClass("drop-zone--home");
    });

    it("marks nothing once the drag is over", async () => {
      answer({ keys: ["b"] });
      await renderList();
      dragPricing();
      await waitFor(() => expect(zoneOf(rowOf("NW-1003"))).toHaveClass("drop-zone--ok"));

      pointer.release();

      expect(zoneOf(rowOf("NW-1003"))).not.toHaveClass("drop-zone--ok");
      expect(zoneOf(rowOf("NW-1004"))).not.toHaveClass("drop-zone--no");
    });
  });

  describe("dropping", () => {
    async function dragOverOk(targets: Partial<DropTargets> = { keys: ["b"] }) {
      answer(targets);
      await renderList();
      const target = rowOf("NW-1003");
      dragPricing(target);
      await waitFor(() => expect(zoneOf(target)).toHaveClass("drop-zone--ok"));
      return target;
    }

    it("moves the issue into the group under the pointer and confirms with a Change back action", async () => {
      mocked.issueDrop.mockResolvedValue({
        kind: "done",
        outcome: { done: "Moved NW-1042 to In Review", reverse: { label: "Change back to In Progress", change: { kind: "assign", accountId: null } } },
      });
      await dragOverOk();

      pointer.release();

      expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "b");
      await waitFor(() => expect(toasts()).toEqual([["success", "Moved NW-1042 to In Review"]]));
      expect(useStore.getState().toasts[0].action?.label).toBe("Change back to In Progress");
    });

    it("opens a status menu with the moves when Jira needs a choice", async () => {
      mocked.issueDrop.mockResolvedValue({ kind: "choose", moves: [REVIEW, TESTING] });
      await dragOverOk();

      pointer.release();

      const menu = await screen.findByRole("menu", { name: "Status of NW-1042" });
      expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["In Review", "In Testing"]);
    });

    it("shows an error toast when the drop fails", async () => {
      mocked.issueDrop.mockRejectedValue({ kind: "invalid_input", message: "The issue needs a resolution", code: null });
      await dragOverOk();

      pointer.release();

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not move the issue: The issue needs a resolution"]]));
    });

    it("does nothing when released over a group that does not take the issue", async () => {
      await dragOverOk();
      pointer.over(rowOf("NW-1004"));
      pointer.moveTo(50, 50);

      pointer.release(50, 50);

      expect(mocked.issueDrop).not.toHaveBeenCalled();
    });

    it("does nothing when released over the issue's own group", async () => {
      await dragOverOk();
      pointer.over(rowOf("NW-1042"));
      pointer.moveTo(50, 50);

      pointer.release(50, 50);

      expect(mocked.issueDrop).not.toHaveBeenCalled();
    });

    it("does nothing when released over nothing", async () => {
      await dragOverOk();
      pointer.over(null);
      pointer.moveTo(50, 50);

      pointer.release(50, 50);

      expect(mocked.issueDrop).not.toHaveBeenCalled();
      expect(ghostShowing("Redesign the pricing page")).toBe(false);
    });

    it("cancels on Escape: no ghost and no drop on release", async () => {
      await dragOverOk();

      fireEvent.keyDown(window, { key: "Escape" });
      pointer.release();

      expect(ghostShowing("Redesign the pricing page")).toBe(false);
      expect(mocked.issueDrop).not.toHaveBeenCalled();
    });

    it("does not select the row under the pointer with the click that ends the drag", async () => {
      await dragOverOk();
      pointer.release();

      fireEvent.click(rowOf("NW-1003"));

      expect(rowOf("NW-1003")).toHaveAttribute("aria-selected", "false");
    });
  });
});

describe("dragging on the Board", () => {
  const TODO = boardColumn({ key: "1", name: "To Do", style: "todo", mark: { kind: "status", style: "todo" } });
  const PROGRESS = boardColumn({ key: "3", name: "In Progress", style: "progress", ids: ["1"] });
  const DONE = boardColumn({ key: "5", name: "Done", style: "done", mark: { kind: "status", style: "done" }, ids: [] });

  async function renderBoard(current: View = view({ layout: "board" })) {
    mocked.viewBoard.mockResolvedValue(boardData([TODO, PROGRESS, DONE], [PRICING]));
    render(
      <QueryClientProvider client={createQueryClient()}>
        <BoardView view={current} />
        <MenuHost />
      </QueryClientProvider>,
    );
    await screen.findByRole("list", { name: "Board" });
    await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
    await act(async () => {});
  }

  const column = (name: string) => screen.getByRole("listitem", { name });
  const card = () => screen.getByText("Redesign the pricing page");

  it("shows a ghost and asks Rust where the card may go, by the view's grouping", async () => {
    await renderBoard(view({ layout: "board", groupBy: "assignee" }));

    pointer.press(card());
    pointer.moveTo(40, 40);

    expect(ghostShowing("Redesign the pricing page")).toBe(true);
    expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "assignee");
  });

  it("asks by status when the board groups by nothing", async () => {
    await renderBoard(view({ layout: "board", groupBy: "none" }));

    pointer.press(card());
    pointer.moveTo(40, 40);

    expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "status");
  });

  it("marks the card's own column as home, the answer's columns as ok and the rest as unavailable", async () => {
    answer({ keys: ["5"] });
    await renderBoard();

    pointer.press(card());
    pointer.moveTo(40, 40);

    await waitFor(() => expect(column("Done")).toHaveClass("drop-zone--ok"));
    expect(column("In Progress")).toHaveClass("drop-zone--home");
    expect(column("To Do")).toHaveClass("drop-zone--no");
  });

  it("moves the card into the column it is released over", async () => {
    answer({ keys: ["5"] });
    await renderBoard();
    pointer.press(card());
    pointer.over(column("Done"));
    pointer.moveTo(40, 40);
    await waitFor(() => expect(column("Done")).toHaveClass("drop-zone--ok"));

    pointer.release();

    expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "5");
  });

  it("does nothing when released over a column that does not take the card", async () => {
    answer({ keys: ["5"] });
    await renderBoard();
    pointer.press(card());
    pointer.over(column("To Do"));
    pointer.moveTo(40, 40);
    await waitFor(() => expect(column("Done")).toHaveClass("drop-zone--ok"));

    pointer.release();

    expect(mocked.issueDrop).not.toHaveBeenCalled();
  });

  it("does not start without changes to Jira", async () => {
    allowChanges(false);
    await renderBoard();

    pointer.press(card());
    pointer.moveTo(40, 40);

    expect(mocked.issueDropTargets).not.toHaveBeenCalled();
  });
});
