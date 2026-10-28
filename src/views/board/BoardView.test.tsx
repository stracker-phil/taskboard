import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type BoardData } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { boardColumn, boardData } from "../../test/board";
import { ALEX, row, SAM } from "../../test/issues";
import { issueTarget, pinItem, pins } from "../../test/pins";
import { view } from "../../test/views";
import { BoardView } from "./BoardView";

const mocked = vi.mocked(api, true);

const BOARD = view({ layout: "board" });
const PRICING = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page" });
const NOTES = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes", style: "done", statusName: "Done" });
const TODO = boardColumn({ statusId: "1", name: "To Do", style: "todo" });
const PROGRESS = boardColumn({ statusId: "3", name: "In Progress", style: "progress", ids: ["1"] });
const DONE = boardColumn({ statusId: "5", name: "Done", style: "done", ids: ["2"] });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.viewSetColumnCollapsed.mockResolvedValue(BOARD);
  mocked.pinsList.mockResolvedValue(pins());
  mocked.copyText.mockResolvedValue(undefined);
});

async function renderBoard(data: BoardData) {
  mocked.viewBoard.mockResolvedValue(data);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <BoardView view={BOARD} />
    </QueryClientProvider>,
  );
  await screen.findByRole("list", { name: "Board" });
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

  it("renders no columns for an empty board", async () => {
    mocked.viewBoard.mockResolvedValue(boardData([]));
    const { container } = render(
      <QueryClientProvider client={createQueryClient()}>
        <BoardView view={BOARD} />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(container.firstChild).not.toBeNull());
    expect(screen.queryByRole("listitem")).not.toBeInTheDocument();
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

  describe("hover toolbar", () => {
    it("copies the key without opening the card", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      await userEvent.click(screen.getByRole("button", { name: "Copy NW-1042" }));

      expect(mocked.copyText).toHaveBeenCalledWith("NW-1042");
      expect(useStore.getState().issueView).toBeNull();
    });

    it("pins an unpinned issue without opening the card", async () => {
      await renderBoard(boardData([PROGRESS], [PRICING]));

      await userEvent.click(screen.getByRole("button", { name: "Pin NW-1042" }));

      expect(mocked.pinToggleIssue).toHaveBeenCalledWith("1");
      expect(useStore.getState().issueView).toBeNull();
    });

    it("offers Unpin for a pinned issue", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ target: issueTarget({ id: "1", key: "NW-1042" }) })]));
      await renderBoard(boardData([PROGRESS], [PRICING]));

      expect(await screen.findByRole("button", { name: "Unpin NW-1042" })).toHaveAttribute("aria-pressed", "true");
    });

    it("offers Mark seen only on a changed card, and marks it seen", async () => {
      await renderBoard(boardData([PROGRESS, DONE], [{ ...PRICING, changed: true }, NOTES]));

      await userEvent.click(screen.getByRole("button", { name: "Mark NW-1042 seen" }));

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["1"]);
      expect(screen.queryByRole("button", { name: "Mark NW-1001 seen" })).not.toBeInTheDocument();
    });
  });
});
