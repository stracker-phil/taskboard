import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type DropTarget, type GroupMark, type TransitionOption, type View, type ViewData } from "../api";
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
import { TableView } from "../views/table/TableView";
import { colorClass } from "./DropZone";

const mocked = vi.mocked(api, true);

const PRICING = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page" });
const FOOTER = row({ id: "3", key: "NW-1003", summary: "Fix the footer links" });
const MIGRATION = row({ id: "4", key: "NW-1004", summary: "Plan the content migration" });

const REVIEW: TransitionOption = { id: "31", statusId: "3", statusName: "In Review", style: "review", opensJira: false };
const TESTING: TransitionOption = { id: "32", statusId: "3", statusName: "In Testing", style: "review", opensJira: false };

const allowChanges = (on = true) => mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: on }) }));
const target = (key: string, action: string, mark: GroupMark | null = null, label = key): DropTarget => ({ key, label, action, mark });
const answer = (targets: DropTarget[], note: string | null = null) => mocked.issueDropTargets.mockResolvedValue({ targets, note });
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

const copyOf = () => document.body.querySelector<HTMLElement>(".drag-copy");
const zones = (state?: "ok" | "no" | "wait") => [...document.querySelectorAll<HTMLElement>(state ? `.drop-zone--${state}` : ".drop-zone")];
const tray = () => document.querySelector<HTMLElement>(".drop-tray");
const chips = () => [...document.querySelectorAll<HTMLElement>(".drop-chip")];
const answered = () => waitFor(() => expect(zones("wait")).toHaveLength(0));

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
    vi.spyOn(window, "requestAnimationFrame").mockReturnValue(1),
  ];
});

afterEach(() => {
  fireEvent.keyDown(window, { key: "Escape" });
  spies.forEach((s) => s.mockRestore());
  pointer.reset();
});

