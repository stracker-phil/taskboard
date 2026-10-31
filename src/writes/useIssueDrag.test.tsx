import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type DropTarget, type GroupMark, type TransitionOption, type View } from "../api";
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
const target = (key: string, action: string, mark: GroupMark | null = null): DropTarget => ({ key, label: key, action, mark });
const answer = (targets: DropTarget[], note: string | null = null, refused: string[] = []) =>
  mocked.issueDropTargets.mockResolvedValue({ targets, refused, note });
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);
const overlay = () => screen.queryByRole("list", { name: "Places to move NW-1042 to" });
const tileTexts = async () => (await within(await screen.findByRole("list", { name: "Places to move NW-1042 to" })).findAllByRole("listitem")).map((t) => t.textContent);
const tile = async (action: string) => (await screen.findByText(action)).closest<HTMLElement>("[role=listitem]")!;
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
      expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "status", "a", [
        expect.objectContaining({ key: "a", label: "Alpha" }),
        expect.objectContaining({ key: "b", label: "Beta" }),
        expect.objectContaining({ key: "c", label: "Gamma" }),
      ]);
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

  describe("the drop overlay", () => {
    it("says it is asking Jira until Rust answers", async () => {
      mocked.issueDropTargets.mockReturnValue(new Promise(() => {}));
      await renderList();

      dragPricing();

      expect(await screen.findByText("Asking Jira where NW-1042 can go…")).toBeInTheDocument();
      expect(overlay()).toBeInTheDocument();
    });

    it("offers a tile with the action text for each target", async () => {
      answer([target("b", "Set status to Beta"), target("c", "Set status to Gamma")]);
      await renderList();

      dragPricing();

      expect(await tileTexts()).toEqual(["Set status to Beta", "Set status to Gamma"]);
    });

    it("keeps the tiles in the order Rust answered", async () => {
      answer([target("c", "Set status to Gamma"), target("x", "Assign to Sam Roe"), target("b", "Set status to Beta")]);
      await renderList();

      dragPricing();

      expect(await tileTexts()).toEqual(["Set status to Gamma", "Assign to Sam Roe", "Set status to Beta"]);
    });

    it("names the groups Rust refused", async () => {
      answer([target("b", "Set status to Beta")], null, ["Gamma", "None"]);
      await renderList();

      dragPricing();

      expect(await screen.findByText("Not allowed here: Gamma, None")).toBeInTheDocument();
    });

    it("leaves out the refused line when nothing is refused", async () => {
      answer([target("b", "Set status to Beta")]);
      await renderList();

      dragPricing();

      await tile("Set status to Beta");
      expect(screen.queryByText(/Not allowed here/)).not.toBeInTheDocument();
    });

    it("shows Rust's note when there are no targets", async () => {
      answer([], "Resolve the blocking issue first.");
      await renderList();

      dragPricing();

      expect(await screen.findByText("Resolve the blocking issue first.")).toBeInTheDocument();
      expect(within(overlay()!).queryAllByRole("listitem")).toHaveLength(0);
    });

    it("says Jira does not let the issue move when there are no targets and no note", async () => {
      answer([]);
      await renderList();

      dragPricing();

      expect(await screen.findByText("Jira doesn't let NW-1042 move anywhere from here.")).toBeInTheDocument();
    });

    it.each<[string, GroupMark | null, string]>([
      ["a status", { kind: "status", style: "review" }, "drop-tile--status-review"],
      ["a person", { kind: "person", person: { accountId: "u1", name: "Jamie Doe", initials: "JD", color: 3 } }, "drop-tile--avatar-3"],
      ["a priority", { kind: "priority", level: 2 }, "drop-tile--priority-2"],
      ["a priority level above the scale", { kind: "priority", level: 5 }, "drop-tile--priority-4"],
      ["no mark", null, "drop-tile--plain"],
    ])("outlines the tile by the color of %s", async (_name, mark, expected) => {
      answer([target("b", "Move it over here", mark)]);
      await renderList();

      dragPricing();

      expect(await tile("Move it over here")).toHaveClass(expected);
    });

    it("removes the overlay once the drag is over", async () => {
      answer([target("b", "Set status to Beta")]);
      await renderList();
      dragPricing();
      await tile("Set status to Beta");

      pointer.release();

      expect(overlay()).not.toBeInTheDocument();
    });
  });

  describe("dropping", () => {
    async function dragOverTile(targets: DropTarget[] = [target("b", "Set status to Beta")]) {
      answer(targets);
      await renderList();
      dragPricing();
      const over = await tile(targets[0].action);
      pointer.over(over);
      return over;
    }

    it("moves the issue onto the tile under the pointer and confirms with a Change back action", async () => {
      mocked.issueDrop.mockResolvedValue({
        kind: "done",
        outcome: { done: "Moved NW-1042 to In Review", reverse: { label: "Change back to In Progress", change: { kind: "assign", accountId: null } } },
      });
      await dragOverTile();

      pointer.release();

      expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "b");
      await waitFor(() => expect(toasts()).toEqual([["success", "Moved NW-1042 to In Review"]]));
      expect(useStore.getState().toasts[0].action?.label).toBe("Change back to In Progress");
    });

    it("opens a status menu with the moves when Jira needs a choice", async () => {
      mocked.issueDrop.mockResolvedValue({ kind: "choose", moves: [REVIEW, TESTING] });
      await dragOverTile();

      pointer.release();

      const menu = await screen.findByRole("menu", { name: "Status of NW-1042" });
      expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["In Review", "In Testing"]);
    });

    it("shows an error toast when the drop fails", async () => {
      mocked.issueDrop.mockRejectedValue({ kind: "invalid_input", message: "The issue needs a resolution", code: null });
      await dragOverTile();

      pointer.release();

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not move the issue: The issue needs a resolution"]]));
    });

    it("does nothing when released over the overlay but not over a tile", async () => {
      await dragOverTile();
      pointer.over(screen.getByRole("list", { name: "Places to move NW-1042 to" }));
      pointer.moveTo(50, 50);

      pointer.release(50, 50);

      expect(mocked.issueDrop).not.toHaveBeenCalled();
      expect(overlay()).not.toBeInTheDocument();
    });

    it("does nothing when released over nothing", async () => {
      await dragOverTile();
      pointer.over(null);
      pointer.moveTo(50, 50);

      pointer.release(50, 50);

      expect(mocked.issueDrop).not.toHaveBeenCalled();
      expect(ghostShowing("Redesign the pricing page")).toBe(false);
      expect(overlay()).not.toBeInTheDocument();
    });

    it("cancels on Escape: no ghost, no overlay and no drop on release", async () => {
      await dragOverTile();

      fireEvent.keyDown(window, { key: "Escape" });
      pointer.release();

      expect(ghostShowing("Redesign the pricing page")).toBe(false);
      expect(overlay()).not.toBeInTheDocument();
      expect(mocked.issueDrop).not.toHaveBeenCalled();
    });

    it("does not select the row under the pointer with the click that ends the drag", async () => {
      await dragOverTile();
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
    expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "assignee", "3", [
      { key: "1", label: "To Do" },
      { key: "3", label: "In Progress" },
      { key: "5", label: "Done" },
    ]);
  });

  it("asks by status when the board groups by nothing", async () => {
    await renderBoard(view({ layout: "board", groupBy: "none" }));

    pointer.press(card());
    pointer.moveTo(40, 40);

    expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "status", "3", expect.any(Array));
  });

  it("offers tiles for the answer's places and names the columns Rust refused", async () => {
    answer([target("5", "Set status to Done")], null, ["To Do"]);
    await renderBoard();

    pointer.press(card());
    pointer.moveTo(40, 40);

    expect(await tileTexts()).toEqual(["Set status to Done"]);
    expect(screen.getByText("Not allowed here: To Do")).toBeInTheDocument();
  });

  it("moves the card onto the tile it is released over", async () => {
    answer([target("5", "Set status to Done")]);
    await renderBoard();
    pointer.press(card());
    pointer.moveTo(40, 40);
    pointer.over(await tile("Set status to Done"));

    pointer.release();

    expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "5");
  });

  it("does nothing when released over a column of the board instead of a tile", async () => {
    answer([target("5", "Set status to Done")]);
    await renderBoard();
    pointer.press(card());
    pointer.moveTo(40, 40);
    await tile("Set status to Done");
    pointer.over(column("To Do"));

    pointer.release();

    expect(mocked.issueDrop).not.toHaveBeenCalled();
    expect(overlay()).not.toBeInTheDocument();
  });

  it("does not start without changes to Jira", async () => {
    allowChanges(false);
    await renderBoard();

    pointer.press(card());
    pointer.moveTo(40, 40);

    expect(mocked.issueDropTargets).not.toHaveBeenCalled();
  });
});
