import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Row, type View, type ViewData, type ViewPlace } from "../../api";
import { installCommands } from "../../commands";
import { getCommand } from "../../commands/registry";
import { MenuHost } from "../../components/menu";
import { createQueryClient } from "../../queries/queryClient";
import { NO_NARROWING } from "../../queries/views";
import { resetStore, useStore } from "../../state/store";
import { ALEX, COLUMNS, groupOf, row, viewData } from "../../test/issues";
import { behaviour, settings } from "../../test/settings";
import { tab, tabbedNav } from "../../test/nav";
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
  mocked.placeGet.mockResolvedValue(null);
  mocked.copyText.mockResolvedValue(undefined);
  spies = [
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(1000),
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(800),
  ];
});

afterEach(() => {
  spies.forEach((s) => s.mockRestore());
});

let client = createQueryClient();
const TABLE = view({ layout: "table", sort: { by: "key", descending: false } });
const OPEN = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page" });
const DONE = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes", style: "done", statusName: "Done" });

async function renderTable(data: ViewData = viewData([groupOf([OPEN, DONE])], [OPEN, DONE], { columns: ["assignee", "priority"], groupBy: "none" }), current: View = TABLE) {
  mocked.viewRows.mockResolvedValue(data);
  client = createQueryClient();
  const rendered = render(
    <QueryClientProvider client={client}>
      <TableView view={current} />
      <MenuHost />
    </QueryClientProvider>,
  );
  await screen.findByRole("grid");
  return rendered;
}

const place = () => useStore.getState().places["11:101"];