describe("dragging in the List", () => {
  const GROUPS = [
    groupOf([PRICING], { key: "a", label: "Alpha" }),
    groupOf([FOOTER], { key: "b", label: "Beta" }),
    groupOf([MIGRATION], { key: "c", label: "Gamma" }),
  ];

  async function renderList(data: ViewData = viewData(GROUPS, [PRICING, FOOTER, MIGRATION]), withChanges = true) {
    allowChanges(withChanges);
    mocked.viewRows.mockResolvedValue(data);
    render(
      <QueryClientProvider client={createQueryClient()}>
        <ListView listId={11} viewId={101} openWith={null} narrowing={NO_NARROWING} />
        <MenuHost />
      </QueryClientProvider>,
    );
    await screen.findByRole("grid");
    await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
    await act(async () => {
      await mocked.getSettings.mock.results[0]?.value;
    });
  }

  const rowOf = (key: string) => screen.getByRole("row", { name: new RegExp(key) });
  const dragPricing = () => {
    pointer.press(rowOf("NW-1042"));
    pointer.moveTo(40, 40);
  };

  describe("starting", () => {
    it("asks Rust where the issue may go once the pointer moved past a few pixels", async () => {
      await renderList();

      dragPricing();

      expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "status", "a", [
        expect.objectContaining({ key: "a", label: "Alpha" }),
        expect.objectContaining({ key: "b", label: "Beta" }),
        expect.objectContaining({ key: "c", label: "Gamma" }),
      ]);
    });

    it("keeps the row in the folded view, marked as the drag's source, and follows the pointer with an aria-hidden copy on the body", async () => {
      await renderList();

      dragPricing();

      expect(rowOf("NW-1042").closest(".list-item")).toHaveAttribute("data-drag-source");
      expect(rowOf("NW-1042").closest(".list-scroll__space")).not.toBeNull();
      expect(rowOf("NW-1003").closest("[data-drag-source]")).toBeNull();
      const copy = copyOf()!;
      expect(copy.parentElement).toBe(document.body);
      expect(copy).toHaveAttribute("aria-hidden", "true");
      expect(copy).toHaveTextContent("Redesign the pricing page");
    });

    it("stays a click when the pointer barely moves", async () => {
      await renderList();

      pointer.press(rowOf("NW-1042"));
      pointer.moveTo(12, 12);
      pointer.release(12, 12);

      expect(copyOf()).toBeNull();
      expect(rowOf("NW-1042").closest("[data-drag-source]")).toBeNull();
      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });

    it("selects the row on a plain click", async () => {
      await renderList();

      await userEvent.click(rowOf("NW-1042"));

      expect(rowOf("NW-1042")).toHaveAttribute("aria-selected", "true");
      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });

    it("does not start without changes to Jira", async () => {
      await renderList(undefined, false);

      dragPricing();

      expect(copyOf()).toBeNull();
      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });

    it.each([["⌘", { metaKey: true }], ["⌃", { ctrlKey: true }], ["⌥", { altKey: true }], ["⇧", { shiftKey: true }]])(
      "does not start while %s is held",
      async (_name, modifier) => {
        await renderList();

        pointer.press(rowOf("NW-1042"), modifier);
        pointer.moveTo(40, 40);

        expect(copyOf()).toBeNull();
        expect(mocked.issueDropTargets).not.toHaveBeenCalled();
      },
    );

    it("does not start from a button inside the row", async () => {
      await renderList();

      pointer.press(within(rowOf("NW-1042")).getByRole("button", { name: "Copy NW-1042" }));
      pointer.moveTo(40, 40);

      expect(copyOf()).toBeNull();
      expect(mocked.issueDropTargets).not.toHaveBeenCalled();
    });

    it("removes the copy and the source mark when the pointer is released", async () => {
      await renderList();
      dragPricing();

      pointer.release();

      expect(copyOf()).toBeNull();
      expect(rowOf("NW-1042").closest("[data-drag-source]")).toBeNull();
    });

    it("removes the copy and the source mark on Escape", async () => {
      await renderList();
      dragPricing();

      fireEvent.keyDown(window, { key: "Escape" });

      expect(copyOf()).toBeNull();
      expect(rowOf("NW-1042").closest("[data-drag-source]")).toBeNull();
    });

    it("brings the normal rows back after Escape and after the release", async () => {
      await renderList();
      dragPricing();
      expect(document.querySelector(".list-more")).not.toBeNull();

      fireEvent.keyDown(window, { key: "Escape" });

      expect(document.querySelector(".list-more")).toBeNull();
      expect(rowOf("NW-1042").closest(".list-scroll__space")).not.toBeNull();

      dragPricing();
      pointer.release();

      expect(document.querySelector(".list-more")).toBeNull();
      expect(rowOf("NW-1042").closest(".list-scroll__space")).not.toBeNull();
    });

    it("keeps the normal rows in an ungrouped view", async () => {
      await renderList(viewData([groupOf([PRICING, FOOTER])], [PRICING, FOOTER], { groupBy: "none" }));

      dragPricing();

      expect(copyOf()).not.toBeNull();
      expect(document.querySelector(".list-more")).toBeNull();
    });
  });

  describe("the folded view", () => {
    const moreOf = () => [...document.querySelectorAll<HTMLElement>(".list-more")];

    it("shows a closing piece for every group, and the rows of a closed group too", async () => {
      const closed = groupOf([MIGRATION], { key: "c", label: "Gamma", collapsedByDefault: true });
      await renderList(viewData([GROUPS[0], GROUPS[1], closed], [PRICING, FOOTER, MIGRATION]));
      expect(screen.queryByRole("row", { name: /NW-1004/ })).toBeNull();

      dragPricing();

      expect(moreOf()).toHaveLength(3);
      expect(rowOf("NW-1004")).toBeInTheDocument();
    });

    it("counts the rows beyond the two it keeps as more, and says nothing for a group that fits", async () => {
      const extra = [row({ id: "5", key: "NW-1005" }), row({ id: "6", key: "NW-1006" })];
      const crowded = groupOf([FOOTER, ...extra], { key: "b", label: "Beta" });
      const last = groupOf([PRICING], { key: "z", label: "Zulu" });
      await renderList(viewData([crowded, last], [FOOTER, ...extra, PRICING]));

      dragPricing();

      const [first, second] = moreOf();
      expect(first).toHaveTextContent("+1 more");
      expect(second).toBeEmptyDOMElement();
    });

    it("keeps the dragged row visible when it is beyond the rows its group keeps", async () => {
      const others = [row({ id: "5", key: "NW-1005" }), row({ id: "6", key: "NW-1006" })];
      const crowded = groupOf([...others, PRICING], { key: "a", label: "Alpha" });
      await renderList(viewData([crowded, GROUPS[1]], [...others, PRICING, FOOTER]));

      dragPricing();

      expect(rowOf("NW-1042").closest(".list-item")).toHaveAttribute("data-drag-source");
      expect(moreOf()[0]).toHaveTextContent("+1 more");
    });

    it("gives a group with rows from a tabbed header a tabbed piece", async () => {
      await renderList();

      dragPricing();

      moreOf().forEach((piece) => expect(piece).toHaveClass("list-more--tabbed"));
    });
  });

  describe("the zones", () => {
    it("waits on every group but the issue's own, without a drop key, until Rust answers", async () => {
      mocked.issueDropTargets.mockReturnValue(new Promise(() => {}));
      await renderList();

      dragPricing();

      const waiting = zones("wait");
      expect(waiting).toHaveLength(2);
      expect(zones()).toHaveLength(2);
      waiting.forEach((zone) => expect(zone).not.toHaveAttribute("data-drop-key"));
    });

    it("keys and labels the groups the issue may go to, and marks the others as refused", async () => {
      answer([target("b", "Set status to Beta")]);
      await renderList();

      dragPricing();

      await answered();
      const [ok] = zones("ok");
      expect(zones("ok")).toHaveLength(1);
      expect(ok).toHaveAttribute("data-drop-key", "b");
      expect(ok).toHaveTextContent("Set status to Beta");
      expect(zones("no")).toHaveLength(1);
      expect(zones("no")[0]).not.toHaveAttribute("data-drop-key");
      expect(zones("no")[0]).toBeEmptyDOMElement();
    });

    it("lays the zones inside the scroll space, tabbed under their group's header, and none over the issue's own group", async () => {
      answer([target("b", "Set status to Beta")]);
      await renderList();

      dragPricing();

      await answered();
      expect(zones()).toHaveLength(2);
      zones().forEach((zone) => {
        expect(zone.parentElement).toHaveClass("list-scroll__space");
        expect(zone).toHaveClass("drop-zone--tabbed");
      });
    });

    it("gives a closed group a zone as well", async () => {
      answer([target("c", "Set status to Gamma")]);
      const closed = groupOf([MIGRATION], { key: "c", label: "Gamma", collapsedByDefault: true });
      await renderList(viewData([GROUPS[0], GROUPS[1], closed], [PRICING, FOOTER, MIGRATION]));

      dragPricing();

      await answered();
      expect(zones("ok")).toHaveLength(1);
      expect(zones("ok")[0]).toHaveAttribute("data-drop-key", "c");
    });

    it("gives every other group a zone when Rust answers with no targets", async () => {
      answer([], "Resolve the blocking issue first.");
      await renderList();

      dragPricing();

      await answered();
      expect(zones("no")).toHaveLength(2);
      expect(zones("ok")).toHaveLength(0);
    });

    it("marks only the zone under the pointer that takes the drop as over, and unmarks it when the pointer leaves", async () => {
      answer([target("b", "Set status to Beta"), target("c", "Set status to Gamma")]);
      await renderList();
      dragPricing();
      await answered();

      pointer.over(zones("ok")[0]);
      pointer.moveTo(50, 50);

      expect(zones("ok")[0]).toHaveClass("drop-zone--over");
      expect(zones("ok")[1]).not.toHaveClass("drop-zone--over");

      pointer.over(null);
      pointer.moveTo(60, 60);

      expect(zones().filter((zone) => zone.classList.contains("drop-zone--over"))).toHaveLength(0);
    });

    it("never marks a zone that refuses the drop as over", async () => {
      answer([target("b", "Set status to Beta")]);
      await renderList();
      dragPricing();
      await answered();

      pointer.over(zones("no")[0]);
      pointer.moveTo(50, 50);

      expect(zones("no")[0]).not.toHaveClass("drop-zone--over");
    });

    it("removes the zones once the drag is over", async () => {
      answer([target("b", "Set status to Beta")]);
      await renderList();
      dragPricing();
      await answered();

      pointer.release();

      expect(zones()).toHaveLength(0);
    });
  });

  describe("the tray", () => {
    it("lists targets the view has no group for as chips with their action", async () => {
      answer([target("b", "Set status to Beta"), target("sam", "Assign to Sam Lee")]);
      await renderList();

      dragPricing();

      await answered();
      expect(tray()).toHaveTextContent("Also");
      expect(chips()).toHaveLength(1);
      expect(chips()[0]).toHaveAttribute("data-drop-key", "sam");
      expect(chips()[0]).toHaveTextContent("Assign to Sam Lee");
    });

    it("is absent when every target has a group in the view", async () => {
      answer([target("b", "Set status to Beta")]);
      await renderList();

      dragPricing();

      await answered();
      expect(tray()).toBeNull();
    });

    it("is absent without targets", async () => {
      answer([]);
      await renderList();

      dragPricing();

      await answered();
      expect(tray()).toBeNull();
    });

    it("marks the chip under the pointer as over", async () => {
      answer([target("sam", "Assign to Sam Lee")]);
      await renderList();
      dragPricing();
      await waitFor(() => expect(chips()).toHaveLength(1));
      expect(chips()[0]).not.toHaveClass("drop-chip--over");

      pointer.over(chips()[0]);
      pointer.moveTo(50, 50);

      expect(chips()[0]).toHaveClass("drop-chip--over");
    });

    it("takes a drop on a chip", async () => {
      answer([target("sam", "Assign to Sam Lee")]);
      await renderList();
      dragPricing();
      await waitFor(() => expect(chips()).toHaveLength(1));
      pointer.over(chips()[0]);

      pointer.release();

      expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "sam");
    });
  });

  describe("the copy's preview", () => {
    const withColumns = (patch: Partial<ViewData>, rows = [PRICING, FOOTER, MIGRATION], groups = GROUPS) => viewData(groups, rows, patch);

    it("takes the target status's style and, grouped by status, its name while over a zone that takes the drop", async () => {
      answer([target("b", "Set status to In Review", { kind: "status", style: "review" }, "In Review")]);
      await renderList(withColumns({ columns: ["status"] }));
      dragPricing();
      await answered();

      pointer.over(zones("ok")[0]);
      pointer.moveTo(50, 50);

      const copy = copyOf()!;
      expect(copy.querySelector(".status-square")).toHaveClass("status-square--review");
      expect(copy.querySelector(".status-square")).not.toHaveClass("status-square--progress");
      expect(copy.querySelector(".status-pill")).toHaveClass("status-pill--review");
      expect(copy.querySelector(".status-pill")).toHaveTextContent("In Review");
    });

    it("shows the issue as it is again once the pointer leaves the zone", async () => {
      answer([target("b", "Set status to In Review", { kind: "status", style: "review" }, "In Review")]);
      await renderList(withColumns({ columns: ["status"] }));
      dragPricing();
      await answered();
      pointer.over(zones("ok")[0]);
      pointer.moveTo(50, 50);

      pointer.over(null);
      pointer.moveTo(60, 60);

      const copy = copyOf()!;
      expect(copy.querySelector(".status-square")).toHaveClass("status-square--progress");
      expect(copy.querySelector(".status-pill")).toHaveClass("status-pill--progress");
      expect(copy.querySelector(".status-pill")).toHaveTextContent("In Progress");
    });

    it("keeps the copy as it is over a zone that refuses the drop", async () => {
      answer([target("b", "Set status to In Review", { kind: "status", style: "review" }, "In Review")]);
      await renderList(withColumns({ columns: ["status"] }));
      dragPricing();
      await answered();

      pointer.over(zones("no")[0]);
      pointer.moveTo(50, 50);

      expect(copyOf()!.querySelector(".status-square")).toHaveClass("status-square--progress");
      expect(copyOf()!.querySelector(".status-pill")).toHaveTextContent("In Progress");
    });

    it("swaps the priority flag for the target priority's", async () => {
      const high = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", priority: { name: "High", level: 2 } });
      const groups = [groupOf([high], { key: "p2", label: "High" }), groupOf([FOOTER], { key: "p1", label: "Highest" })];
      answer([target("p1", "Set priority to Highest", { kind: "priority", level: 1 }, "Highest")]);
      await renderList(withColumns({ groupBy: "priority", columns: ["priority"] }, [high, FOOTER], groups));
      dragPricing();
      await answered();

      pointer.over(zones("ok")[0]);
      pointer.moveTo(50, 50);

      expect(copyOf()!.querySelector(".priority-flag")).toHaveClass("priority-flag--highest");
      expect(copyOf()!.querySelector(".priority-flag")).not.toHaveClass("priority-flag--high");
    });
  });

  describe("dropping", () => {
    async function dragOverZone() {
      answer([target("b", "Set status to Beta")]);
      await renderList();
      dragPricing();
      await answered();
      pointer.over(zones("ok")[0]);
    }

    it("moves the issue onto the zone under the pointer and confirms with a Change back action", async () => {
      mocked.issueDrop.mockResolvedValue({
        kind: "done",
        outcome: { done: "Moved NW-1042 to In Review", reverse: { label: "Change back to In Progress", change: { kind: "assign", accountId: null } } },
      });
      await dragOverZone();

      pointer.release();

      expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "b");
      await waitFor(() => expect(toasts()).toEqual([["success", "Moved NW-1042 to In Review"]]));
      expect(useStore.getState().toasts[0].action?.label).toBe("Change back to In Progress");
    });

    it("opens a status menu with the moves when Jira needs a choice", async () => {
      mocked.issueDrop.mockResolvedValue({ kind: "choose", moves: [REVIEW, TESTING] });
      await dragOverZone();

      pointer.release();

      const menu = await screen.findByRole("menu", { name: "Status of NW-1042" });
      expect(within(menu).getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["In Review", "In Testing"]);
    });

    it("shows an error toast when the drop fails", async () => {
      mocked.issueDrop.mockRejectedValue({ kind: "invalid_input", message: "The issue needs a resolution", code: null });
      await dragOverZone();

      pointer.release();

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not move the issue: The issue needs a resolution"]]));
    });

    it("does nothing when released over a zone that refuses the drop", async () => {
      await dragOverZone();
      pointer.over(zones("no")[0]);

      pointer.release();

      expect(mocked.issueDrop).not.toHaveBeenCalled();
      expect(copyOf()).toBeNull();
    });

    it("does nothing when released over nothing", async () => {
      await dragOverZone();
      pointer.over(null);
      pointer.moveTo(50, 50);

      pointer.release(50, 50);

      expect(mocked.issueDrop).not.toHaveBeenCalled();
      expect(copyOf()).toBeNull();
    });

    it("cancels on Escape: no copy and no drop on release", async () => {
      await dragOverZone();

      fireEvent.keyDown(window, { key: "Escape" });
      pointer.release();

      expect(copyOf()).toBeNull();
      expect(mocked.issueDrop).not.toHaveBeenCalled();
    });

    it("does not select the row under the pointer with the click that ends the drag", async () => {
      await dragOverZone();
      pointer.release();

      fireEvent.click(rowOf("NW-1003"));

      expect(rowOf("NW-1003")).toHaveAttribute("aria-selected", "false");
    });
  });
});

