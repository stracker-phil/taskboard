import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type View, type ViewData } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { COLUMNS, groupOf, row, viewData } from "../../test/issues";
import { settings } from "../../test/settings";
import { sidebarTree, space } from "../../test/spaces";
import { view } from "../../test/views";
import { TableView } from "./TableView";

const mocked = vi.mocked(api, true);

type Restorable = { mockRestore: () => void };
let spies: Restorable[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getSettings.mockResolvedValue(settings());
  mocked.sidebarTree.mockResolvedValue(sidebarTree([]));
  mocked.columnsCatalogue.mockResolvedValue([...COLUMNS, { id: "due", name: "Due", custom: false }]);
  mocked.viewSetSort.mockResolvedValue(view());
  mocked.viewSetWidths.mockResolvedValue(view());
  spies = [
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(1000),
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(800),
  ];
});

afterEach(() => {
  spies.forEach((s) => s.mockRestore());
});

const TABLE = view({ layout: "table", sort: { by: "key", descending: false } });
const OPEN = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page" });
const DONE = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes", style: "done", statusName: "Done" });

async function renderTable(data: ViewData = viewData([groupOf([OPEN, DONE])], [OPEN, DONE], { columns: ["assignee", "priority"], groupBy: "none" }), current: View = TABLE) {
  mocked.viewRows.mockResolvedValue(data);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TableView view={current} />
    </QueryClientProvider>,
  );
  await screen.findByRole("grid");
}

/** The header cell with this column name. */
const header = (name: string) => screen.getByText(name).closest<HTMLElement>('[role="columnheader"]')!;

