import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Row, type ViewData, type ViewPlace } from "../../api";
import { installCommands } from "../../commands";
import { getCommand } from "../../commands/registry";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { ALEX, COLUMNS, groupOf, row, SAM, viewData } from "../../test/issues";
import { connectionView } from "../../test/jira";
import { startMotionTimers, stubMotion } from "../../test/motion";
import { ListView } from "./ListView";

const mocked = vi.mocked(api, true);

type Restorable = { mockRestore: () => void };
let spies: Restorable[] = [];

const place = () => useStore.getState().places["11:101"];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.columnsCatalogue.mockResolvedValue(COLUMNS);
  mocked.placeGet.mockResolvedValue(null);
  mocked.placeSet.mockResolvedValue(undefined);
  mocked.copyText.mockResolvedValue(undefined);
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));
  spies = [
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(1000),
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(800),
  ];
});

afterEach(() => {
  spies.forEach((s) => s.mockRestore());
});

async function renderList(data: ViewData) {
  mocked.viewRows.mockResolvedValue(data);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ListView listId={11} viewId={101} />
    </QueryClientProvider>,
  );
  await screen.findByRole("grid");
  await waitFor(() => expect(place()).toBeDefined());
}

const OPEN = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", commentCount: 3 });
const DONE = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes", style: "done", statusName: "Done" });

