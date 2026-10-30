import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type BoardData, type View } from "../../api";
import { installCommands, useCommandKeys } from "../../commands";
import { createQueryClient } from "../../queries/queryClient";
import { narrowingKey } from "../../state/slices/narrowing";
import { resetStore, useStore } from "../../state/store";
import { boardColumn, boardData } from "../../test/board";
import { ALEX, row, SAM } from "../../test/issues";
import { issueTarget, pinItem, pins } from "../../test/pins";
import { sidebarTree, space } from "../../test/spaces";
import { view } from "../../test/views";
import { BoardView } from "./BoardView";

const mocked = vi.mocked(api, true);

const BOARD = view({ layout: "board" });
const PRICING = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page" });
const NOTES = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes", style: "done", statusName: "Done" });
const TODO = boardColumn({ key: "1", name: "To Do", style: "todo", mark: { kind: "status", style: "todo" } });
const PROGRESS = boardColumn({ key: "3", name: "In Progress", style: "progress", ids: ["1"] });
const DONE = boardColumn({ key: "5", name: "Done", style: "done", mark: { kind: "status", style: "done" }, ids: ["2"] });

const CARD_HEIGHT = 110;
let spies: { mockRestore: () => void }[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.sidebarTree.mockResolvedValue(sidebarTree([]));
  mocked.viewSetColumnCollapsed.mockResolvedValue(BOARD);
  mocked.pinsList.mockResolvedValue(pins());
  mocked.copyText.mockResolvedValue(undefined);
  mocked.placeGet.mockResolvedValue(null);
  mocked.placeSet.mockResolvedValue(undefined);
  spies = [
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(1000),
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(800),
    vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) {
      return this.classList.contains("board-column__cards") ? 1000 : CARD_HEIGHT;
    }),
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(800),
  ];
});

afterEach(() => {
  spies.forEach((s) => s.mockRestore());
});

async function renderBoard(data: BoardData, current: View = BOARD) {
  mocked.viewBoard.mockResolvedValue(data);
  const rendered = render(
    <QueryClientProvider client={createQueryClient()}>
      <BoardView view={current} />
    </QueryClientProvider>,
  );
  await screen.findByRole("list", { name: "Board" });
  return rendered;
}