/** A preview opens once the double-click interval (500ms in the API mock) has passed. */
const PREVIEW_WAIT = { timeout: 2000 };
const pastDoubleClickInterval = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 700)));
const press = (id: string) => act(async () => void (await getCommand(id)?.run()));

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
    const headerNames = () => screen.getAllByRole("columnheader").map((h) => h.textContent?.replace(/[↑↓]/, ""));

    it("starts with Key and Summary, then the view's columns by their names", async () => {
      await renderTable();

      await waitFor(() => expect(headerNames()).toEqual(["Key", "Summary", "Assignee", "Priority"]));
    });

    it("shows a Status header at the status column's position when the view has it", async () => {
      await renderTable(viewData([groupOf([OPEN, DONE])], [OPEN, DONE], { columns: ["assignee", "status", "priority"], groupBy: "none" }));

      await waitFor(() => expect(headerNames()).toEqual(["Key", "Summary", "Assignee", "Status", "Priority"]));
    });

    it("shows no Status header or status name when the view lacks the status column", async () => {
      await renderTable();
      await screen.findByText("Assignee");

      expect(screen.queryByRole("columnheader", { name: /Status/ })).not.toBeInTheDocument();
      expect(screen.queryByText("In Progress")).not.toBeInTheDocument();
    });

    it("shows each row's status name in the Status column", async () => {
      await renderTable(viewData([groupOf([OPEN, DONE])], [OPEN, DONE], { columns: ["status"], groupBy: "none" }));

      expect(await within(screen.getByRole("row", { name: /NW-1042/ })).findByText("In Progress")).toBeInTheDocument();
      expect(within(screen.getByRole("row", { name: /NW-1001/ })).getByText("Done")).toBeInTheDocument();
    });

    it("keeps the status square out of the Key cell and puts it right before the title in the Summary cell", async () => {
      await renderTable();

      const issue = await screen.findByRole("row", { name: /NW-1001/ });
      const [keyCell, summaryCell] = within(issue).getAllByRole("gridcell");
      const square = summaryCell.querySelector(".status-square--done");
      const title = within(summaryCell).getByText("Publish the launch notes");
      expect(keyCell.querySelector(".status-square")).toBeNull();
      expect(square).not.toBeNull();
      expect(square!.compareDocumentPosition(title) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it("places the status square after the caret and subtask glyph of a Hierarchy row", async () => {
      const parent = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", childCount: 1 });
      const step = row({ id: "11", key: "NW-1043", summary: "Draft the price table", isSubtask: true, parentId: "1", depth: 1 });
      await renderTable(viewData([groupOf([parent, step])], [parent, step], { groupBy: "none", subtasks: "hierarchy" }));

      const parentCell = within(await screen.findByRole("row", { name: /NW-1042/ })).getAllByRole("gridcell")[1];
      const stepCell = within(screen.getByRole("row", { name: /NW-1043/ })).getAllByRole("gridcell")[1];
      const caret = within(parentCell).getByRole("button", { name: "Collapse subtasks" });
      const glyph = stepCell.querySelector(".subtask-icon")!;
      const after = (first: Element, second: Element | null) => first.compareDocumentPosition(second!) & Node.DOCUMENT_POSITION_FOLLOWING;
      expect(after(caret, parentCell.querySelector(".status-square"))).toBeTruthy();
      expect(after(glyph, stepCell.querySelector(".status-square"))).toBeTruthy();
    });

    it("marks only the sorted column with aria-sort", async () => {
      await renderTable();

      expect(header("Key")).toHaveAttribute("aria-sort", "ascending");
      expect(header("Summary")).not.toHaveAttribute("aria-sort");
      expect(header("Assignee")).not.toHaveAttribute("aria-sort");
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
      await renderTable(viewData([groupOf([OPEN])], [OPEN], { columns: ["status", "priority", "due"] }));
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

      expect(header("Key")).toHaveStyle({ width: "100px" });
    });

    it("saves the widths with the dragged column's new width once the pointer is released", async () => {
      await renderTable(undefined, view({ layout: "table", widths: { status: 160 } }));
      const handle = screen.getByRole("separator", { name: "Resize Key" });

      fireEvent.pointerDown(handle, { button: 0, clientX: 100 });
      fireEvent.pointerMove(window, { clientX: 120 });
      fireEvent.pointerMove(window, { clientX: 140 });
      fireEvent.pointerUp(window);

      await waitFor(() => expect(mocked.viewSetWidths).toHaveBeenCalledWith(101, { status: 160, key: 140 }));
      expect(mocked.viewSetWidths).toHaveBeenCalledTimes(1);
    });

    it("widens the header while the pointer is still down", async () => {
      await renderTable();

      fireEvent.pointerDown(screen.getByRole("separator", { name: "Resize Key" }), { button: 0, clientX: 100 });
      fireEvent.pointerMove(window, { clientX: 140 });

      await waitFor(() => expect(header("Key")).toHaveStyle({ width: "140px" }));
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
      await renderTable(viewData([groupOf([OPEN, DONE])], [OPEN, DONE], { columns: ["status"], groupBy: "none" }));

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
        const [, title, labels] = await renderLabeled(true, ["labels"]);

        expect(within(title).queryByText("launch")).not.toBeInTheDocument();
        expect(within(labels).getByText("launch")).toBeInTheDocument();
      });
    });

    describe("cells without a value", () => {
      const withValues = row({
        id: "6",
        key: "NW-1006",
        summary: "Update the footer",
        assignees: [ALEX],
        epic: { id: "9", name: "Launch Q4", color: 2 },
        priority: { name: "High", level: 2 },
      });
      const cellsOf = async (issue: Row) => {
        await renderTable(viewData([groupOf([issue])], [issue], { columns: ["assignee", "epic", "priority"], groupBy: "none" }));
        return within(await screen.findByRole("row", { name: new RegExp(issue.key) })).getAllByRole("gridcell").slice(2);
      };

      it("shows a dash in the assignee, epic and priority cells of an issue without them", async () => {
        const cells = await cellsOf(row({ id: "7", key: "NW-1007", summary: "Archive the old blog" }));

        expect(cells.map((c) => c.textContent)).toEqual(["—", "—", "—"]);
      });

      it("shows the values and no dash when the issue has them", async () => {
        const cells = await cellsOf(withValues);

        expect(within(cells[0]).getByRole("img", { name: "Alex Morgan" })).toBeInTheDocument();
        expect(cells[1]).toHaveTextContent("Launch Q4");
        expect(cells[2]).toHaveTextContent("High");
        expect(screen.queryByText("—")).not.toBeInTheDocument();
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

      await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }), PREVIEW_WAIT);
      expect(useStore.getState().issueView).toBeNull();
    });

    it("selects the row at once and opens the preview only after the double-click interval", async () => {
      await renderTable(undefined, { ...TABLE, openWith: "preview" });

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });
      await waitFor(() => expect(place().peekOpen).toBe(true), PREVIEW_WAIT);
    });

    it("moves an open preview to the clicked row at once", async () => {
      await renderTable(undefined, { ...TABLE, openWith: "preview" });
      await userEvent.click(await screen.findByText("Redesign the pricing page"));
      await waitFor(() => expect(place()).toMatchObject({ selectedId: "1", peekOpen: true }), PREVIEW_WAIT);

      await userEvent.click(screen.getByText("Publish the launch notes"));

      expect(place()).toMatchObject({ selectedId: "2", peekOpen: true });
    });

    it("opens the Issue view alone, never the preview, for a double-clicked row even after the interval has passed", async () => {
      await renderTable(undefined, { ...TABLE, openWith: "preview" });

      await userEvent.dblClick(await screen.findByText("Publish the launch notes"));
      await pastDoubleClickInterval();

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });
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
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ openIssuesIn: "full" }) }));
      await renderTable();
      await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
      await act(async () => {});

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
    });

    it("closes the preview and keeps the selection when a narrowing drops the selected issue", async () => {
      const previewing: ViewPlace = { scrollTop: 0, toggled: {}, collapsedParents: [], selectedId: "2", peekOpen: true };
      mocked.placeGet.mockResolvedValue(previewing);
      const { rerender } = await renderTable();
      await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }));

      mocked.viewRows.mockResolvedValue(viewData([groupOf([OPEN])], [OPEN], { columns: ["assignee", "priority"], groupBy: "none" }));
      rerender(
        <QueryClientProvider client={client}>
          <TableView view={TABLE} narrowing={{ ...NO_NARROWING, text: "pricing" }} />
        </QueryClientProvider>,
      );

      await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: false }));
    });

    describe("the key cell", () => {
      const keyOf = async (key: string) => within(await screen.findByRole("row", { name: new RegExp(key) })).getByText(key);

      it("copies the key and says so in a toast", async () => {
        await renderTable();

        await userEvent.click(await keyOf("NW-1001"));

        expect(mocked.copyText).toHaveBeenCalledWith("NW-1001");
        await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Copied NW-1001"));
      });

      it("neither selects the row nor opens the preview or the Issue view when clicked", async () => {
        await renderTable(undefined, { ...TABLE, openWith: "full" });

        await userEvent.click(await keyOf("NW-1001"));

        expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "false");
        expect(useStore.getState().places["11:101"]?.peekOpen ?? false).toBe(false);
        expect(useStore.getState().issueView).toBeNull();
      });

      it("opens no Issue view when double-clicked", async () => {
        await renderTable(undefined, { ...TABLE, openWith: "preview" });

        await userEvent.dblClick(await keyOf("NW-1001"));

        expect(useStore.getState().issueView).toBeNull();
      });
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

  describe("editable cells", () => {
    const EDITABLE = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", assignees: [ALEX], priority: { name: "High", level: 2 } });
    const REVIEW = { id: "31", statusId: "3", statusName: "In Review", style: "review", opensJira: false } as const;
    const cell = (text: string) => within(screen.getByRole("row", { name: /NW-1042/ })).getByText(text).closest<HTMLElement>('[role="gridcell"]')!;

    const OTHER = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes", priority: { name: "Low", level: 4 } });

    async function renderEditable(patch: Partial<ReturnType<typeof behaviour>> = {}, current: View = TABLE) {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, ...patch }) }));
      mocked.issueTransitions.mockResolvedValue([REVIEW]);
      await renderTable(viewData([groupOf([EDITABLE, OTHER])], [EDITABLE, OTHER], { columns: ["status", "assignee", "priority"], groupBy: "none" }), current);
      await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
      await act(async () => {});
    }

    it("opens the status menu from the Status cell", async () => {
      await renderEditable();

      await userEvent.click(cell("In Progress"));

      const menu = await screen.findByRole("menu", { name: "Status of NW-1042" });
      expect(within(menu).getByRole("menuitem", { name: /In Review/ })).toBeInTheDocument();
      expect(cell("In Progress")).toHaveAttribute("aria-haspopup", "menu");
    });

    it("opens the status menu from the status square, without opening the preview", async () => {
      await renderEditable({}, { ...TABLE, openWith: "preview" });

      await userEvent.click(screen.getByRole("button", { name: "Change the status of NW-1042 (In Progress)" }));
      await pastDoubleClickInterval();

      const menu = await screen.findByRole("menu", { name: "Status of NW-1042" });
      expect(within(menu).getByRole("menuitem", { name: /In Review/ })).toBeInTheDocument();
      expect(place()).toMatchObject({ selectedId: "1", peekOpen: false });
      expect(useStore.getState().issueView).toBeNull();
    });

    it("shows no status square button while changes to Jira are off", async () => {
      await renderEditable({ allowJiraChanges: false });

      expect(screen.queryByRole("button", { name: /Change the status of/ })).not.toBeInTheDocument();
    });

    it("opens the priority menu from the Priority cell, and choosing an entry writes it", async () => {
      await renderEditable();

      await userEvent.click(cell("High"));
      const menu = await screen.findByRole("menu", { name: "Priority of NW-1042" });
      await userEvent.click(within(menu).getByRole("menuitemradio", { name: /Highest/ }));

      expect(mocked.issueWrite).toHaveBeenCalledWith("1", { kind: "priority", name: "Highest" });
    });

    it("opens the assignee menu from the Assignee cell", async () => {
      await renderEditable();

      const assignee = within(screen.getByRole("row", { name: /NW-1042/ })).getByRole("img", { name: "Alex Morgan" });
      await userEvent.click(assignee);

      expect(await screen.findByRole("menu", { name: "Assignee of NW-1042" })).toBeInTheDocument();
    });

    it("selects the row when an editable cell is clicked", async () => {
      await renderEditable();

      await userEvent.click(cell("High"));

      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "true");
    });

    it("selects the row without opening the preview, even after the double-click interval", async () => {
      await renderEditable({}, { ...TABLE, openWith: "preview" });

      await userEvent.click(cell("High"));
      await pastDoubleClickInterval();

      expect(place()).toMatchObject({ selectedId: "1", peekOpen: false });
      expect(screen.getByRole("menu", { name: "Priority of NW-1042" })).toBeInTheDocument();
    });

    it("opens no Issue view when an editable cell is double-clicked", async () => {
      await renderEditable({}, { ...TABLE, openWith: "preview" });

      await userEvent.dblClick(cell("High"));
      await pastDoubleClickInterval();

      expect(useStore.getState().issueView).toBeNull();
    });

    it("keeps an open preview open on the row whose cell is clicked", async () => {
      await renderEditable({}, { ...TABLE, openWith: "preview" });
      await userEvent.click(screen.getByText("Publish the launch notes"));
      await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }), PREVIEW_WAIT);

      await userEvent.click(cell("High"));

      expect(place()).toMatchObject({ selectedId: "1", peekOpen: true });
    });

    it("opens nothing from the Priority cell while priority changes are switched off", async () => {
      await renderEditable({ disabledChanges: ["changePriority"] });

      await userEvent.click(cell("High"));

      expect(cell("High")).not.toHaveAttribute("aria-haspopup");
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(mocked.issuePriorities).not.toHaveBeenCalled();
      expect(cell("In Progress")).toHaveAttribute("aria-haspopup", "menu");
    });

    it("opens no menu from the Status cell while changes to Jira are off", async () => {
      await renderEditable({ allowJiraChanges: false });

      await userEvent.click(cell("In Progress"));

      expect(cell("In Progress")).not.toHaveAttribute("aria-haspopup");
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
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

  describe("issue keys", () => {
    const THIRD = row({ id: "3", key: "NW-1003", summary: "Fix the footer links" });
    const rows = [OPEN, DONE, THIRD];
    const flat = () => viewData([groupOf(rows)], rows, { groupBy: "none" });
    const grouped = () => viewData([groupOf([OPEN], { key: "a", label: "Alpha" }), groupOf([DONE], { key: "b", label: "Beta" })], [OPEN, DONE]);
    const selectedKeys = () =>
      screen
        .getAllByRole("row")
        .filter((r) => r.getAttribute("aria-selected") === "true")
        .map((r) => /NW-\d+/.exec(r.textContent ?? "")?.[0]);

    beforeEach(() => {
      installCommands("main");
    });

    it.each(["issue.next", "issue.next.arrow"])("%s selects the first row, then the following one", async (id) => {
      await renderTable(flat());
      await screen.findByRole("row", { name: /NW-1042/ });

      await press(id);
      expect(selectedKeys()).toEqual(["NW-1042"]);

      await press(id);
      expect(selectedKeys()).toEqual(["NW-1001"]);
    });

    it.each(["issue.previous", "issue.previous.arrow"])("%s selects the row before the selected one", async (id) => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Fix the footer links"));

      await press(id);

      expect(selectedKeys()).toEqual(["NW-1001"]);
    });

    it("stays on the last row at the bottom", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Fix the footer links"));

      await press("issue.next");

      expect(selectedKeys()).toEqual(["NW-1003"]);
    });

    it("opens the selected issue in the Issue view with the view as origin", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Publish the launch notes"));

      await press("issue.open");

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
    });

    it("closes the selected issue's peek and opens it again", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Publish the launch notes"));
      await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }), PREVIEW_WAIT);

      await press("issue.togglePeek");
      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });

      await press("issue.togglePeek");
      expect(place()).toMatchObject({ selectedId: "2", peekOpen: true });
    });

    it("closes the peek on escape and keeps the row selected", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Publish the launch notes"));
      await waitFor(() => expect(place().peekOpen).toBe(true), PREVIEW_WAIT);

      await press("peek.close");

      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });
      expect(selectedKeys()).toEqual(["NW-1001"]);
    });

    it("collapses the selected row's group on left and expands it on right", async () => {
      await renderTable(grouped());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      await press("group.collapse");
      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
      expect(screen.getByText("Publish the launch notes")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand Alpha" })).toBeInTheDocument();

      await press("group.expand");
      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Collapse Alpha" })).toBeInTheDocument();
    });

    it("copies the selected issue's key", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Publish the launch notes"));

      await press("issue.copyKey");

      expect(mocked.copyText).toHaveBeenCalledWith("NW-1001");
    });
  });

  describe("picking", () => {
    const THIRD = row({ id: "3", key: "NW-1003", summary: "Fix the footer links" });
    const rows = [OPEN, DONE, THIRD];
    const flat = () => viewData([groupOf(rows)], rows, { groupBy: "none" });
    const pickedKeys = () =>
      screen
        .getAllByRole("checkbox")
        .filter((b) => (b as HTMLInputElement).checked)
        .map((b) => b.getAttribute("aria-label")?.replace("Pick ", ""));

    beforeEach(() => {
      installCommands("main");
    });

    it("picks the ⌘-clicked row with the selected one, and opens no preview", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      fireEvent.click(screen.getByText("Publish the launch notes"), { metaKey: true });
      await pastDoubleClickInterval();

      expect(pickedKeys()).toEqual(["NW-1042", "NW-1001"]);
      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });
    });

    it("unpicks a picked row on a second ⌘-click", async () => {
      await renderTable(flat());
      fireEvent.click(await screen.findByText("Publish the launch notes"), { metaKey: true });
      fireEvent.click(screen.getByText("Fix the footer links"), { metaKey: true });

      fireEvent.click(screen.getByText("Fix the footer links"), { metaKey: true });

      expect(pickedKeys()).toEqual(["NW-1001"]);
    });

    it("highlights the selected row", async () => {
      await renderTable(flat());

      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      const selected = screen.getByRole("row", { name: /NW-1042/ });
      expect(selected).toHaveClass("table-row--selected");
      expect(selected).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "false");
    });

    it("highlights the picked rows as picked and keeps the selected highlight on the current row", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      fireEvent.click(screen.getByText("Publish the launch notes"), { metaKey: true });

      const current = screen.getByRole("row", { name: /NW-1001/ });
      const other = screen.getByRole("row", { name: /NW-1042/ });
      expect(current).toHaveClass("table-row--picked", "table-row--selected");
      expect(current).toHaveAttribute("aria-selected", "true");
      expect(other).toHaveClass("table-row--picked");
      expect(other).not.toHaveClass("table-row--selected");
      expect(other).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("row", { name: /NW-1003/ })).toHaveAttribute("aria-selected", "false");
    });

    it("titles a pick box with its action", async () => {
      await renderTable(flat());
      const box = await screen.findByRole("checkbox", { name: "Pick NW-1042" });
      expect(box).toHaveAttribute("title", "Pick (X)");

      await userEvent.click(box);

      expect(screen.getByRole("checkbox", { name: "Pick NW-1042" })).toHaveAttribute("title", "Drop from the picks (X)");
    });

    it("keeps the picks when J or K moves the selection", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));
      await press("issue.pickNext");

      await press("issue.next");
      expect(pickedKeys()).toEqual(["NW-1042", "NW-1001"]);
      expect(place().selectedId).toBe("3");

      await press("issue.previous");
      expect(pickedKeys()).toEqual(["NW-1042", "NW-1001"]);
      expect(place().selectedId).toBe("2");
    });

    it("picks the selected row on X and drops it on a second X", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      await press("issue.togglePick");
      expect(pickedKeys()).toEqual(["NW-1042"]);
      expect(within(await screen.findByRole("toolbar", { name: "Picked issues" })).getByText("1 picked")).toBeInTheDocument();

      await press("issue.togglePick");
      expect(pickedKeys()).toEqual([]);
    });

    it("picks two rows with X, J and X", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      await press("issue.togglePick");
      await press("issue.next");
      await press("issue.togglePick");

      expect(pickedKeys()).toEqual(["NW-1042", "NW-1001"]);
      expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveClass("table-row--selected");
    });

    it("disables X while no row is selected", async () => {
      await renderTable(flat());
      await screen.findByText("Publish the launch notes");

      expect(getCommand("issue.togglePick")?.when?.()).toBe(false);
    });

    it("shows no extra checkbox for the current row while rows are picked", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      fireEvent.click(screen.getByText("Publish the launch notes"), { metaKey: true });

      expect(screen.getByRole("checkbox", { name: "Pick NW-1001" })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Pick NW-1042" })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Pick NW-1003" })).not.toBeChecked();
      screen.getAllByRole("checkbox").forEach((b) => expect(b).not.toHaveClass("pick-box--shown"));
    });

    it("shows no checkbox without the pointer while nothing is picked", async () => {
      await renderTable(flat());

      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      screen.getAllByRole("checkbox").forEach((box) => expect(box).not.toHaveClass("pick-box--shown"));
    });

    it("picks the rows from the selected one to the ⇧-clicked one", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      fireEvent.click(screen.getByText("Fix the footer links"), { shiftKey: true });

      expect(pickedKeys()).toEqual(["NW-1042", "NW-1001", "NW-1003"]);
    });

    it("drops the picks on a plain click", async () => {
      await renderTable(flat());
      fireEvent.click(await screen.findByText("Publish the launch notes"), { metaKey: true });

      await userEvent.click(screen.getByText("Fix the footer links"));

      expect(pickedKeys()).toEqual([]);
      expect(place().selectedId).toBe("3");
    });

    it("picks only its own row from the checkbox, leaving the selection and the preview alone", async () => {
      await renderTable(flat());
      await userEvent.click(await screen.findByText("Redesign the pricing page"));
      await waitFor(() => expect(place().peekOpen).toBe(true), PREVIEW_WAIT);
      await press("peek.close");

      await userEvent.click(screen.getByRole("checkbox", { name: "Pick NW-1003" }));
      await pastDoubleClickInterval();

      expect(pickedKeys()).toEqual(["NW-1003"]);
      expect(place()).toMatchObject({ selectedId: "1", peekOpen: false });
      expect(useStore.getState().issueView).toBeNull();
    });

    it("opens no Issue view when a checkbox is double-clicked", async () => {
      await renderTable(flat());

      await userEvent.dblClick(await screen.findByRole("checkbox", { name: "Pick NW-1003" }));

      expect(useStore.getState().issueView).toBeNull();
    });

    describe("a ⌘-double-click", () => {
      const twoTabs = () => mocked.navTabNew.mockResolvedValue(tabbedNav([tab(1, { listId: 11, viewId: 101 }), tab(2, { listId: 11, viewId: 101 })], 1));
      const metaDoubleClick = (target: HTMLElement, shiftKey = false) => {
        fireEvent.click(target, { metaKey: true, shiftKey, detail: 1 });
        fireEvent.click(target, { metaKey: true, shiftKey, detail: 2 });
        fireEvent.dblClick(target, { metaKey: true, shiftKey });
      };

      it("opens the issue in a new tab behind the shown one, leaving the picks and selection as they were", async () => {
        twoTabs();
        await renderTable(flat());
        await userEvent.click(await screen.findByText("Redesign the pricing page"));

        metaDoubleClick(screen.getByText("Publish the launch notes"));

        await waitFor(() => expect(mocked.navTabNew).toHaveBeenCalledWith(11, 101, false));
        expect(useStore.getState().parkedIssueViews[2]).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
        expect(mocked.navTabActivate).not.toHaveBeenCalled();
        expect(useStore.getState().issueView).toBeNull();
        expect(pickedKeys()).toEqual([]);
        expect(place().selectedId).toBe("1");
        expect(screen.queryByRole("toolbar", { name: "Picked issues" })).not.toBeInTheDocument();
      });

      it("also shows the new tab with ⇧", async () => {
        twoTabs();
        await renderTable(flat());

        metaDoubleClick(await screen.findByText("Publish the launch notes"), true);

        await waitFor(() => expect(mocked.navTabActivate).toHaveBeenCalledWith(2));
      });
    });

    it("opens the Issue view in place on a plain double-click", async () => {
      await renderTable(flat());

      await userEvent.dblClick(await screen.findByText("Publish the launch notes"));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
      expect(mocked.navTabNew).not.toHaveBeenCalled();
    });

    it("opens nothing on a ⇧-double-click", async () => {
      await renderTable(flat());

      fireEvent.dblClick(await screen.findByText("Publish the launch notes"), { shiftKey: true });

      expect(useStore.getState().issueView).toBeNull();
      expect(mocked.navTabNew).not.toHaveBeenCalled();
    });

    describe("Open in a new tab (⌘⇧↵)", () => {
      it("opens the selected issue in a new tab and shows it", async () => {
        mocked.navTabNew.mockResolvedValue(tabbedNav([tab(1, { listId: 11, viewId: 101 }), tab(2, { listId: 11, viewId: 101 })], 1));
        await renderTable(flat());
        await userEvent.click(await screen.findByText("Publish the launch notes"));

        await press("issue.openInNewTab");

        expect(mocked.navTabNew).toHaveBeenCalledWith(11, 101, false);
        await waitFor(() => expect(mocked.navTabActivate).toHaveBeenCalledWith(2));
      });

      it("is disabled while no row is selected", async () => {
        await renderTable(flat());
        await screen.findByText("Publish the launch notes");

        expect(getCommand("issue.openInNewTab")?.when?.()).toBe(false);
      });
    });

    it("drops the picks on Esc", async () => {
      await renderTable(flat());
      fireEvent.click(await screen.findByText("Publish the launch notes"), { metaKey: true });
      expect(pickedKeys()).toEqual(["NW-1001"]);

      await press("peek.close");

      expect(pickedKeys()).toEqual([]);
    });
  });

  describe("saved place", () => {
    const groups = () => [
      groupOf([OPEN], { key: "status:progress", label: "In Progress" }),
      groupOf([DONE], { key: "status:done", label: "Done", collapsedByDefault: true }),
    ];
    const grouped = () => viewData(groups(), [OPEN, DONE]);
    const saved: ViewPlace = { scrollTop: 0, toggled: { "status:done": true, "status:progress": false }, collapsedParents: [], selectedId: "2", peekOpen: false };

    it("applies the place saved by Rust on the first render", async () => {
      mocked.placeGet.mockResolvedValue(saved);
      await renderTable(grouped());

      expect(await screen.findByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Collapse Done" })).toHaveAttribute("aria-expanded", "true");
      expect(mocked.placeGet).toHaveBeenCalledWith(11, 101);
    });

    it("comes back with the same row selected and the same groups open or closed", async () => {
      const first = await renderTable(grouped());
      await userEvent.click(await screen.findByRole("button", { name: "Expand Done" }));
      await userEvent.click(await screen.findByText("Publish the launch notes"));
      await userEvent.click(screen.getByRole("button", { name: "Collapse In Progress" }));
      first.unmount();

      await renderTable(grouped());

      expect(await screen.findByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("button", { name: "Collapse Done" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand In Progress" })).toBeInTheDocument();
      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
    });

    it("saves the toggled groups and the selected row to Rust", async () => {
      await renderTable(grouped());

      await userEvent.click(await screen.findByRole("button", { name: "Expand Done" }));
      await userEvent.click(await screen.findByText("Publish the launch notes"));

      await waitFor(() => expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, expect.objectContaining({ toggled: { "status:done": true }, selectedId: "2" })));
    });
  });

  describe("subtasks", () => {
    const PARENT = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", childCount: 2 });
    const STEP = row({ id: "11", key: "NW-1043", summary: "Draft the price table", isSubtask: true, parentId: "1", depth: 1 });
    const LAST_STEP = row({ id: "12", key: "NW-1044", summary: "Review the price table", isSubtask: true, parentId: "1", depth: 1 });
    const modeData = (subtasks: ViewData["subtasks"], rows = [PARENT, STEP, LAST_STEP]) => viewData([groupOf(rows)], rows, { groupBy: "none", subtasks });
    const rowOf = (key: RegExp) => screen.getByRole("row", { name: key });

    it("shows a hidden-mode parent's child count as a chip after its summary", async () => {
      await renderTable(modeData("hidden", [PARENT]));

      expect(within(await screen.findByRole("row", { name: /NW-1042/ })).getByLabelText("2 child issues")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /subtasks$/ })).not.toBeInTheDocument();
    });

    it("indents the subtasks of a parent in Hierarchy and gives the parent a caret", async () => {
      await renderTable(modeData("hierarchy"));

      expect(await screen.findByRole("row", { name: /NW-1043/ })).toHaveClass("table-row--child");
      expect(rowOf(/NW-1042/)).not.toHaveClass("table-row--child");
      expect(within(rowOf(/NW-1042/)).getByRole("button", { name: "Collapse subtasks" })).toHaveAttribute("aria-expanded", "true");
    });

    it("indents rows by depth and gives a caret to a mid-level row in a multi-level tree", async () => {
      const mid = row({ id: "21", key: "NW-2002", summary: "Write the onboarding articles", parentId: "1", depth: 1, childCount: 1 });
      const leaf = row({ id: "22", key: "NW-2003", summary: "Proofread the welcome article", parentId: "21", depth: 2 });
      await renderTable(modeData("hierarchy", [PARENT, mid, leaf]));

      await screen.findByRole("row", { name: /NW-2002/ });
      expect([/NW-1042/, /NW-2002/, /NW-2003/].map((key) => rowOf(key).style.getPropertyValue("--depth"))).toEqual(["0", "1", "2"]);
      expect(within(rowOf(/NW-2002/)).getByRole("button", { name: "Collapse subtasks" })).toBeInTheDocument();
      expect(within(rowOf(/NW-2003/)).queryByRole("button", { name: /subtasks$/ })).not.toBeInTheDocument();
    });

    describe("indent guides", () => {
      const mid = row({ id: "21", key: "NW-2002", summary: "Write the onboarding articles", parentId: "1", depth: 1, childCount: 1 });
      const leaf = row({ id: "22", key: "NW-2003", summary: "Proofread the welcome article", parentId: "21", depth: 2 });
      const sibling = row({ id: "23", key: "NW-2004", summary: "Set up the search index", parentId: "1", depth: 1 });
      const tree = [PARENT, mid, leaf, sibling];
      const guidesOf = (key: RegExp) => [...rowOf(key).querySelectorAll(".tree-guide")];
      const isMarked = (guide: Element) => guide.classList.contains("tree-guide--through") || guide.classList.contains("tree-guide--end");

      it("draws one hidden guide per level on a nested row and none on a top-level row", async () => {
        await renderTable(modeData("hierarchy", tree));

        await screen.findByRole("row", { name: /NW-2003/ });
        expect([/NW-1042/, /NW-2002/, /NW-2003/].map((key) => guidesOf(key).length)).toEqual([0, 1, 2]);
        expect(rowOf(/NW-2003/).querySelector(".tree-guides")).toHaveAttribute("aria-hidden", "true");
      });

      it("shapes each level as a tree: a branch to a child with a later sibling, a line passing by, the last child's curve", async () => {
        await renderTable(modeData("hierarchy", tree));

        await screen.findByRole("row", { name: /NW-2003/ });
        expect(guidesOf(/NW-2002/)[0]).toHaveClass("tree-guide--branch");
        expect(guidesOf(/NW-2003/)[0]).toHaveClass("tree-guide--pass");
        expect(guidesOf(/NW-2003/)[1]).toHaveClass("tree-guide--last");
        expect(guidesOf(/NW-2004/)[0]).toHaveClass("tree-guide--last");
      });

      it("marks the path up to the parent while the pointer is over a depth-2 row, and no other row", async () => {
        await renderTable(modeData("hierarchy", tree));
        await screen.findByRole("row", { name: /NW-2003/ });

        await userEvent.hover(rowOf(/NW-2003/));

        expect(guidesOf(/NW-2003/)[1]).toHaveClass("tree-guide--end");
        expect(guidesOf(/NW-2002/)[0]).toHaveClass("tree-guide--end");
        expect(guidesOf(/NW-2002/)[0]).not.toHaveClass("tree-guide--through");
        expect(guidesOf(/NW-2004/).some(isMarked)).toBe(false);
      });
    });

    it.each([
      ["hierarchy", true],
      ["list", false],
      ["hidden", false],
    ] as const)("indents the header row only in Hierarchy (%s)", async (subtasks, indented) => {
      await renderTable(modeData(subtasks));
      await screen.findByRole("row", { name: /NW-1042/ });

      const header = within(screen.getByRole("grid")).getAllByRole("row")[0];

      expect(header).toHaveClass("table-header");
      if (indented) expect(header).toHaveClass("table-header--tree");
      else expect(header).not.toHaveClass("table-header--tree");
    });

    it("collapses and expands the subtasks with the caret, keeping the collapse in the view's place", async () => {
      await renderTable(modeData("hierarchy"));

      await userEvent.click(await screen.findByRole("button", { name: "Collapse subtasks" }));

      expect(screen.queryByText("Draft the price table")).not.toBeInTheDocument();
      expect(useStore.getState().places["11:101"].collapsedParents).toEqual(["1"]);
      expect(within(rowOf(/NW-1042/)).getByLabelText("2 child issues")).toBeInTheDocument();
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