describe("ListView", () => {
  it("renders nothing until the rows have loaded", () => {
    mocked.viewRows.mockReturnValue(new Promise(() => {}));
    render(
      <QueryClientProvider client={createQueryClient()}>
        <ListView listId={11} viewId={101} />
      </QueryClientProvider>,
    );

    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  it("asks Rust for the rows of the list and view", async () => {
    await renderList(viewData([groupOf([OPEN])], [OPEN]));

    expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, { me: false, text: "", epic: null });
  });

  describe("rows", () => {
    it("shows key, summary and comment count", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      const issue = await screen.findByRole("row", { name: /NW-1042/ });
      expect(issue).toHaveTextContent("Redesign the pricing page");
      expect(within(issue).getByLabelText("3 comments")).toBeInTheDocument();
    });

    it("shows the change chip only for a row with a change note", async () => {
      const changed = row({ id: "3", key: "NW-1003", summary: "Fix the footer links", changed: true, changeNote: "2 new comments" });
      await renderList(viewData([groupOf([OPEN, changed])], [OPEN, changed]));

      expect(within(await screen.findByRole("row", { name: /NW-1003/ })).getByText("2 new comments")).toBeInTheDocument();
      expect(screen.getAllByText(/new comments/)).toHaveLength(1);
    });

    it("marks a changed row and leaves the others plain", async () => {
      const changed = row({ id: "3", key: "NW-1003", summary: "Fix the footer links", changed: true });
      await renderList(viewData([groupOf([OPEN, changed])], [OPEN, changed]));

      expect(await screen.findByRole("row", { name: /NW-1003/ })).toHaveClass("list-row--changed");
      expect(screen.getByRole("row", { name: /NW-1042/ })).not.toHaveClass("list-row--changed");
    });

    it("selects the row that is clicked", async () => {
      await renderList(viewData([groupOf([OPEN, DONE])], [OPEN, DONE]));

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "false");
    });

    it("remembers the clicked row as the view's selection and opens its peek", async () => {
      await renderList(viewData([groupOf([OPEN, DONE])], [OPEN, DONE]));

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(place()).toMatchObject({ selectedId: "2", peekOpen: true });
    });
  });

  describe("title wrapping", () => {
    const renderWith = (wrapTitles?: boolean) => {
      mocked.viewRows.mockResolvedValue(viewData([groupOf([OPEN])], [OPEN]));
      return render(
        <QueryClientProvider client={createQueryClient()}>
          <ListView listId={11} viewId={101} wrapTitles={wrapTitles} />
        </QueryClientProvider>,
      );
    };

    it.each([
      ["by default", undefined],
      ["when wrapping is on", true],
    ])("lets the rows wrap their titles %s", async (_, wrapTitles) => {
      renderWith(wrapTitles);

      expect(await screen.findByRole("row", { name: /NW-1042/ })).toHaveClass("list-row--wrap");
    });

    it("keeps titles on one line when wrapping is off", async () => {
      renderWith(false);

      expect(await screen.findByRole("row", { name: /NW-1042/ })).not.toHaveClass("list-row--wrap");
    });
  });

  describe("saved place", () => {
    const groups = () => [
      groupOf([OPEN], { key: "status:progress", label: "In Progress" }),
      groupOf([DONE], { key: "status:done", label: "Done", collapsedByDefault: true }),
    ];
    const saved: ViewPlace = { scrollTop: 0, toggled: { "status:done": true, "status:progress": false }, selectedId: "2", peekOpen: false };

    it("restores the selection and the opened and closed groups", async () => {
      mocked.placeGet.mockResolvedValue(saved);
      await renderList(viewData(groups(), [OPEN, DONE]));

      expect(await screen.findByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Collapse Done" })).toHaveAttribute("aria-expanded", "true");
    });

    it("asks Rust for the place of this list and view", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      expect(mocked.placeGet).toHaveBeenCalledWith(11, 101);
    });

    it("keeps a toggled group in the place and saves it", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      await userEvent.click(await screen.findByRole("button", { name: "Expand Done" }));

      expect(place().toggled).toEqual({ "status:done": true });
      await waitFor(() => expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, expect.objectContaining({ toggled: { "status:done": true } })));
    });
  });

  describe("row actions", () => {
    const action = (name: string) => screen.findByRole("button", { name });

    it("copies the key without selecting the row", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      await userEvent.click(await action("Copy NW-1042"));

      expect(mocked.copyText).toHaveBeenCalledWith("NW-1042");
      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "false");
      expect(place().selectedId).toBeNull();
    });

    it("opens the issue on the Jira site without selecting the row", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "https://example.atlassian.net" }));
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      await userEvent.click(await action("Open NW-1042 in Jira"));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-1042");
      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "false");
    });

    it("offers no Open in Jira without a site", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]));
      await screen.findByRole("row", { name: /NW-1042/ });
      await waitFor(() => expect(mocked.connectionGet).toHaveBeenCalled());

      expect(screen.queryByRole("button", { name: /in Jira$/ })).not.toBeInTheDocument();
    });

    it("shows Pin disabled", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      expect(await action("Pin NW-1042")).toBeDisabled();
    });
  });

  describe("issue keys", () => {
    const FOLLOW_UP = row({ id: "3", key: "NW-1003", summary: "Fix the footer links" });
    const LAST = row({ id: "4", key: "NW-1004", summary: "Plan the content migration" });
    const rows = [OPEN, FOLLOW_UP, LAST];

    const press = (id: string) => act(async () => void (await getCommand(id)?.run()));
    const selectedKeys = () =>
      screen
        .getAllByRole("row")
        .filter((r) => r.getAttribute("aria-selected") === "true")
        .map((r) => /NW-\d+/.exec(r.textContent ?? "")?.[0]);

    beforeEach(() => {
      installCommands("main");
    });

    it("selects the first row on the first next, then the following ones", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await screen.findByRole("row", { name: /NW-1042/ });

      await press("issue.next");
      expect(selectedKeys()).toEqual(["NW-1042"]);

      await press("issue.next");
      expect(selectedKeys()).toEqual(["NW-1003"]);
    });

    it("goes back with previous", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await userEvent.click(await screen.findByText("Fix the footer links"));

      await press("issue.previous");

      expect(selectedKeys()).toEqual(["NW-1042"]);
    });

    it("stays on the first row at the top and on the last row at the bottom", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      await press("issue.previous");
      expect(selectedKeys()).toEqual(["NW-1042"]);

      await press("issue.next");
      await press("issue.next");
      await press("issue.next");
      expect(selectedKeys()).toEqual(["NW-1004"]);
      expect(place().selectedId).toBe("4");
    });

    it("moves through the rows of the open groups in order", async () => {
      const groups = [groupOf([OPEN], { key: "a", label: "Alpha" }), groupOf([FOLLOW_UP], { key: "b", label: "Beta" })];
      await renderList(viewData(groups, [OPEN, FOLLOW_UP]));
      await screen.findByRole("row", { name: /NW-1042/ });

      await press("issue.next");
      await press("issue.next");

      expect(selectedKeys()).toEqual(["NW-1003"]);
    });

    it("keeps the peek open or closed as it was while moving", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await userEvent.click(await screen.findByText("Redesign the pricing page"));
      await press("peek.close");
      expect(place().peekOpen).toBe(false);

      await press("issue.next");

      expect(place()).toMatchObject({ selectedId: "3", peekOpen: false });
    });

    it("opens the selected issue in the Issue view with the view as origin", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await userEvent.click(await screen.findByText("Fix the footer links"));

      await press("issue.open");

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["3"] });
    });

    it("opens nothing without a selected issue", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await screen.findByRole("row", { name: /NW-1042/ });

      await press("issue.open");

      expect(useStore.getState().issueView).toBeNull();
    });

    it("opens the selected issue's epic in the Issue view", async () => {
      const inEpic = row({ id: "5", key: "NW-1005", summary: "Review the copy", epic: { id: "9", name: "Launch Q4", color: 2 } });
      await renderList(viewData([groupOf([OPEN, inEpic])], [OPEN, inEpic]));
      await userEvent.click(await screen.findByText("Review the copy"));

      await press("issue.openEpic");

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["9"] });
    });

    it("opens no epic for an issue without one", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await userEvent.click(await screen.findByText("Fix the footer links"));

      await press("issue.openEpic");

      expect(useStore.getState().issueView).toBeNull();
    });

    it("collapses the selected row's group", async () => {
      const groups = [groupOf([OPEN], { key: "a", label: "Alpha" }), groupOf([FOLLOW_UP], { key: "b", label: "Beta" })];
      await renderList(viewData(groups, [OPEN, FOLLOW_UP]));
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      await press("group.collapse");

      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
      expect(screen.getByText("Fix the footer links")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand Alpha" })).toBeInTheDocument();
    });
  });

  describe("narrow width", () => {
    afterEach(() => {
      vi.unstubAllGlobals();
    });

    it("hides the columns that do not fit from the right and keeps the title", async () => {
      vi.stubGlobal(
        "ResizeObserver",
        class {
          constructor(private callback: (entries: unknown[], observer: unknown) => void) {}
          observe() {
            this.callback([], this);
          }
          unobserve() {}
          disconnect() {}
        },
      );
      spies.push(vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(400));
      const issue = row({ id: "4", key: "NW-1004", summary: "Plan the content migration", assignees: [ALEX], priority: { name: "High", level: 2 } });

      await renderList(viewData([groupOf([issue])], [issue], { columns: ["assignee", "priority"], groupBy: "none" }));

      await screen.findByText("Plan the content migration");
      expect((await screen.findAllByRole("columnheader")).map((h) => h.textContent)).toEqual([""]);
      expect(screen.queryByText("High")).not.toBeInTheDocument();
    });
  });

  describe("columns", () => {
    const withEpic = row({
      id: "4",
      key: "NW-1004",
      summary: "Plan the content migration",
      assignees: [ALEX],
      epic: { id: "9", name: "Launch Q4", color: 2 },
      priority: { name: "High", level: 2 },
    });

    it("shows headers and cells for the view's chosen columns only", async () => {
      await renderList(viewData([groupOf([withEpic])], [withEpic], { columns: ["assignee", "priority"], groupBy: "none" }));

      const headers = (await screen.findAllByRole("columnheader")).map((h) => h.textContent);
      expect(headers).toEqual(["", "Assignee", "Priority"]);
      const issue = screen.getByRole("row", { name: /NW-1004/ });
      expect(within(issue).getByRole("img", { name: "Alex Morgan" })).toBeInTheDocument();
      expect(within(issue).getByText("High")).toBeInTheDocument();
      expect(screen.queryByText("Launch Q4")).not.toBeInTheDocument();
    });

    it("shows the status as a pill in the row when the view has the Status column, beside the title's status square", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN], { columns: ["status"], groupBy: "none" }));

      expect(await screen.findByRole("columnheader", { name: "Status" })).toBeInTheDocument();
      const issue = screen.getByRole("row", { name: /NW-1042/ });
      expect(within(issue).getByText("In Progress")).toBeInTheDocument();
      expect(issue.querySelector(".status-square")).not.toBeNull();
    });

    it("shows the status square but no pill when the view lacks the Status column", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN], { columns: ["assignee"], groupBy: "none" }));

      const issue = await screen.findByRole("row", { name: /NW-1042/ });
      expect(within(issue).queryByText("In Progress")).not.toBeInTheDocument();
      expect(issue.querySelector(".status-square")).not.toBeNull();
    });

    it("shows the epic chip when the view has the Epic column", async () => {
      await renderList(viewData([groupOf([withEpic])], [withEpic], { columns: ["epic"] }));

      expect(await screen.findByText("Launch Q4")).toBeInTheDocument();
    });

    it("sets the scroller's column template with the title taking the remaining space", async () => {
      await renderList(viewData([groupOf([withEpic])], [withEpic], { columns: ["assignee", "priority"] }));

      const template = (await screen.findByRole("grid")).style.getPropertyValue("--list-columns");
      expect(template).toMatch(/^minmax\(0, 1fr\) \d+px \d+px$/);
    });

    it("sizes a column wider for a longer value, across all groups", async () => {
      const short = row({ id: "6", key: "NW-1006", priority: { name: "Low", level: 1 } });
      const long = row({ id: "7", key: "NW-1007", priority: { name: "Highest priority of all", level: 3 } });
      const groups = [groupOf([short], { key: "a", label: "Alpha" }), groupOf([long], { key: "b", label: "Beta", collapsedByDefault: true })];
      await renderList(viewData(groups, [short, long], { columns: ["priority"] }));

      const template = (await screen.findByRole("grid")).style.getPropertyValue("--list-columns");
      // 14 + 6 + 23 chars at the 7px estimate, though the longest value sits in a collapsed group
      expect(template).toBe("minmax(0, 1fr) 181px");
    });

    it("shows a person custom field as an avatar under the field's name", async () => {
      const reviewed: Row = row({ id: "5", key: "NW-1005", summary: "Review the copy", custom: { customfield_10071: { kind: "person", value: SAM } } });
      await renderList(viewData([groupOf([reviewed])], [reviewed], { columns: ["customfield_10071"] }));

      expect(await screen.findByRole("columnheader", { name: "Reviewer" })).toBeInTheDocument();
      expect(within(screen.getByRole("row", { name: /NW-1005/ })).getByRole("img", { name: "Sam Lee" })).toBeInTheDocument();
    });
  });

  describe("groups", () => {
    const groups = () => [
      groupOf([OPEN], { key: "status:progress", label: "In Progress", mark: { kind: "status", style: "progress" } }),
      groupOf([DONE], { key: "status:done", label: "Done", count: 12, collapsedByDefault: true, mark: { kind: "status", style: "done" } }),
    ];

    const groupRow = (label: string) => screen.getByRole("button", { name: new RegExp(`^(Collapse|Expand) ${label}$`) }).closest<HTMLElement>('[role="row"]')!;

    it("shows a collapsed-by-default group with its header and count but not its rows", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      expect(await screen.findByRole("button", { name: "Done" })).toBeInTheDocument();
      expect(screen.getByText("12")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand Done" })).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByText("Publish the launch notes")).not.toBeInTheDocument();
    });

    it("labels the chevron of an open group as collapsing it", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      expect(await screen.findByRole("button", { name: "Collapse In Progress" })).toHaveAttribute("aria-expanded", "true");
    });

    it("shows the rows of a collapsed group once its chevron is clicked", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      await userEvent.click(await screen.findByRole("button", { name: "Expand Done" }));

      expect(await screen.findByText("Publish the launch notes")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Collapse Done" })).toHaveAttribute("aria-expanded", "true");
    });

    it("hides the rows of an open group when its chevron is clicked", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));
      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Collapse In Progress" }));

      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
    });

    it("toggles the group when its tab is clicked, keeping the tab and the count", async () => {
      await renderList(viewData([groupOf([OPEN], { label: "In Progress", count: 7 })], [OPEN]));

      await userEvent.click(await screen.findByRole("button", { name: "In Progress" }));

      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand In Progress" })).toHaveAttribute("aria-expanded", "false");
      expect(screen.getByRole("button", { name: "In Progress" })).toBeInTheDocument();
      expect(screen.getByText("7")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "In Progress" }));

      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
    });

    it("lists the visible columns' names in every group header", async () => {
      await renderList(viewData(groups(), [OPEN, DONE], { columns: ["assignee", "priority"] }));

      await screen.findByRole("button", { name: "Collapse In Progress" });
      for (const label of ["In Progress", "Done"]) {
        const names = within(groupRow(label)).getAllByRole("columnheader").map((h) => h.textContent);
        expect(names.slice(1)).toEqual(["Assignee", "Priority"]);
      }
    });

    it("shows one tabless header with the column names and no chevron when the view is not grouped", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN], { groupBy: "none", columns: ["epic"] }));

      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^(Collapse|Expand) / })).not.toBeInTheDocument();
      expect(screen.getByRole("columnheader", { name: "Epic" })).toBeInTheDocument();
    });

    describe("card positions", () => {
      const second = row({ id: "8", key: "NW-1008", summary: "Update the footer" });
      const third = row({ id: "9", key: "NW-1009", summary: "Archive the old blog" });

      it("marks the first row of a card first and the last row last, with the rows between in the middle", async () => {
        await renderList(viewData([groupOf([OPEN, second, third])], [OPEN, second, third]));

        expect(await screen.findByRole("row", { name: /NW-1042/ })).toHaveClass("list-row--first");
        expect(screen.getByRole("row", { name: /NW-1008/ })).toHaveClass("list-row--middle");
        expect(screen.getByRole("row", { name: /NW-1009/ })).toHaveClass("list-row--last");
      });

      it("marks the only row of a card only", async () => {
        await renderList(viewData([groupOf([OPEN])], [OPEN]));

        expect(await screen.findByRole("row", { name: /NW-1042/ })).toHaveClass("list-row--only");
      });

      it("positions the rows of each group within its own card", async () => {
        await renderList(viewData([groupOf([OPEN, second], { key: "a", label: "Alpha" }), groupOf([third], { key: "b", label: "Beta" })], [OPEN, second, third]));

        expect(await screen.findByRole("row", { name: /NW-1008/ })).toHaveClass("list-row--last");
        expect(screen.getByRole("row", { name: /NW-1009/ })).toHaveClass("list-row--only");
      });
    });

    describe("active header", () => {
      it("marks the header of the group the pointer is over and clears it when the pointer leaves the grid", async () => {
        await renderList(viewData(groups(), [OPEN, DONE]));
        const issue = await screen.findByRole("row", { name: /NW-1042/ });

        await userEvent.hover(issue);

        expect(groupRow("In Progress")).toHaveClass("list-group--active");
        expect(groupRow("Done")).not.toHaveClass("list-group--active");

        await userEvent.unhover(issue);

        expect(groupRow("In Progress")).not.toHaveClass("list-group--active");
      });

      it("moves the mark to another group when the pointer moves there", async () => {
        await renderList(viewData(groups(), [OPEN, DONE]));
        await screen.findByRole("row", { name: /NW-1042/ });

        await userEvent.hover(groupRow("In Progress"));
        await userEvent.hover(groupRow("Done"));

        expect(groupRow("Done")).toHaveClass("list-group--active");
        expect(groupRow("In Progress")).not.toHaveClass("list-group--active");
      });

      it("marks the header of the group that has focus and follows the focus", async () => {
        await renderList(viewData(groups(), [OPEN, DONE]));
        await screen.findByRole("button", { name: "Collapse In Progress" });

        act(() => screen.getByRole("button", { name: "Collapse In Progress" }).focus());

        expect(groupRow("In Progress")).toHaveClass("list-group--active");
        expect(groupRow("Done")).not.toHaveClass("list-group--active");

        act(() => screen.getByRole("button", { name: "Expand Done" }).focus());

        expect(groupRow("Done")).toHaveClass("list-group--active");
        expect(groupRow("In Progress")).not.toHaveClass("list-group--active");
      });

      it("clears the mark when focus leaves the grid", async () => {
        await renderList(viewData(groups(), [OPEN, DONE]));
        await screen.findByRole("button", { name: "Collapse In Progress" });

        const chevron = screen.getByRole("button", { name: "Expand Done" });
        act(() => chevron.focus());
        expect(groupRow("Done")).toHaveClass("list-group--active");
        act(() => chevron.blur());

        expect(groupRow("Done")).not.toHaveClass("list-group--active");
      });
    });
  });

  describe("with motion", () => {
    const groups = () => [
      groupOf([OPEN], { key: "status:progress", label: "In Progress" }),
      groupOf([DONE], { key: "status:done", label: "Done", collapsedByDefault: true }),
    ];

    afterEach(() => {
      vi.useRealTimers();
    });

    async function renderWithMotion() {
      spies.push(stubMotion({ "--motion-slow": "200ms" }));
      await renderList(viewData(groups(), [OPEN, DONE]));
      await screen.findByText("Redesign the pricing page");
      startMotionTimers();
    }

    it("keeps the rows of a collapsing group until the motion time has passed, with the chevron already collapsed", async () => {
      await renderWithMotion();

      await userEvent.click(screen.getByRole("button", { name: "Collapse In Progress" }));

      expect(screen.getByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand In Progress" })).toHaveAttribute("aria-expanded", "false");
      await act(() => vi.advanceTimersByTimeAsync(400));
      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
    });

    it("renders the rows of an expanding group at once", async () => {
      await renderWithMotion();

      await userEvent.click(screen.getByRole("button", { name: "Expand Done" }));

      expect(screen.getByText("Publish the launch notes")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Collapse Done" })).toHaveAttribute("aria-expanded", "true");
      await act(() => vi.advanceTimersByTimeAsync(400));
      expect(screen.getByText("Publish the launch notes")).toBeInTheDocument();
    });
  });

  describe("virtualisation", () => {
    it("renders only the visible part of 5,000 rows", async () => {
      const many = Array.from({ length: 5000 }, (_, i) => row({ id: String(i), key: `NW-${i + 1}`, summary: `Task ${i + 1}` }));
      await renderList(viewData([groupOf(many)], many));

      await screen.findByText("Task 1");
      const rendered = within(screen.getByRole("grid")).getAllByRole("row").length;
      expect(rendered).toBeGreaterThan(0);
      expect(rendered).toBeLessThan(200);
    });
  });
});