describe("TableView", () => {
  it("renders nothing until the rows have loaded", () => {
    mocked.viewRows.mockReturnValue(new Promise(() => {}));
    render(
      <QueryClientProvider client={createQueryClient()}>
        <TableView view={TABLE} />
      </QueryClientProvider>,
    );

    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  it("shows what the level is missing instead of a grid when the view has no issues", async () => {
    mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
    mocked.viewRows.mockResolvedValue(viewData([], [], { groupBy: "none" }));

    render(
      <QueryClientProvider client={createQueryClient()}>
        <TableView view={TABLE} />
      </QueryClientProvider>,
    );

    expect(await screen.findByRole("heading", { name: "Nothing in Website relaunch" })).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  describe("headers", () => {
    it("starts with Key, Summary and Status, then the view's columns by their names", async () => {
      await renderTable();

      await waitFor(() => expect(screen.getAllByRole("columnheader").map((h) => h.textContent?.replace(/[↑↓]/, ""))).toEqual(["Key", "Summary", "Status", "Assignee", "Priority"]));
    });

    it("shows one Status header when the view also has the status column", async () => {
      await renderTable(viewData([groupOf([OPEN, DONE])], [OPEN, DONE], { columns: ["status", "assignee"], groupBy: "none" }));

      await waitFor(() => expect(screen.getAllByRole("columnheader").map((h) => h.textContent?.replace(/[↑↓]/, ""))).toEqual(["Key", "Summary", "Status", "Assignee"]));
    });

    it("marks only the sorted column with aria-sort", async () => {
      await renderTable();

      expect(header("Key")).toHaveAttribute("aria-sort", "ascending");
      expect(header("Summary")).not.toHaveAttribute("aria-sort");
      expect(header("Status")).not.toHaveAttribute("aria-sort");
    });

    it("marks a descending sort as descending", async () => {
      await renderTable(undefined, view({ layout: "table", sort: { by: "key", descending: true } }));

      expect(header("Key")).toHaveAttribute("aria-sort", "descending");
    });

    it("marks no column when the view sorts by one the table has no header for", async () => {
      await renderTable(undefined, view({ layout: "table", sort: { by: "updated", descending: true } }));

      expect(screen.getAllByRole("columnheader").some((h) => h.hasAttribute("aria-sort"))).toBe(false);
    });

    it("flips the direction when the sorted header is clicked", async () => {
      await renderTable();

      await userEvent.click(screen.getByRole("button", { name: /^Key/ }));

      expect(mocked.viewSetSort).toHaveBeenCalledWith(101, { by: "key", descending: true });
    });

    it.each([
      ["Summary", { by: "summary", descending: false }],
      ["Status", { by: "status", descending: false }],
      ["Priority", { by: "priority", descending: true }],
      ["Due", { by: "due", descending: true }],
    ])("sorts by %s in its natural direction when another column is sorted", async (name, sort) => {
      await renderTable(viewData([groupOf([OPEN])], [OPEN], { columns: ["priority", "due"] }));
      await screen.findByRole("button", { name });

      await userEvent.click(screen.getByRole("button", { name }));

      expect(mocked.viewSetSort).toHaveBeenCalledWith(101, sort);
    });

    it("offers no sort button on a column that cannot sort", async () => {
      await renderTable();
      await screen.findByText("Assignee");

      expect(within(header("Assignee")).queryByRole("button")).not.toBeInTheDocument();
    });

    it("shows a toast when the sort cannot be saved", async () => {
      mocked.viewSetSort.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderTable();

      await userEvent.click(screen.getByRole("button", { name: /^Key/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not sort the view: Disk full"));
    });
  });

  describe("column widths", () => {
    it("applies a stored width to the header cell", async () => {
      await renderTable(undefined, view({ layout: "table", widths: { key: 150 } }));

      expect(header("Key")).toHaveStyle({ width: "150px" });
    });

    it("uses the default width for a column without a stored one", async () => {
      await renderTable();

      expect(header("Key")).toHaveStyle({ width: "90px" });
    });

    it("saves the widths with the dragged column's new width once the pointer is released", async () => {
      await renderTable(undefined, view({ layout: "table", widths: { status: 160 } }));
      const handle = screen.getByRole("separator", { name: "Resize Key" });

      fireEvent.pointerDown(handle, { button: 0, clientX: 100 });
      fireEvent.pointerMove(window, { clientX: 120 });
      fireEvent.pointerMove(window, { clientX: 140 });
      fireEvent.pointerUp(window);

      await waitFor(() => expect(mocked.viewSetWidths).toHaveBeenCalledWith(101, { status: 160, key: 130 }));
      expect(mocked.viewSetWidths).toHaveBeenCalledTimes(1);
    });

    it("widens the header while the pointer is still down", async () => {
      await renderTable();

      fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize Key" }), { button: 0, clientX: 100 });
      fireEvent.pointerMove(window, { clientX: 140 });

      expect(header("Key")).toHaveStyle({ width: "130px" });
      expect(mocked.viewSetWidths).not.toHaveBeenCalled();
      fireEvent.pointerUp(window);
    });

    it("never narrows a column below 48", async () => {
      await renderTable();

      fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize Key" }), { button: 0, clientX: 300 });
      fireEvent.pointerMove(window, { clientX: 0 });
      fireEvent.pointerUp(window);

      await waitFor(() => expect(mocked.viewSetWidths).toHaveBeenCalledWith(101, { key: 48 }));
    });

    it("saves nothing when the pointer is released where it went down", async () => {
      await renderTable();

      fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize Key" }), { button: 0, clientX: 100 });
      fireEvent.pointerUp(window);

      expect(mocked.viewSetWidths).not.toHaveBeenCalled();
    });

    it("gives Summary no resize handle", async () => {
      await renderTable();

      expect(screen.queryByRole("separator", { name: "Resize Summary" })).not.toBeInTheDocument();
    });
  });

  describe("rows", () => {
    it("shows key, summary and status of each issue", async () => {
      await renderTable();

      const issue = await screen.findByRole("row", { name: /NW-1001/ });
      expect(issue).toHaveTextContent("Publish the launch notes");
      expect(issue).toHaveTextContent("Done");
    });

    describe("labels", () => {
      const LABELED = row({ id: "5", key: "NW-1005", summary: "Update the footer", labels: [{ name: "launch", tint: 1 }] });
      const renderLabeled = async (inlineLabels: boolean, columns: string[]) => {
        await renderTable(viewData([groupOf([LABELED])], [LABELED], { columns, groupBy: "none" }), view({ layout: "table", inlineLabels }));
        return within(screen.getByRole("row", { name: /NW-1005/ })).getAllByRole("gridcell");
      };

      it("shows them after the title when the view shows labels inline", async () => {
        const [, title] = await renderLabeled(true, []);

        expect(within(title).getByText("launch")).toBeInTheDocument();
      });

      it("shows none when inline labels are off", async () => {
        await renderLabeled(false, []);

        expect(screen.queryByText("launch")).not.toBeInTheDocument();
      });

      it("shows them only in the Labels column when the view has it", async () => {
        const [, title, , labels] = await renderLabeled(true, ["labels"]);

        expect(within(title).queryByText("launch")).not.toBeInTheDocument();
        expect(within(labels).getByText("launch")).toBeInTheDocument();
      });
    });

    it("selects the row that is clicked", async () => {
      await renderTable();

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "false");
    });

    it("asks Rust to sync the history and web links of the clicked row", async () => {
      await renderTable();

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(mocked.issueSyncHistory).toHaveBeenCalledWith("2");
      expect(mocked.issueSyncWebLinks).toHaveBeenCalledWith("2");
    });

    it("shows no toast when syncing the history and web links fails", async () => {
      mocked.issueSyncHistory.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      mocked.issueSyncWebLinks.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      await renderTable();

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      await waitFor(() => expect(mocked.issueSyncWebLinks).toHaveBeenCalled());
      expect(useStore.getState().toasts).toEqual([]);
    });

    it("opens the preview of the clicked row when the view opens with the preview", async () => {
      await renderTable(undefined, { ...TABLE, openWith: "preview" });

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(useStore.getState().places["11:101"]).toMatchObject({ selectedId: "2", peekOpen: true });
      expect(useStore.getState().issueView).toBeNull();
    });

    it("opens the Issue view over the view when the view opens in full", async () => {
      await renderTable(undefined, { ...TABLE, openWith: "full" });

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
      expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
    });

    it("opens the Issue view of a double-clicked row even when the view opens with the preview", async () => {
      await renderTable(undefined, { ...TABLE, openWith: "preview" });

      await userEvent.dblClick(await screen.findByText("Publish the launch notes"));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
    });

    it("follows the app setting when the view has no choice", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "untilUnpinned", openIssuesIn: "full" } }));
      await renderTable();
      await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
      await act(async () => {});

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
    });

    it("renders only the visible part of 5,000 rows", async () => {
      const many = Array.from({ length: 5000 }, (_, i) => row({ id: String(i), key: `NW-${i + 1}`, summary: `Task ${i + 1}` }));
      await renderTable(viewData([groupOf(many)], many));

      await screen.findByText("Task 1");
      const rendered = within(screen.getByRole("grid")).getAllByRole("row").length;
      expect(rendered).toBeGreaterThan(1);
      expect(rendered).toBeLessThan(200);
    });
  });

  describe("groups", () => {
    const groups = () => [
      groupOf([OPEN], { key: "status:progress", label: "In Progress", mark: { kind: "status", style: "progress" } }),
      groupOf([DONE], { key: "status:done", label: "Done", count: 12, collapsedByDefault: true, mark: { kind: "status", style: "done" } }),
    ];

    it("shows a group header with its count and hides the rows of a collapsed-by-default group", async () => {
      await renderTable(viewData(groups(), [OPEN, DONE]));

      expect(await screen.findByText("12")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand Done" })).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByText("Publish the launch notes")).not.toBeInTheDocument();
    });

    it("shows no group header when the table is not grouped", async () => {
      await renderTable(viewData([groupOf([OPEN])], [OPEN], { groupBy: "none" }));

      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^(Collapse|Expand) / })).not.toBeInTheDocument();
    });

    it("shows the rows of a group once it is expanded", async () => {
      await renderTable(viewData(groups(), [OPEN, DONE]));

      await userEvent.click(await screen.findByRole("button", { name: "Expand Done" }));

      expect(await screen.findByText("Publish the launch notes")).toBeInTheDocument();
    });

    it("hides the rows of an open group when it is collapsed", async () => {
      await renderTable(viewData(groups(), [OPEN, DONE]));
      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Collapse In Progress" }));

      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
    });
  });

  describe("subtasks", () => {
    const PARENT = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", subtaskCount: 2 });
    const STEP = row({ id: "11", key: "NW-1043", summary: "Draft the price table", isSubtask: true, parentId: "1" });
    const LAST_STEP = row({ id: "12", key: "NW-1044", summary: "Review the price table", isSubtask: true, parentId: "1" });
    const modeData = (subtasks: ViewData["subtasks"], rows = [PARENT, STEP, LAST_STEP]) => viewData([groupOf(rows)], rows, { groupBy: "none", subtasks });
    const rowOf = (key: RegExp) => screen.getByRole("row", { name: key });

    it("shows a hidden-mode parent's subtask count as a chip after its summary", async () => {
      await renderTable(modeData("hidden", [PARENT]));

      expect(within(await screen.findByRole("row", { name: /NW-1042/ })).getByLabelText("2 subtasks")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /subtasks$/ })).not.toBeInTheDocument();
    });

    it("indents the subtasks of a parent in Hierarchy and gives the parent a caret", async () => {
      await renderTable(modeData("hierarchy"));

      expect(await screen.findByRole("row", { name: /NW-1043/ })).toHaveClass("table-row--child");
      expect(rowOf(/NW-1042/)).not.toHaveClass("table-row--child");
      expect(within(rowOf(/NW-1042/)).getByRole("button", { name: "Collapse subtasks" })).toHaveAttribute("aria-expanded", "true");
    });

    it("collapses and expands the subtasks with the caret, keeping the collapse in the view's place", async () => {
      await renderTable(modeData("hierarchy"));

      await userEvent.click(await screen.findByRole("button", { name: "Collapse subtasks" }));

      expect(screen.queryByText("Draft the price table")).not.toBeInTheDocument();
      expect(useStore.getState().places["11:101"].collapsedParents).toEqual(["1"]);
      expect(within(rowOf(/NW-1042/)).getByLabelText("2 subtasks")).toBeInTheDocument();
      expect(rowOf(/NW-1042/)).toHaveAttribute("aria-selected", "false");

      await userEvent.click(screen.getByRole("button", { name: "Expand subtasks" }));

      expect(await screen.findByText("Draft the price table")).toBeInTheDocument();
      expect(useStore.getState().places["11:101"].collapsedParents).toEqual([]);
    });

    it("shows the subtask icon before the summary of a subtask in List mode, with no caret", async () => {
      await renderTable(modeData("list"));

      const step = await screen.findByRole("row", { name: /NW-1043/ });
      const icon = step.querySelector(".subtask-icon");
      const title = within(step).getByText("Draft the price table");
      expect(icon).not.toBeNull();
      expect(icon!.compareDocumentPosition(title) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(step).not.toHaveClass("table-row--child");
      expect(screen.queryByRole("button", { name: /subtasks$/ })).not.toBeInTheDocument();
    });
  });
});