describe("BoardView", () => {
  it("renders nothing until the board has loaded", () => {
    mocked.viewBoard.mockReturnValue(new Promise(() => {}));
    render(
      <QueryClientProvider client={createQueryClient()}>
        <BoardView view={BOARD} />
      </QueryClientProvider>,
    );

    expect(screen.queryByRole("list", { name: "Board" })).not.toBeInTheDocument();
  });

  it("shows what the level is missing instead of columns for an empty board", async () => {
    mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
    mocked.viewBoard.mockResolvedValue(boardData([]));
    render(
      <QueryClientProvider client={createQueryClient()}>
        <BoardView view={BOARD} />
      </QueryClientProvider>,
    );

    expect(await screen.findByRole("heading", { name: "Nothing in Website relaunch" })).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "Board" })).not.toBeInTheDocument();
    expect(screen.queryByRole("article")).not.toBeInTheDocument();
  });

  describe("columns", () => {
    it("lists the columns in the order Rust gives, each with its status pill and card count", async () => {
      await renderBoard(boardData([DONE, TODO, PROGRESS], [PRICING, NOTES]));

      const columns = screen.getAllByRole("listitem");
      expect(columns.map((c) => c.getAttribute("aria-label"))).toEqual(["Done", "To Do", "In Progress"]);
      const progress = screen.getByRole("listitem", { name: "In Progress" });
      expect(within(progress).getByText("In Progress")).toBeInTheDocument();
      expect(within(progress).getByText("1")).toBeInTheDocument();
      expect(within(screen.getByRole("listitem", { name: "To Do" })).getByText("0")).toBeInTheDocument();
    });

    it("shows each card in the column of its status", async () => {
      await renderBoard(boardData([PROGRESS, DONE], [PRICING, NOTES]));

      expect(within(screen.getByRole("listitem", { name: "In Progress" })).getByText("Redesign the pricing page")).toBeInTheDocument();
      expect(within(screen.getByRole("listitem", { name: "Done" })).getByText("Publish the launch notes")).toBeInTheDocument();
    });

    it("skips an id whose row is missing", async () => {
      await renderBoard(boardData([boardColumn({ ids: ["1", "404"] })], [PRICING]));

      expect(screen.getAllByRole("article")).toHaveLength(1);
    });

    it("shows a collapsed column as an Expand control with its name and count, without its cards", async () => {
      await renderBoard(boardData([PROGRESS, { ...DONE, collapsed: true }], [PRICING, NOTES]));

      const strip = screen.getByRole("button", { name: "Expand Done (1)" });
      expect(screen.getByRole("listitem", { name: "Done" })).toContainElement(strip);
      expect(strip).toHaveTextContent("Done");
      expect(strip).toHaveTextContent("1");
      expect(screen.queryByText("Publish the launch notes")).not.toBeInTheDocument();
    });

    it("shows the number of issues of a collapsed column inside its strip", async () => {
      const many = [PRICING, NOTES, row({ id: "3", key: "NW-1003", summary: "Archive the old banners" })];
      await renderBoard(boardData([{ ...PROGRESS, ids: ["1", "2", "3"], collapsed: true }], many));

      const strip = screen.getByRole("button", { name: "Expand In Progress (3)" });
      expect(within(strip).getByText("3")).toBeInTheDocument();
    });

    it("writes a collapsed column's name in the vertical status pill of its strip", async () => {
      await renderBoard(boardData([PROGRESS, { ...DONE, collapsed: true }], [PRICING, NOTES]));

      const pill = within(screen.getByRole("button", { name: "Expand Done (1)" })).getByText("Done");
      expect(pill).toHaveClass("status-pill", "status-pill--done", "status-pill--vertical");
    });

    it("asks to expand a collapsed column when its strip is clicked", async () => {
      await renderBoard(boardData([{ ...DONE, collapsed: true }], [NOTES]));

      await userEvent.click(screen.getByRole("button", { name: "Expand Done (1)" }));

      expect(mocked.viewSetColumnCollapsed).toHaveBeenCalledWith(BOARD.id, "5", false);
    });

    it("asks to collapse an expanded column through its Collapse column button", async () => {
      await renderBoard(boardData([TODO, PROGRESS], [PRICING]));

      await userEvent.click(within(screen.getByRole("listitem", { name: "In Progress" })).getByRole("button", { name: "Collapse column" }));

      expect(mocked.viewSetColumnCollapsed).toHaveBeenCalledWith(BOARD.id, "3", true);
    });
  });

  describe("columns by status type", () => {
    const BACKLOG = boardColumn({ key: "backlog", name: "Backlog", style: "backlog", mark: { kind: "status", style: "backlog" } });
    const IN_PROGRESS = boardColumn({ key: "progress", name: "In progress", style: "progress", ids: ["1"] });

    it("renders each column with the pill and the board class of its status type", async () => {
      await renderBoard(boardData([BACKLOG, IN_PROGRESS], [PRICING]));

      const backlog = screen.getByRole("listitem", { name: "Backlog" });
      expect(backlog).toHaveClass("board--backlog");
      expect(within(backlog).getByText("Backlog", { selector: ".status-pill" })).toHaveClass("status-pill--backlog");
      const progress = screen.getByRole("listitem", { name: "In progress" });
      expect(progress).toHaveClass("board--progress");
      expect(within(progress).getByText("Redesign the pricing page")).toBeInTheDocument();
    });

    it("gives a collapsed column's strip the board class of its status type", async () => {
      await renderBoard(boardData([{ ...BACKLOG, collapsed: true }]));

      expect(screen.getByRole("button", { name: "Expand Backlog (0)" })).toHaveClass("board--backlog");
    });
  });

  describe("columns of another grouping", () => {
    const ALEX_COLUMN = boardColumn({ key: "demo-account-1", name: "Alex Morgan", mark: { kind: "person", person: ALEX }, ids: ["1"] });
    const SAM_COLUMN = boardColumn({ key: "demo-account-2", name: "Sam Lee", mark: { kind: "person", person: SAM }, ids: ["2"] });
    const NOBODY = boardColumn({ key: "none", name: "Unassigned", mark: null, ids: [] });

    it("renders one column per person with their name and avatar, and a neutral pill instead of a status pill", async () => {
      await renderBoard(boardData([ALEX_COLUMN, SAM_COLUMN, NOBODY], [PRICING, NOTES]));

      expect(screen.getAllByRole("listitem").map((c) => c.getAttribute("aria-label"))).toEqual(["Alex Morgan", "Sam Lee", "Unassigned"]);
      const alex = screen.getByRole("listitem", { name: "Alex Morgan" });
      expect(within(alex).getByRole("img", { name: "Alex Morgan" })).toBeInTheDocument();
      expect(within(alex).getByText("Alex Morgan", { selector: ".status-pill" })).toHaveClass("status-pill--todo");
      expect(within(alex).getByText("Redesign the pricing page")).toBeInTheDocument();
    });

    it("shows a column without a mark as its name alone", async () => {
      await renderBoard(boardData([NOBODY]));

      const column = screen.getByRole("listitem", { name: "Unassigned" });
      expect(within(column).getByText("Unassigned")).toHaveClass("status-pill");
      expect(within(column).queryByRole("img")).not.toBeInTheDocument();
    });

    it("writes a collapsed column's name vertically in the neutral pill", async () => {
      await renderBoard(boardData([{ ...ALEX_COLUMN, collapsed: true }], [PRICING]));

      const pill = within(screen.getByRole("button", { name: "Expand Alex Morgan (1)" })).getByText("Alex Morgan", { selector: ".status-pill" });
      expect(pill).toHaveClass("status-pill--vertical");
    });

    it("collapses a column by its group key", async () => {
      await renderBoard(boardData([ALEX_COLUMN, SAM_COLUMN], [PRICING, NOTES]));

      await userEvent.click(within(screen.getByRole("listitem", { name: "Sam Lee" })).getByRole("button", { name: "Collapse column" }));

      expect(mocked.viewSetColumnCollapsed).toHaveBeenCalledWith(BOARD.id, "demo-account-2", true);
    });

    it("expands a collapsed column by its group key", async () => {
      await renderBoard(boardData([{ ...ALEX_COLUMN, collapsed: true }], [PRICING]));

      await userEvent.click(screen.getByRole("button", { name: "Expand Alex Morgan (1)" }));

      expect(mocked.viewSetColumnCollapsed).toHaveBeenCalledWith(BOARD.id, "demo-account-1", false);
    });
  });

  describe("cards", () => {
    const FULL = row({
      id: "1",
      key: "NW-1042",
      summary: "Redesign the pricing page",
      epic: { id: "10001", name: "Checkout", color: 1 },
      assignees: [ALEX, SAM],
      release: "2.4.0",
      priority: { name: "High", level: 2 },
      commentCount: 3,
    });
    const ALL_FIELDS = ["epic", "assignee", "release", "priority"];

    it("always shows the key and the summary", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      const card = screen.getByRole("article", { name: "NW-1042 Redesign the pricing page" });
      expect(within(card).getByText("NW-1042")).toBeInTheDocument();
      expect(within(card).getByText("Redesign the pricing page")).toBeInTheDocument();
    });

    it("shows epic, assignees, release and priority when the card fields list them", async () => {
      await renderBoard(boardData([PROGRESS], [FULL], { cardFields: ALL_FIELDS }));

      const card = screen.getByRole("article");
      expect(within(card).getByText("Checkout")).toBeInTheDocument();
      expect(within(card).getByRole("img", { name: "Alex Morgan" })).toBeInTheDocument();
      expect(within(card).getByRole("img", { name: "Sam Lee" })).toBeInTheDocument();
      expect(within(card).getByText("2.4.0")).toBeInTheDocument();
      expect(within(card).getByRole("img", { name: "High priority" })).toBeInTheDocument();
    });

    it("hides epic, assignees, release and priority when the card fields do not list them", async () => {
      await renderBoard(boardData([PROGRESS], [FULL], { cardFields: [] }));

      const card = screen.getByRole("article");
      expect(within(card).queryByText("Checkout")).not.toBeInTheDocument();
      expect(within(card).queryByRole("img", { name: "Alex Morgan" })).not.toBeInTheDocument();
      expect(within(card).queryByText("2.4.0")).not.toBeInTheDocument();
      expect(within(card).queryByRole("img", { name: "High priority" })).not.toBeInTheDocument();
    });

    it("leaves out the fields a card has no value for", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING], { cardFields: ALL_FIELDS }));

      const card = screen.getByRole("article");
      expect(within(card).queryByRole("img")).not.toBeInTheDocument();
      expect(within(card).queryByText("2.4.0")).not.toBeInTheDocument();
    });

    it("shows only the fields that are listed", async () => {
      await renderBoard(boardData([PROGRESS], [FULL], { cardFields: ["priority"] }));

      const card = screen.getByRole("article");
      expect(within(card).getByRole("img", { name: "High priority" })).toBeInTheDocument();
      expect(within(card).queryByText("Checkout")).not.toBeInTheDocument();
      expect(within(card).queryByText("2.4.0")).not.toBeInTheDocument();
    });

    it("writes the summary before the key", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      const card = screen.getByRole("article");
      const summary = within(card).getByText("Redesign the pricing page");
      const key = within(card).getByText("NW-1042");
      expect(summary.compareDocumentPosition(key) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    describe("labels", () => {
      const LABELLED = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", labels: [{ name: "frontend", tint: 1 }, { name: "design", tint: 2 }] });

      it("shows the issue's labels when the card fields list them", async () => {
        await renderBoard(boardData([PROGRESS], [LABELLED], { cardFields: ["labels"] }), view({ layout: "board", inlineLabels: false }));

        const card = screen.getByRole("article");
        expect(within(card).getByText("frontend")).toBeInTheDocument();
        expect(within(card).getByText("design")).toBeInTheDocument();
      });

      it("shows each label once when the card fields list them and the view is inline", async () => {
        await renderBoard(boardData([PROGRESS], [LABELLED], { cardFields: ["labels"] }), view({ layout: "board", inlineLabels: true }));

        expect(within(screen.getByRole("article")).getAllByText("frontend")).toHaveLength(1);
      });

      it("shows the labels after the title when they are not a card field and the view is inline", async () => {
        await renderBoard(boardData([PROGRESS], [LABELLED], { cardFields: [] }), view({ layout: "board", inlineLabels: true }));

        const card = screen.getByRole("article");
        const summary = within(card).getByText("Redesign the pricing page");
        const label = within(card).getByText("frontend");
        expect(summary.compareDocumentPosition(label) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      });

      it("hides the labels when they are not a card field and the view is not inline", async () => {
        await renderBoard(boardData([PROGRESS], [LABELLED], { cardFields: [] }), view({ layout: "board", inlineLabels: false }));

        const card = screen.getByRole("article");
        expect(within(card).queryByText("frontend")).not.toBeInTheDocument();
        expect(within(card).queryByText("design")).not.toBeInTheDocument();
      });
    });

    it("shows the comment count when there are comments, whatever the card fields", async () => {
      await renderBoard(boardData([PROGRESS], [FULL]));

      expect(within(screen.getByRole("article")).getByLabelText("3 comments")).toBeInTheDocument();
    });

    it("shows no comment count without comments", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      expect(within(screen.getByRole("article")).queryByLabelText(/comment/)).not.toBeInTheDocument();
    });

    it("marks a changed card with the Changed since yesterday dot", async () => {
      await renderBoard(boardData([PROGRESS], [{ ...PRICING, changed: true }]));

      expect(within(screen.getByRole("article")).getByTitle("Changed since yesterday")).toBeInTheDocument();
    });

    it("shows no change dot on an unchanged card", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      expect(screen.queryByTitle("Changed since yesterday")).not.toBeInTheDocument();
    });

    it("makes nothing draggable", async () => {
      await renderBoard(boardData([PROGRESS, DONE], [PRICING, NOTES]));

      expect(document.querySelector("[draggable]")).toBeNull();
    });
  });

  describe("opening a card", () => {
    it("opens the Issue view with the board as its origin when the card is clicked", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      await userEvent.click(screen.getByRole("article"));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: BOARD.listId, viewId: BOARD.id }, stack: ["1"] });
    });

    it("opens the Issue view when Enter is pressed on the focused card", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      screen.getByRole("article").focus();
      await userEvent.keyboard("{Enter}");

      expect(useStore.getState().issueView?.stack).toEqual(["1"]);
    });

    it("does not open the Issue view when Enter is pressed on one of the card's own buttons", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      screen.getByRole("button", { name: "Copy NW-1042" }).focus();
      await userEvent.keyboard("{Enter}");

      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("keys", () => {
    const todo = [row({ id: "1", key: "NW-101", summary: "Draft the sitemap" }), row({ id: "2", key: "NW-102", summary: "Write the footer copy" }), row({ id: "3", key: "NW-103", summary: "Pick the typefaces" })];
    const progress = [row({ id: "4", key: "NW-104", summary: "Build the header" }), row({ id: "5", key: "NW-105", summary: "Wire up the search" })];
    const review = [row({ id: "7", key: "NW-107", summary: "Check the contrast" })];
    const done = [row({ id: "6", key: "NW-106", summary: "Register the domain" })];
    const rows = [...todo, ...progress, ...review, ...done];

    const BACKLOG = boardColumn({ key: "10", name: "Backlog", style: "todo", ids: [] });
    const TODO_COLUMN = boardColumn({ key: "1", name: "To Do", style: "todo", ids: ["1", "2", "3"] });
    const REVIEW_COLUMN = boardColumn({ key: "4", name: "In Review", style: "progress", ids: ["7"], collapsed: true });
    const PROGRESS_COLUMN = boardColumn({ key: "3", name: "In Progress", style: "progress", ids: ["4", "5"] });
    const DONE_COLUMN = boardColumn({ key: "5", name: "Done", style: "done", ids: ["6"] });
    const FULL_BOARD = boardData([BACKLOG, TODO_COLUMN, REVIEW_COLUMN, PROGRESS_COLUMN, DONE_COLUMN], rows);

    function Keys() {
      useCommandKeys();
      return null;
    }

    async function renderKeyed(data: BoardData = FULL_BOARD) {
      mocked.viewBoard.mockResolvedValue(data);
      render(
        <QueryClientProvider client={createQueryClient()}>
          <Keys />
          <BoardView view={BOARD} />
        </QueryClientProvider>,
      );
      await screen.findByRole("list", { name: "Board" });
    }

    const currentKeys = () => screen.queryAllByRole("article", { current: true }).map((card) => within(card).getByText(/^NW-\d+$/).textContent);

    beforeEach(() => {
      installCommands("main");
    });

    describe("moving down and up", () => {
      it.each(["j", "{ArrowDown}"])("%s selects the first card of the first open column, skipping empty ones", async (key) => {
        await renderKeyed();

        await userEvent.keyboard(key);

        expect(currentKeys()).toEqual(["NW-101"]);
      });

      it.each(["k", "{ArrowUp}"])("%s with no card selected also starts on the first card", async (key) => {
        await renderKeyed();

        await userEvent.keyboard(key);

        expect(currentKeys()).toEqual(["NW-101"]);
      });

      it.each([
        ["j", "NW-102"],
        ["{ArrowDown}", "NW-102"],
      ])("%s moves to the next card in the column", async (key, expected) => {
        await renderKeyed();
        await userEvent.keyboard("j");

        await userEvent.keyboard(key);

        expect(currentKeys()).toEqual([expected]);
      });

      it.each(["k", "{ArrowUp}"])("%s moves to the previous card in the column", async (key) => {
        await renderKeyed();
        await userEvent.keyboard("jj");

        await userEvent.keyboard(key);

        expect(currentKeys()).toEqual(["NW-101"]);
      });

      it("stops at the last card of the column", async () => {
        await renderKeyed();

        await userEvent.keyboard("jjjjj");

        expect(currentKeys()).toEqual(["NW-103"]);
      });

      it("stops at the first card of the column", async () => {
        await renderKeyed();
        await userEvent.keyboard("j");

        await userEvent.keyboard("kk");

        expect(currentKeys()).toEqual(["NW-101"]);
      });

      it("selects nothing on a board whose columns are all collapsed or empty", async () => {
        await renderKeyed(boardData([BACKLOG, REVIEW_COLUMN], review));

        await userEvent.keyboard("j");

        expect(currentKeys()).toEqual([]);
      });
    });

    describe("moving between columns", () => {
      it("→ goes to the card at the same place in the next open column, skipping a collapsed column", async () => {
        await renderKeyed();
        await userEvent.keyboard("jj");

        await userEvent.keyboard("{ArrowRight}");

        expect(currentKeys()).toEqual(["NW-105"]);
      });

      it("→ lands on the last card when the next column is shorter", async () => {
        await renderKeyed();
        await userEvent.keyboard("jjj");

        await userEvent.keyboard("{ArrowRight}");

        expect(currentKeys()).toEqual(["NW-105"]);
      });

      it("→ stays in the last column", async () => {
        await renderKeyed();
        await userEvent.keyboard("j{ArrowRight}{ArrowRight}");

        await userEvent.keyboard("{ArrowRight}");

        expect(currentKeys()).toEqual(["NW-106"]);
      });

      it("← goes back to the same place in the previous open column", async () => {
        await renderKeyed();
        await userEvent.keyboard("j{ArrowRight}j");
        expect(currentKeys()).toEqual(["NW-105"]);

        await userEvent.keyboard("{ArrowLeft}");

        expect(currentKeys()).toEqual(["NW-102"]);
      });

      it("← stays in the first column", async () => {
        await renderKeyed();
        await userEvent.keyboard("j");

        await userEvent.keyboard("{ArrowLeft}");

        expect(currentKeys()).toEqual(["NW-101"]);
      });

      it("ignores ← and → while no card is selected", async () => {
        await renderKeyed();

        await userEvent.keyboard("{ArrowRight}{ArrowLeft}");

        expect(currentKeys()).toEqual([]);
      });
    });

    describe("a long column", () => {
      const many = Array.from({ length: 300 }, (_, i) => row({ id: String(i + 1), key: `NW-${1000 + i}`, summary: `Sample task ${i + 1}` }));
      const LONG = boardData([boardColumn({ ids: many.map((r) => r.id) })], many);

      it("mounts only part of its cards, starting with the first", async () => {
        await renderKeyed(LONG);

        const cards = screen.getAllByRole("article");
        expect(cards.length).toBeLessThan(many.length);
        expect(screen.getByRole("article", { name: "NW-1000 Sample task 1" })).toBeInTheDocument();
        expect(screen.queryByRole("article", { name: "NW-1299 Sample task 300" })).not.toBeInTheDocument();
      });

      it("marks the current card while moving down inside the rendered cards", async () => {
        await renderKeyed(LONG);

        await userEvent.keyboard("jjj");

        expect(currentKeys()).toEqual(["NW-1002"]);
      });

    });

    describe("acting on the selected card", () => {
      const ORIGIN = { listId: BOARD.listId, viewId: BOARD.id };

      it.each([
        ["↩", "{Enter}"],
        ["Space", " "],
      ])("%s opens the Issue view for the card", async (_name, key) => {
        await renderKeyed();
        await userEvent.keyboard("jj");

        await userEvent.keyboard(key);

        expect(useStore.getState().issueView).toEqual({ origin: ORIGIN, stack: ["2"] });
      });

      it("P pins the card's issue", async () => {
        await renderKeyed();
        await userEvent.keyboard("jj");

        await userEvent.keyboard("p");

        expect(mocked.pinToggleIssue).toHaveBeenCalledWith("2");
      });

      it("M marks the card's unread issue read", async () => {
        const unread = [row({ id: "1", key: "NW-101", summary: "Draft the sitemap", changed: true })];
        await renderKeyed(boardData([boardColumn({ key: "1", name: "To Do", style: "todo", ids: ["1"] })], unread));
        await userEvent.keyboard("j");

        await userEvent.keyboard("m");

        expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["1"]);
        expect(mocked.issueMarkUnread).not.toHaveBeenCalled();
      });

      it("M marks the card's read issue unread", async () => {
        await renderKeyed();
        await userEvent.keyboard("j");

        await userEvent.keyboard("m");

        expect(mocked.issueMarkUnread).toHaveBeenCalledWith("1");
        expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      });

      it("⌘C copies the card's key", async () => {
        await renderKeyed();
        await userEvent.keyboard("jj");

        await userEvent.keyboard("{Meta>}c{/Meta}");

        expect(mocked.copyText).toHaveBeenCalledWith("NW-102");
      });

      it.each([
        ["↩", "{Enter}"],
        ["Space", " "],
        ["P", "p"],
        ["M", "m"],
        ["⌘C", "{Meta>}c{/Meta}"],
      ])("%s does nothing while no card is selected", async (_name, key) => {
        await renderKeyed();

        await userEvent.keyboard(key);

        expect(useStore.getState().issueView).toBeNull();
        expect(mocked.pinToggleIssue).not.toHaveBeenCalled();
        expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
        expect(mocked.issueMarkUnread).not.toHaveBeenCalled();
        expect(mocked.copyText).not.toHaveBeenCalled();
      });
    });
  });

  describe("the current card", () => {
    const PLACE_KEY = narrowingKey(BOARD.listId, BOARD.id);
    const currentCards = () => screen.queryAllByRole("article", { current: true }).map((card) => within(card).getByText(/^NW-\d+$/).textContent);

    function Keys() {
      useCommandKeys();
      return null;
    }

    beforeEach(() => {
      installCommands("main");
    });

    it("makes a clicked card current, remembers it in the view's place and opens it", async () => {
      await renderBoard(boardData([PROGRESS, DONE], [PRICING, NOTES]));

      await userEvent.click(screen.getByRole("article", { name: /NW-1001/ }));

      expect(currentCards()).toEqual(["NW-1001"]);
      expect(useStore.getState().places[PLACE_KEY]?.selectedId).toBe("2");
      expect(useStore.getState().issueView?.stack).toEqual(["2"]);
    });

    it("moves the current card with J and keeps it in the view's place", async () => {
      const column = boardColumn({ key: "3", name: "In Progress", style: "progress", ids: ["1", "2"] });
      mocked.viewBoard.mockResolvedValue(boardData([column], [PRICING, NOTES]));
      render(
        <QueryClientProvider client={createQueryClient()}>
          <Keys />
          <BoardView view={BOARD} />
        </QueryClientProvider>,
      );
      await screen.findByRole("list", { name: "Board" });

      await userEvent.keyboard("jj");

      expect(currentCards()).toEqual(["NW-1001"]);
      expect(useStore.getState().places[PLACE_KEY]?.selectedId).toBe("2");
    });

    it("marks the same card current after the Board is shown again", async () => {
      const data = boardData([PROGRESS, DONE], [PRICING, NOTES]);
      const first = await renderBoard(data);
      await userEvent.click(screen.getByRole("article", { name: /NW-1001/ }));
      first.unmount();

      await renderBoard(data);

      await waitFor(() => expect(currentCards()).toEqual(["NW-1001"]));
    });

    it("marks the card saved by Rust current on the first render", async () => {
      mocked.placeGet.mockResolvedValue({ scrollTop: 0, toggled: {}, collapsedParents: [], selectedId: "1", peekOpen: false });

      await renderBoard(boardData([PROGRESS, DONE], [PRICING, NOTES]));

      await waitFor(() => expect(currentCards()).toEqual(["NW-1042"]));
      expect(mocked.placeGet).toHaveBeenCalledWith(BOARD.listId, BOARD.id);
    });
  });

  describe("the key on a card", () => {
    it("copies the key and says so in a toast, without opening the card", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      await userEvent.click(within(screen.getByRole("article")).getByText("NW-1042"));

      expect(mocked.copyText).toHaveBeenCalledWith("NW-1042");
      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Copied NW-1042"));
      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("hover toolbar", () => {
    it("copies the key without opening the card", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      await userEvent.click(screen.getByRole("button", { name: "Copy NW-1042" }));

      expect(mocked.copyText).toHaveBeenCalledWith("NW-1042");
      expect(useStore.getState().issueView).toBeNull();
    });

    it("bookmarks an issue without opening the card", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      await userEvent.click(screen.getByRole("button", { name: "Bookmark NW-1042" }));

      expect(mocked.pinToggleIssue).toHaveBeenCalledWith("1");
      expect(useStore.getState().issueView).toBeNull();
    });

    it("offers Remove the bookmark of a bookmarked issue", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ target: issueTarget({ id: "1", key: "NW-1042" }) })]));
      await renderBoard(boardData([PROGRESS], [PRICING]));

      expect(await screen.findByRole("button", { name: "Remove the bookmark of NW-1042" })).toHaveAttribute("aria-pressed", "true");
    });

    it("offers Mark as read on an unread card and Mark as unread on a read one", async () => {
      await renderBoard(boardData([PROGRESS, DONE], [{ ...PRICING, changed: true }, NOTES]));

      expect(screen.getByRole("button", { name: "Mark NW-1042 as read" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Mark NW-1042 as unread" })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Mark NW-1001 as unread" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Mark NW-1001 as read" })).not.toBeInTheDocument();
    });

    it("marks an unread card read", async () => {
      await renderBoard(boardData([PROGRESS, DONE], [{ ...PRICING, changed: true }, NOTES]));

      await userEvent.click(screen.getByRole("button", { name: "Mark NW-1042 as read" }));

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["1"]);
    });

    it("marks a read card unread without opening it", async () => {
      await renderBoard(boardData([PROGRESS, DONE], [{ ...PRICING, changed: true }, NOTES]));

      await userEvent.click(screen.getByRole("button", { name: "Mark NW-1001 as unread" }));

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("2");
      expect(useStore.getState().issueView).toBeNull();
    });
  });
});