describe("dragging in the Table", () => {
  const GROUPS = [
    groupOf([PRICING], { key: "a", label: "Alpha" }),
    groupOf([FOOTER], { key: "b", label: "Beta" }),
    groupOf([MIGRATION], { key: "c", label: "Gamma", collapsedByDefault: true }),
  ];

  async function renderTable(data: ViewData = viewData(GROUPS, [PRICING, FOOTER, MIGRATION]), withChanges = true) {
    allowChanges(withChanges);
    mocked.viewRows.mockResolvedValue(data);
    render(
      <QueryClientProvider client={createQueryClient()}>
        <TableView view={view({ layout: "table" })} />
      </QueryClientProvider>,
    );
    await screen.findByRole("grid");
    await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
    await act(async () => {});
  }

  const rowOf = (key: string) => screen.getByRole("row", { name: new RegExp(key) });
  const dragPricing = () => {
    pointer.press(rowOf("NW-1042"));
    pointer.moveTo(40, 40);
  };

  it("asks Rust where the issue may go, with the view's groups and the row's own group", async () => {
    await renderTable();

    dragPricing();

    expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "status", "a", [
      expect.objectContaining({ key: "a", label: "Alpha" }),
      expect.objectContaining({ key: "b", label: "Beta" }),
      expect.objectContaining({ key: "c", label: "Gamma" }),
    ]);
  });

  it("folds the groups in place, closed ones open too, with the dragged row marked and the column header still there", async () => {
    await renderTable();
    expect(screen.queryByRole("row", { name: /NW-1004/ })).toBeNull();
    expect(document.querySelector(".table-more")).toBeNull();

    dragPricing();

    expect(document.querySelectorAll(".table-more")).toHaveLength(3);
    expect(rowOf("NW-1042").closest("[data-drag-source]")).not.toBeNull();
    expect(rowOf("NW-1004")).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Summary" })).toBeInTheDocument();
    expect(copyOf()).toHaveTextContent("Redesign the pricing page");
  });

  it("counts the folded rows of a group as more", async () => {
    const extra = [row({ id: "5", key: "NW-1005" }), row({ id: "6", key: "NW-1006" })];
    await renderTable(viewData([groupOf([PRICING, FOOTER, ...extra], { key: "a", label: "Alpha" })], [PRICING, FOOTER, ...extra]));

    dragPricing();

    expect(document.querySelector(".table-more")).toHaveTextContent("+2 more");
  });

  it("puts the zones over the rows of the other groups once Rust answers", async () => {
    answer([target("b", "Set status to Beta")]);
    await renderTable();

    dragPricing();

    await answered();
    expect(zones()).toHaveLength(2);
    zones().forEach((zone) => {
      expect(zone).toHaveClass("drop-zone--rows");
      expect(zone.closest(".table-space")).not.toBeNull();
    });
    expect(zones("ok")[0]).toHaveAttribute("data-drop-key", "b");
    expect(zones("ok")[0]).toHaveTextContent("Set status to Beta");
    expect(zones("no")).toHaveLength(1);
  });

  it("shows the target status in the copy's status cell while over a zone that takes the drop, and the own one again after leaving it", async () => {
    answer([target("b", "Set status to In Review", { kind: "status", style: "review" }, "In Review")]);
    await renderTable();
    dragPricing();
    await answered();

    pointer.over(zones("ok")[0]);
    pointer.moveTo(50, 50);

    expect(copyOf()!.querySelector(".status-name")).toHaveTextContent("In Review");
    expect(copyOf()!.querySelector(".status-square")).toHaveClass("status-square--review");

    pointer.over(null);
    pointer.moveTo(60, 60);

    expect(copyOf()!.querySelector(".status-name")).toHaveTextContent("In Progress");
    expect(copyOf()!.querySelector(".status-square")).toHaveClass("status-square--progress");
  });

  it("moves the issue onto the zone it is released over", async () => {
    answer([target("b", "Set status to Beta")]);
    await renderTable();
    dragPricing();
    await answered();
    pointer.over(zones("ok")[0]);

    pointer.release();

    expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "b");
  });

  it("does nothing when released over a zone that refuses the drop", async () => {
    answer([target("b", "Set status to Beta")]);
    await renderTable();
    dragPricing();
    await answered();
    pointer.over(zones("no")[0]);

    pointer.release();

    expect(mocked.issueDrop).not.toHaveBeenCalled();
  });

  it("brings the normal rows back on Escape", async () => {
    await renderTable();
    dragPricing();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(document.querySelector(".table-more")).toBeNull();
    expect(rowOf("NW-1042").closest(".table-space")).not.toBeNull();
    expect(copyOf()).toBeNull();
  });

  it("keeps its layout in an ungrouped view", async () => {
    await renderTable(viewData([groupOf([PRICING, FOOTER])], [PRICING, FOOTER], { groupBy: "none" }));

    dragPricing();

    expect(copyOf()).not.toBeNull();
    expect(document.querySelector(".table-more")).toBeNull();
    expect(rowOf("NW-1042").closest(".table-space")).not.toBeNull();
  });

  it("does not start without changes to Jira", async () => {
    await renderTable(undefined, false);

    dragPricing();

    expect(copyOf()).toBeNull();
    expect(document.querySelector(".table-more")).toBeNull();
    expect(mocked.issueDropTargets).not.toHaveBeenCalled();
  });
});

