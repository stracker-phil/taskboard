import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Row, type ViewData } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { ALEX, COLUMNS, groupOf, row, SAM, viewData } from "../../test/issues";
import { ListView } from "./ListView";

const mocked = vi.mocked(api, true);

type Restorable = { mockRestore: () => void };
let spies: Restorable[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  mocked.columnsCatalogue.mockResolvedValue(COLUMNS);
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
      await renderList(viewData([groupOf([withEpic])], [withEpic], { columns: ["assignee", "priority"] }));

      const headers = (await screen.findAllByRole("columnheader")).map((h) => h.textContent);
      expect(headers).toEqual(["Name", "Assignee", "Priority"]);
      const issue = screen.getByRole("row", { name: /NW-1004/ });
      expect(within(issue).getByRole("img", { name: "Alex Morgan" })).toBeInTheDocument();
      expect(within(issue).getByText("High")).toBeInTheDocument();
      expect(screen.queryByText("Launch Q4")).not.toBeInTheDocument();
    });

    it("shows the epic chip when the view has the Epic column", async () => {
      await renderList(viewData([groupOf([withEpic])], [withEpic], { columns: ["epic"] }));

      expect(await screen.findByText("Launch Q4")).toBeInTheDocument();
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

    it("shows a collapsed-by-default group with its header and count but not its rows", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      expect(await screen.findByText("Done")).toBeInTheDocument();
      expect(screen.getByText("12")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand group" })).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByText("Publish the launch notes")).not.toBeInTheDocument();
    });

    it("shows the rows of a collapsed group once it is expanded", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      await userEvent.click(await screen.findByRole("button", { name: "Expand group" }));

      expect(await screen.findByText("Publish the launch notes")).toBeInTheDocument();
    });

    it("hides the rows of an open group when it is collapsed", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));
      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Collapse group" }));

      expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
    });

    it("shows no group headers when the view is not grouped", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN], { groupBy: "none" }));

      expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /group/ })).not.toBeInTheDocument();
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