describe("dragging on the Board", () => {
  const TODO = boardColumn({ key: "1", name: "To Do", style: "todo", mark: { kind: "status", style: "todo" } });
  const PROGRESS = boardColumn({ key: "3", name: "In Progress", style: "progress", ids: ["1"] });
  const DONE = boardColumn({ key: "5", name: "Done", style: "done", mark: { kind: "status", style: "done" }, ids: [] });
  const BLOCKED = boardColumn({ key: "7", name: "Blocked", style: "blocked", mark: { kind: "status", style: "blocked" }, collapsed: true });

  async function renderBoard(current: View = view({ layout: "board" }), columns = [TODO, PROGRESS, DONE]) {
    mocked.viewBoard.mockResolvedValue(boardData(columns, [PRICING]));
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
  const card = () => screen.getByRole("article", { name: "NW-1042 Redesign the pricing page" });
  const dragCard = () => {
    pointer.press(screen.getByText("Redesign the pricing page"));
    pointer.moveTo(40, 40);
  };

  it("asks Rust where the card may go, by the view's grouping", async () => {
    await renderBoard(view({ layout: "board", groupBy: "assignee" }));

    dragCard();

    expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "assignee", "3", [
      { key: "1", label: "To Do" },
      { key: "3", label: "In Progress" },
      { key: "5", label: "Done" },
    ]);
  });

  it("asks by status when the board groups by nothing", async () => {
    await renderBoard(view({ layout: "board", groupBy: "none" }));

    dragCard();

    expect(mocked.issueDropTargets).toHaveBeenCalledWith("1", "status", "3", expect.any(Array));
  });

  it("keeps the card in its column, marked as the source, with a copy on the body that goes away on release", async () => {
    await renderBoard();

    dragCard();

    expect(card()).toHaveAttribute("data-drag-source");
    expect(column("In Progress")).toContainElement(card());
    expect(copyOf()!.parentElement).toBe(document.body);
    expect(copyOf()).toHaveAttribute("aria-hidden", "true");

    pointer.release();

    expect(copyOf()).toBeNull();
    expect(card()).not.toHaveAttribute("data-drag-source");
  });

  it("puts a zone in each column but the card's own, and one in a collapsed column's strip", async () => {
    answer([target("5", "Set status to Done", { kind: "status", style: "done" })]);
    await renderBoard(view({ layout: "board" }), [TODO, PROGRESS, DONE, BLOCKED]);

    dragCard();

    await answered();
    expect(column("In Progress").querySelector(".drop-zone")).toBeNull();
    expect(column("To Do").querySelector(".drop-zone--no")).not.toBeNull();
    expect(column("Blocked").querySelector(".drop-zone--no")).not.toBeNull();
    const ok = column("Done").querySelector(".drop-zone--ok");
    expect(ok).toHaveAttribute("data-drop-key", "5");
    expect(ok).toHaveTextContent("Set status to Done");
  });

  it("marks the board as dragging while the drag lasts, and puts the zones inside the columns and strips", async () => {
    answer([target("5", "Set status to Done")]);
    await renderBoard(view({ layout: "board" }), [TODO, PROGRESS, DONE, BLOCKED]);
    const board = screen.getByRole("list", { name: "Board" });
    expect(board).not.toHaveClass("board--dragging");

    dragCard();

    await answered();
    expect(board).toHaveClass("board--dragging");
    zones().forEach((zone) => expect(zone.closest(".board-column, .board-strip__item")).not.toBeNull());

    pointer.release();

    expect(board).not.toHaveClass("board--dragging");
  });

  it("waits in every other column until Rust answers", async () => {
    mocked.issueDropTargets.mockReturnValue(new Promise(() => {}));
    await renderBoard();

    dragCard();

    expect(zones("wait")).toHaveLength(2);
    zones("wait").forEach((zone) => expect(zone).not.toHaveAttribute("data-drop-key"));
  });

  it("lists targets without a column in the tray", async () => {
    answer([target("5", "Set status to Done"), target("sam", "Assign to Sam Lee")]);
    await renderBoard();

    dragCard();

    await waitFor(() => expect(chips()).toHaveLength(1));
    expect(tray()).toHaveTextContent("Also");
    expect(chips()[0]).toHaveAttribute("data-drop-key", "sam");
    expect(chips()[0]).toHaveTextContent("Assign to Sam Lee");
  });

  it("marks the column zone under the pointer as over, and unmarks it when the pointer leaves", async () => {
    answer([target("5", "Set status to Done")]);
    await renderBoard();
    dragCard();
    await answered();

    pointer.over(zones("ok")[0]);
    pointer.moveTo(50, 50);

    expect(zones("ok")[0]).toHaveClass("drop-zone--over");
    expect(zones("no")[0]).not.toHaveClass("drop-zone--over");

    pointer.over(null);
    pointer.moveTo(60, 60);

    expect(zones("ok")[0]).not.toHaveClass("drop-zone--over");
  });

  it("never marks a column that refuses the drop as over", async () => {
    answer([target("5", "Set status to Done")]);
    await renderBoard();
    dragCard();
    await answered();

    pointer.over(zones("no")[0]);
    pointer.moveTo(50, 50);

    expect(zones("no")[0]).not.toHaveClass("drop-zone--over");
  });

  it("marks the chip under the pointer as over", async () => {
    answer([target("sam", "Assign to Sam Lee")]);
    await renderBoard();
    dragCard();
    await waitFor(() => expect(chips()).toHaveLength(1));

    pointer.over(chips()[0]);
    pointer.moveTo(50, 50);

    expect(chips()[0]).toHaveClass("drop-chip--over");
  });

  it("moves the card onto the column zone it is released over", async () => {
    answer([target("5", "Set status to Done")]);
    await renderBoard();
    dragCard();
    await answered();
    pointer.over(zones("ok")[0]);

    pointer.release();

    expect(mocked.issueDrop).toHaveBeenCalledWith("1", "status", "5");
  });

  it("does nothing when released over a column that refuses the drop", async () => {
    answer([target("5", "Set status to Done")]);
    await renderBoard();
    dragCard();
    await answered();
    pointer.over(column("To Do").querySelector(".drop-zone--no"));

    pointer.release();

    expect(mocked.issueDrop).not.toHaveBeenCalled();
  });

  it("does not start without changes to Jira", async () => {
    allowChanges(false);
    await renderBoard();

    dragCard();

    expect(copyOf()).toBeNull();
    expect(mocked.issueDropTargets).not.toHaveBeenCalled();
  });
});

describe("colorClass", () => {
  it.each<[string, GroupMark | null, string]>([
    ["a status", { kind: "status", style: "review" }, "drop-color--status-review"],
    ["a person", { kind: "person", person: { accountId: "u1", name: "Jamie Doe", initials: "JD", color: 3 } }, "drop-color--avatar-3"],
    ["a priority", { kind: "priority", level: 2 }, "drop-color--priority-2"],
    ["a priority level above the scale", { kind: "priority", level: 5 }, "drop-color--priority-4"],
    ["no mark", null, "drop-color--plain"],
  ])("names the color of %s", (_name, mark, expected) => {
    expect(colorClass(mark)).toBe(expected);
  });
});
