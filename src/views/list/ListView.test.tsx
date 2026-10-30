import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type IssueOpening, type Narrowing, type Row, type ViewData, type ViewPlace } from "../../api";
import { installCommands } from "../../commands";
import { getCommand } from "../../commands/registry";
import { createQueryClient } from "../../queries/queryClient";
import { NO_NARROWING } from "../../queries/views";
import { resetStore, useStore } from "../../state/store";
import { ALEX, COLUMNS, groupOf, row, SAM, viewData } from "../../test/issues";
import { connectionView } from "../../test/jira";
import { startMotionTimers, stubMotion } from "../../test/motion";
import { issueTarget, pinItem, pins } from "../../test/pins";
import { settings } from "../../test/settings";
import { sidebarTree, space } from "../../test/spaces";
import { ListView } from "./ListView";

const mocked = vi.mocked(api, true);

type Restorable = { mockRestore: () => void };
let spies: Restorable[] = [];

const place = () => useStore.getState().places["11:101"];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getSettings.mockResolvedValue(settings());
  mocked.sidebarTree.mockResolvedValue(sidebarTree([]));
  mocked.pinsList.mockResolvedValue(pins());
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

async function renderList(data: ViewData, openWith: IssueOpening | null = null) {
  mocked.viewRows.mockResolvedValue(data);
  const client = createQueryClient();
  const tree = (narrowing: Narrowing) => (
    <QueryClientProvider client={client}>
      <ListView listId={11} viewId={101} openWith={openWith} narrowing={narrowing} />
    </QueryClientProvider>
  );
  const { rerender } = render(tree(NO_NARROWING));
  await screen.findByRole("grid");
  await waitFor(() => expect(place()).toBeDefined());
  await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
  await act(async () => {});
  return { narrowTo: (narrowing: Narrowing) => rerender(tree(narrowing)) };
}

const OPEN = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", commentCount: 3 });
const DONE = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes", style: "done", statusName: "Done" });

/** A preview opens once the double-click interval (500ms in the API mock) has passed. */
const PREVIEW_WAIT = { timeout: 2000 };
const pastDoubleClickInterval = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 700)));

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

    expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, { me: false, text: "", epic: null, listDraft: null, viewDraft: null });
  });

  it("shows what the level is missing instead of a grid when the view has no issues", async () => {
    mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
    mocked.viewRows.mockResolvedValue(viewData([], []));

    render(
      <QueryClientProvider client={createQueryClient()}>
        <ListView listId={11} viewId={101} />
      </QueryClientProvider>,
    );

    expect(await screen.findByRole("heading", { name: "Nothing in Website relaunch" })).toBeInTheDocument();
    expect(screen.getByText("No issues here yet.")).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
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

    it("offers Mark as read on an unread row, which marks it read without selecting it", async () => {
      const changed = row({ id: "3", key: "NW-1003", summary: "Fix the footer links", changed: true, changeNote: "2 new comments" });
      await renderList(viewData([groupOf([OPEN, changed])], [OPEN, changed]));

      await userEvent.click(await screen.findByRole("button", { name: "Mark NW-1003 as read" }));

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["3"]);
      expect(screen.getByRole("row", { name: /NW-1003/ })).toHaveAttribute("aria-selected", "false");
    });

    it("offers Mark as unread on a read row, which marks it unread without selecting it", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      await userEvent.click(await screen.findByRole("button", { name: "Mark NW-1042 as unread" }));

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("1");
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "false");
    });

    it("offers only one of Mark as read and Mark as unread on a row", async () => {
      const changed = row({ id: "3", key: "NW-1003", summary: "Fix the footer links", changed: true });
      await renderList(viewData([groupOf([OPEN, changed])], [OPEN, changed]));

      await screen.findByRole("row", { name: /NW-1003/ });
      expect(screen.queryByRole("button", { name: "Mark NW-1003 as unread" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Mark NW-1042 as read" })).not.toBeInTheDocument();
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

      expect(place().selectedId).toBe("2");
      await waitFor(() => expect(place().peekOpen).toBe(true), PREVIEW_WAIT);
    });

    it("asks Rust to sync the history and web links of the clicked row", async () => {
      await renderList(viewData([groupOf([OPEN, DONE])], [OPEN, DONE]));

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      expect(mocked.issueSyncHistory).toHaveBeenCalledWith("2");
      expect(mocked.issueSyncWebLinks).toHaveBeenCalledWith("2");
    });

    it("shows no toast when syncing the history and web links fails", async () => {
      mocked.issueSyncHistory.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      mocked.issueSyncWebLinks.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      await renderList(viewData([groupOf([OPEN, DONE])], [OPEN, DONE]));

      await userEvent.click(await screen.findByText("Publish the launch notes"));

      await waitFor(() => expect(mocked.issueSyncWebLinks).toHaveBeenCalled());
      expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
      expect(useStore.getState().toasts).toEqual([]);
    });
  });

  describe("opening an issue by click", () => {
    const click = async () => userEvent.click(await screen.findByText("Publish the launch notes"));
    const data = () => viewData([groupOf([OPEN, DONE])], [OPEN, DONE]);

    it("selects the row, opens the preview and no Issue view when the view opens with the preview", async () => {
      await renderList(data(), "preview");

      await click();

      await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }), PREVIEW_WAIT);
      expect(useStore.getState().issueView).toBeNull();
    });

    it("selects the row at once and opens the preview only after the double-click interval", async () => {
      await renderList(data(), "preview");

      await click();

      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });
      await waitFor(() => expect(place().peekOpen).toBe(true), PREVIEW_WAIT);
    });

    it("moves an open preview to the clicked row at once", async () => {
      await renderList(data(), "preview");
      await userEvent.click(await screen.findByText("Redesign the pricing page"));
      await waitFor(() => expect(place()).toMatchObject({ selectedId: "1", peekOpen: true }), PREVIEW_WAIT);

      await click();

      expect(place()).toMatchObject({ selectedId: "2", peekOpen: true });
    });

    it("does not open the preview for a row the selection has left in the meantime", async () => {
      installCommands("main");
      await renderList(data(), "preview");
      await userEvent.click(await screen.findByText("Redesign the pricing page"));

      await act(async () => void (await getCommand("issue.next")?.run()));
      await pastDoubleClickInterval();

      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });
    });

    it("selects the row without the preview and opens the Issue view over the view when the view opens in full", async () => {
      await renderList(data(), "full");

      await click();

      expect(screen.getByRole("row", { name: /NW-1001/ })).toHaveAttribute("aria-selected", "true");
      expect(place().peekOpen).toBe(false);
      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
    });

    it("follows the app setting when the view has no choice", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "untilUnpinned", openIssuesIn: "full" } }));
      await renderList(data());

      await click();

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
    });

    it("prefers the view's choice over the app setting", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "untilUnpinned", openIssuesIn: "full" } }));
      await renderList(data(), "preview");

      await click();

      await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }), PREVIEW_WAIT);
      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("opening an issue by double-click", () => {
    const data = () => viewData([groupOf([OPEN, DONE])], [OPEN, DONE]);

    it("opens the Issue view alone, never the preview, even after the interval has passed", async () => {
      await renderList(data(), "preview");

      await userEvent.dblClick(await screen.findByText("Publish the launch notes"));
      await pastDoubleClickInterval();

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
      expect(place()).toMatchObject({ selectedId: "2", peekOpen: false });
    });

    it("opens the Issue view even when the view opens with the preview", async () => {
      await renderList(data(), "preview");

      await userEvent.dblClick(await screen.findByText("Publish the launch notes"));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["2"] });
    });

    it("opens no Issue view when a row's action button is double-clicked", async () => {
      await renderList(data(), "preview");

      await userEvent.dblClick(await screen.findByRole("button", { name: "Copy NW-1001" }));

      expect(useStore.getState().issueView).toBeNull();
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

    it("still shows a wrapping row's change note and comment count", async () => {
      const changed = row({ id: "3", key: "NW-1003", summary: "Fix the footer links", changed: true, changeNote: "2 new comments", commentCount: 4 });
      mocked.viewRows.mockResolvedValue(viewData([groupOf([changed])], [changed]));
      render(
        <QueryClientProvider client={createQueryClient()}>
          <ListView listId={11} viewId={101} wrapTitles />
        </QueryClientProvider>,
      );

      const issue = await screen.findByRole("row", { name: /NW-1003/ });
      expect(within(issue).getByText("2 new comments")).toBeInTheDocument();
      expect(within(issue).getByLabelText("4 comments")).toBeInTheDocument();
    });

    it("keeps titles on one line when wrapping is off", async () => {
      renderWith(false);

      expect(await screen.findByRole("row", { name: /NW-1042/ })).not.toHaveClass("list-row--wrap");
    });
  });

  describe("inline labels", () => {
    const LABELED = row({ id: "5", key: "NW-1005", summary: "Update the footer", labels: [{ name: "launch", tint: 1 }] });
    const renderWith = async (inlineLabels: boolean | undefined, columns: string[]) => {
      mocked.viewRows.mockResolvedValue(viewData([groupOf([LABELED])], [LABELED], { columns }));
      render(
        <QueryClientProvider client={createQueryClient()}>
          <ListView listId={11} viewId={101} inlineLabels={inlineLabels} />
        </QueryClientProvider>,
      );
      const issue = await screen.findByRole("row", { name: /NW-1005/ });
      return within(issue).getAllByRole("gridcell");
    };

    it.each([
      ["by default", undefined],
      ["when inline labels are on", true],
    ])("shows the labels after the title %s", async (_, inlineLabels) => {
      const [title] = await renderWith(inlineLabels, []);

      expect(within(title).getByText("launch")).toBeInTheDocument();
    });

    it("shows no labels after the title when inline labels are off", async () => {
      await renderWith(false, []);

      expect(screen.queryByText("launch")).not.toBeInTheDocument();
    });

    it("shows the labels only in the Labels column when the view has it", async () => {
      const [title, labels] = await renderWith(true, ["labels"]);

      expect(within(title).queryByText("launch")).not.toBeInTheDocument();
      expect(within(labels).getByText("launch")).toBeInTheDocument();
    });
  });

  describe("saved place", () => {
    const groups = () => [
      groupOf([OPEN], { key: "status:progress", label: "In Progress" }),
      groupOf([DONE], { key: "status:done", label: "Done", collapsedByDefault: true }),
    ];
    const saved: ViewPlace = { scrollTop: 0, toggled: { "status:done": true, "status:progress": false }, collapsedParents: [], selectedId: "2", peekOpen: false };

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

    describe("with the preview open", () => {
      const previewing: ViewPlace = { ...saved, peekOpen: true };
      const pricing: Narrowing = { ...NO_NARROWING, text: "pricing" };
      const refetchWith = (rows: Row[]) => mocked.viewRows.mockResolvedValue(viewData([groupOf(rows)], rows));

      it("closes the preview and keeps the selection when a narrowing drops the selected issue", async () => {
        mocked.placeGet.mockResolvedValue(previewing);
        const { narrowTo } = await renderList(viewData(groups(), [OPEN, DONE]));
        await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }));

        refetchWith([OPEN]);
        narrowTo(pricing);

        await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: false }));
      });

      it("keeps the preview open when the selected issue is still among the rows", async () => {
        mocked.placeGet.mockResolvedValue(previewing);
        const { narrowTo } = await renderList(viewData(groups(), [OPEN, DONE]));
        await waitFor(() => expect(place()).toMatchObject({ selectedId: "2", peekOpen: true }));

        refetchWith([OPEN, DONE]);
        narrowTo(pricing);
        await waitFor(() => expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, expect.objectContaining({ text: "pricing" })));
        await act(async () => {});

        expect(place()).toMatchObject({ selectedId: "2", peekOpen: true });
      });
    });

    it("keeps a toggled group in the place and saves it", async () => {
      await renderList(viewData(groups(), [OPEN, DONE]));

      await userEvent.click(await screen.findByRole("button", { name: "Expand Done" }));

      expect(place().toggled).toEqual({ "status:done": true });
      await waitFor(() => expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, expect.objectContaining({ toggled: { "status:done": true } })));
    });
  });

  describe("subtasks", () => {
    const PARENT = row({ id: "1", key: "NW-1042", summary: "Redesign the pricing page", childCount: 2 });
    const STEP = row({ id: "11", key: "NW-1043", summary: "Draft the price table", isSubtask: true, parentId: "1", depth: 1 });
    const LAST_STEP = row({ id: "12", key: "NW-1044", summary: "Review the price table", isSubtask: true, parentId: "1", depth: 1 });
    const OTHER = row({ id: "2", key: "NW-1001", summary: "Publish the launch notes" });
    const modeData = (subtasks: ViewData["subtasks"], rows = [PARENT, STEP, LAST_STEP, OTHER]) => viewData([groupOf(rows)], rows, { subtasks });
    const rowOf = (key: RegExp) => screen.getByRole("row", { name: key });
    const caret = (name: "Collapse subtasks" | "Expand subtasks") => screen.findByRole("button", { name });
    const caretOf = (key: RegExp) => {
      const before = rowOf(key).previousElementSibling;
      return before instanceof HTMLButtonElement && /subtasks$/.test(before.getAttribute("aria-label") ?? "") ? before : null;
    };
    const toggleCaretOf = async (key: RegExp) => {
      const button = caretOf(key);
      if (!button) throw new Error(`No subtask caret before ${key}`);
      await userEvent.click(button);
    };

    it("shows a parent's child count as a chip after its title in Hidden", async () => {
      const single = row({ id: "3", key: "NW-1003", summary: "Fix the footer links", childCount: 1 });
      const many = row({ id: "4", key: "NW-1004", summary: "Migrate the blog posts", childCount: 76 });
      await renderList(modeData("hidden", [PARENT, single, many, OTHER]));

      expect(within(await screen.findByRole("row", { name: /NW-1042/ })).getByLabelText("2 child issues")).toBeInTheDocument();
      expect(within(rowOf(/NW-1003/)).getByLabelText("1 child issue")).toBeInTheDocument();
      expect(within(rowOf(/NW-1004/)).getByLabelText("76 child issues")).toBeInTheDocument();
      expect(within(rowOf(/NW-1001/)).queryByLabelText(/child issue/)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /subtasks$/ })).not.toBeInTheDocument();
    });

    it("indents the subtasks and gives the parent a caret in Hierarchy", async () => {
      await renderList(modeData("hierarchy"));

      expect(await screen.findByRole("row", { name: /NW-1043/ })).toHaveClass("list-row--child");
      expect(rowOf(/NW-1044/)).toHaveClass("list-row--child");
      expect(rowOf(/NW-1042/)).not.toHaveClass("list-row--child");
      expect(rowOf(/NW-1001/)).not.toHaveClass("list-row--child");
      expect(caretOf(/NW-1042/)).toHaveAccessibleName("Collapse subtasks");
      expect(caretOf(/NW-1042/)).toHaveAttribute("aria-expanded", "true");
      expect(within(rowOf(/NW-1042/)).queryByLabelText("2 child issues")).not.toBeInTheDocument();
      expect(caretOf(/NW-1001/)).toBeNull();
    });

    it("leaves the selection where it was when a caret is clicked", async () => {
      await renderList(modeData("hierarchy"));
      await userEvent.click(await screen.findByText("Publish the launch notes"));

      await toggleCaretOf(/NW-1042/);

      expect(rowOf(/NW-1001/)).toHaveAttribute("aria-selected", "true");
      expect(rowOf(/NW-1042/)).toHaveAttribute("aria-selected", "false");
      expect(place().selectedId).toBe("2");
    });

    it("selects nothing when a caret is clicked with no selection", async () => {
      await renderList(modeData("hierarchy"));
      await screen.findByRole("row", { name: /NW-1042/ });

      await toggleCaretOf(/NW-1042/);

      expect(place().selectedId).toBeNull();
    });

    it("shows the key and subtask icon inside the title, before the summary", async () => {
      await renderList(modeData("hierarchy"));

      const step = await screen.findByRole("row", { name: /NW-1043/ });
      const key = within(step).getByText("NW-1043");
      const summary = within(step).getByText("Draft the price table");
      expect(key.compareDocumentPosition(summary) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(summary.parentElement).toContainElement(key);
      expect(summary.parentElement).toContainElement(step.querySelector<HTMLElement>(".subtask-icon"));
    });

    it("copies a parent's key in Hierarchy without selecting the row or toggling its subtasks", async () => {
      await renderList(modeData("hierarchy"));

      await userEvent.click(within(await screen.findByRole("row", { name: /NW-1042/ })).getByText("NW-1042"));

      expect(mocked.copyText).toHaveBeenCalledWith("NW-1042");
      expect(rowOf(/NW-1042/)).toHaveAttribute("aria-selected", "false");
      expect(caretOf(/NW-1042/)).toHaveAccessibleName("Collapse subtasks");
      expect(screen.getByText("Draft the price table")).toBeInTheDocument();
    });

    describe("with several levels in Hierarchy", () => {
      const TOP = row({ id: "20", key: "NW-2001", summary: "Launch the help center", childCount: 2 });
      const MID = row({ id: "21", key: "NW-2002", summary: "Write the onboarding articles", parentId: "20", depth: 1, childCount: 1 });
      const LEAF = row({ id: "22", key: "NW-2003", summary: "Proofread the welcome article", parentId: "21", depth: 2, childCount: 1 });
      const DEEP = row({ id: "23", key: "NW-2004", summary: "Check the screenshots", parentId: "22", depth: 3 });
      const SIBLING = row({ id: "24", key: "NW-2005", summary: "Set up the search index", parentId: "20", depth: 1 });
      const NEXT = row({ id: "25", key: "NW-2006", summary: "Plan the support rota" });
      const tree = [TOP, MID, LEAF, DEEP, SIBLING, NEXT];
      const depthOf = (key: RegExp) => rowOf(key).style.getPropertyValue("--depth");
      const press = (id: string) => act(async () => void (await getCommand(id)?.run()));
      const selectedKeys = () =>
        screen
          .getAllByRole("row")
          .filter((r) => r.getAttribute("aria-selected") === "true")
          .map((r) => /NW-\d+/.exec(r.textContent ?? "")?.[0]);

      beforeEach(() => {
        installCommands("main");
      });

      it("indents every row by its depth", async () => {
        await renderList(modeData("hierarchy", tree));

        await screen.findByRole("row", { name: /NW-2001/ });
        expect([/NW-2001/, /NW-2002/, /NW-2003/, /NW-2004/, /NW-2005/, /NW-2006/].map(depthOf)).toEqual(["0", "1", "2", "3", "1", "0"]);
      });

      it("gives a caret to every row with children at any level, and none to the leaves", async () => {
        await renderList(modeData("hierarchy", tree));

        await screen.findByRole("row", { name: /NW-2001/ });
        for (const key of [/NW-2001/, /NW-2002/, /NW-2003/]) {
          expect(caretOf(key)).toHaveAccessibleName("Collapse subtasks");
        }
        for (const key of [/NW-2004/, /NW-2005/, /NW-2006/]) {
          expect(caretOf(key)).toBeNull();
        }
      });

      it("collapses only the subtree of a mid-level row and leaves its siblings and the rest", async () => {
        await renderList(modeData("hierarchy", tree));

        await screen.findByRole("row", { name: /NW-2002/ });
        await toggleCaretOf(/NW-2002/);

        expect(screen.queryByText("Proofread the welcome article")).not.toBeInTheDocument();
        expect(screen.queryByText("Check the screenshots")).not.toBeInTheDocument();
        expect(screen.getByText("Write the onboarding articles")).toBeInTheDocument();
        expect(screen.getByText("Set up the search index")).toBeInTheDocument();
        expect(screen.getByText("Plan the support rota")).toBeInTheDocument();
        expect(within(rowOf(/NW-2002/)).getByLabelText("1 child issue")).toBeInTheDocument();
      });

      it("moves the selection onto the collapsed row when it held the selected row", async () => {
        await renderList(modeData("hierarchy", tree));
        await userEvent.click(await screen.findByText("Check the screenshots"));

        await toggleCaretOf(/NW-2002/);

        expect(selectedKeys()).toEqual(["NW-2002"]);
      });

      it("collapses an open mid-level row on left and expands it on right", async () => {
        await renderList(modeData("hierarchy", tree));
        await userEvent.click(await screen.findByText("Write the onboarding articles"));

        await press("group.collapse");
        expect(screen.queryByText("Proofread the welcome article")).not.toBeInTheDocument();
        expect(screen.getByText("Set up the search index")).toBeInTheDocument();

        await press("group.expand");
        expect(await screen.findByText("Proofread the welcome article")).toBeInTheDocument();
      });

      it("selects the parent on left from a nested leaf", async () => {
        await renderList(modeData("hierarchy", tree));
        await userEvent.click(await screen.findByText("Check the screenshots"));

        await press("group.collapse");

        expect(selectedKeys()).toEqual(["NW-2003"]);
      });

      it("selects the parent on left from a mid-level row whose subtree is already collapsed", async () => {
        await renderList(modeData("hierarchy", tree));
        await userEvent.click(await screen.findByText("Write the onboarding articles"));
        await press("group.collapse");

        await press("group.collapse");

        expect(selectedKeys()).toEqual(["NW-2001"]);
      });

      it("walks the visible rows with next and previous, skipping a collapsed subtree", async () => {
        await renderList(modeData("hierarchy", tree));
        await screen.findByRole("row", { name: /NW-2002/ });
        await toggleCaretOf(/NW-2002/);
        await userEvent.click(screen.getByText("Write the onboarding articles"));

        await press("issue.next");
        expect(selectedKeys()).toEqual(["NW-2005"]);
        await press("issue.next");
        expect(selectedKeys()).toEqual(["NW-2006"]);
        await press("issue.previous");
        await press("issue.previous");
        expect(selectedKeys()).toEqual(["NW-2002"]);
      });
    });

    it("shows the subtask icon before the title of a subtask in every mode, and no indent in List", async () => {
      await renderList(modeData("list"));

      const step = await screen.findByRole("row", { name: /NW-1043/ });
      const icon = step.querySelector(".subtask-icon");
      expect(icon).not.toBeNull();
      expect(icon!.compareDocumentPosition(within(step).getByText("Draft the price table")) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(step).not.toHaveClass("list-row--child");
      expect(rowOf(/NW-1042/).querySelector(".subtask-icon")).toBeNull();
      expect(screen.queryByRole("button", { name: /subtasks$/ })).not.toBeInTheDocument();
    });

    it("collapses a parent's subtasks with the caret and leaves its count as a chip", async () => {
      await renderList(modeData("hierarchy"));

      await userEvent.click(await caret("Collapse subtasks"));

      expect(screen.queryByText("Draft the price table")).not.toBeInTheDocument();
      expect(screen.queryByText("Review the price table")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Expand subtasks" })).toHaveAttribute("aria-expanded", "false");
      expect(within(rowOf(/NW-1042/)).getByLabelText("2 child issues")).toBeInTheDocument();
      expect(rowOf(/NW-1042/)).toHaveAttribute("aria-selected", "false");
    });

    it("expands them again with the caret", async () => {
      await renderList(modeData("hierarchy"));
      await userEvent.click(await caret("Collapse subtasks"));

      await userEvent.click(await caret("Expand subtasks"));

      expect(await screen.findByText("Draft the price table")).toBeInTheDocument();
      expect(screen.getByText("Review the price table")).toBeInTheDocument();
      expect(screen.queryByLabelText("2 child issues")).not.toBeInTheDocument();
    });

    it("saves the collapsed parents in the view's place", async () => {
      await renderList(modeData("hierarchy"));

      await userEvent.click(await caret("Collapse subtasks"));

      expect(place().collapsedParents).toEqual(["1"]);
      await waitFor(() => expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, expect.objectContaining({ collapsedParents: ["1"] })));
    });

    it("restores a saved collapse", async () => {
      mocked.placeGet.mockResolvedValue({ scrollTop: 0, toggled: {}, collapsedParents: ["1"], selectedId: null, peekOpen: false });
      await renderList(modeData("hierarchy"));

      expect(await screen.findByRole("button", { name: "Expand subtasks" })).toBeInTheDocument();
      expect(screen.queryByText("Draft the price table")).not.toBeInTheDocument();
    });

    describe("keys", () => {
      const press = (id: string) => act(async () => void (await getCommand(id)?.run()));
      const selectedKeys = () =>
        screen
          .getAllByRole("row")
          .filter((r) => r.getAttribute("aria-selected") === "true")
          .map((r) => /NW-\d+/.exec(r.textContent ?? "")?.[0]);

      beforeEach(() => {
        installCommands("main");
      });

      it("walks into the subtask rows with next and back out with previous", async () => {
        await renderList(modeData("hierarchy"));
        await userEvent.click(await screen.findByText("Redesign the pricing page"));

        await press("issue.next");
        expect(selectedKeys()).toEqual(["NW-1043"]);
        await press("issue.next");
        expect(selectedKeys()).toEqual(["NW-1044"]);
        await press("issue.next");
        expect(selectedKeys()).toEqual(["NW-1001"]);
        await press("issue.previous");
        await press("issue.previous");
        expect(selectedKeys()).toEqual(["NW-1043"]);
      });

      it("skips the subtasks of a collapsed parent", async () => {
        await renderList(modeData("hierarchy"));
        await userEvent.click(await caret("Collapse subtasks"));
        await userEvent.click(screen.getByText("Redesign the pricing page"));

        await press("issue.next");

        expect(selectedKeys()).toEqual(["NW-1001"]);
      });

      it("selects the parent when the subtask is selected and left is pressed", async () => {
        await renderList(modeData("hierarchy"));
        await userEvent.click(await screen.findByText("Review the price table"));

        await press("group.collapse");

        expect(selectedKeys()).toEqual(["NW-1042"]);
        expect(screen.getByText("Review the price table")).toBeInTheDocument();
      });

      it("collapses the subtasks of the selected parent on left and expands them on right", async () => {
        await renderList(modeData("hierarchy"));
        await userEvent.click(await screen.findByText("Redesign the pricing page"));

        await press("group.collapse");
        expect(screen.queryByText("Draft the price table")).not.toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Collapse In Progress" })).toBeInTheDocument();

        await press("group.expand");
        expect(await screen.findByText("Draft the price table")).toBeInTheDocument();
      });

      it("closes the group on left once the parent's subtasks are already collapsed", async () => {
        const groups = [groupOf([PARENT, STEP, LAST_STEP], { key: "a", label: "Alpha" }), groupOf([OTHER], { key: "b", label: "Beta" })];
        await renderList(viewData(groups, [PARENT, STEP, LAST_STEP, OTHER], { subtasks: "hierarchy" }));
        await userEvent.click(await screen.findByText("Redesign the pricing page"));
        await press("group.collapse");

        await press("group.collapse");

        expect(screen.queryByText("Redesign the pricing page")).not.toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Expand Alpha" })).toBeInTheDocument();
      });

      it("closes the group on left from a subtask in List mode, where it has no parent to go to", async () => {
        await renderList(modeData("list"));
        await userEvent.click(await screen.findByText("Review the price table"));

        await press("group.collapse");

        expect(screen.queryByText("Review the price table")).not.toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Expand In Progress" })).toBeInTheDocument();
      });
    });
  });

  describe("the key in a row", () => {
    const keyOf = async (key: string) => within(await screen.findByRole("row", { name: new RegExp(key) })).getByText(key);

    it("copies the key and says so in a toast", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      await userEvent.click(await keyOf("NW-1042"));

      expect(mocked.copyText).toHaveBeenCalledWith("NW-1042");
      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Copied NW-1042"));
    });

    it("neither selects the row nor opens the preview or the Issue view when clicked", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]), "full");

      await userEvent.click(await keyOf("NW-1042"));

      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "false");
      expect(place()).toMatchObject({ selectedId: null, peekOpen: false });
      expect(useStore.getState().issueView).toBeNull();
    });

    it("opens no Issue view when double-clicked", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]), "preview");

      await userEvent.dblClick(await keyOf("NW-1042"));

      expect(useStore.getState().issueView).toBeNull();
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

    it("toggles the issue's bookmark without selecting the row", async () => {
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      await userEvent.click(await action("Bookmark NW-1042"));

      expect(mocked.pinToggleIssue).toHaveBeenCalledWith("1");
      expect(screen.getByRole("row", { name: /NW-1042/ })).toHaveAttribute("aria-selected", "false");
      expect(place().selectedId).toBeNull();
    });

    it("reads Remove the bookmark of an issue that is bookmarked", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ target: issueTarget({ id: "1", key: "NW-1042" }) })]));
      await renderList(viewData([groupOf([OPEN])], [OPEN]));

      expect(await action("Remove the bookmark of NW-1042")).toHaveAttribute("aria-pressed", "true");
      expect(screen.queryByRole("button", { name: "Bookmark NW-1042" })).not.toBeInTheDocument();
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
      await waitFor(() => expect(place().peekOpen).toBe(true), PREVIEW_WAIT);
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

    it("marks the selected unread issue read", async () => {
      const unread = row({ id: "3", key: "NW-1003", summary: "Fix the footer links", changed: true });
      const unreadRows = [OPEN, unread, LAST];
      await renderList(viewData([groupOf(unreadRows)], unreadRows));
      await userEvent.click(await screen.findByText("Fix the footer links"));

      await press("issue.markSeen");

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["3"]);
      expect(mocked.issueMarkUnread).not.toHaveBeenCalled();
    });

    it("marks the selected read issue unread", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await userEvent.click(await screen.findByText("Fix the footer links"));

      await press("issue.markSeen");

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("3");
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });

    it("marks nothing without a selected issue", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await screen.findByRole("row", { name: /NW-1042/ });

      await press("issue.markSeen");

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      expect(mocked.issueMarkUnread).not.toHaveBeenCalled();
    });

    it("closes the selected issue's peek and opens it again", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await userEvent.click(await screen.findByText("Fix the footer links"));
      await waitFor(() => expect(place()).toMatchObject({ selectedId: "3", peekOpen: true }), PREVIEW_WAIT);

      await press("issue.togglePeek");
      expect(place()).toMatchObject({ selectedId: "3", peekOpen: false });

      await press("issue.togglePeek");
      expect(place()).toMatchObject({ selectedId: "3", peekOpen: true });
    });

    it("opens no peek without a selected issue", async () => {
      await renderList(viewData([groupOf(rows)], rows));
      await screen.findByRole("row", { name: /NW-1042/ });

      await press("issue.togglePeek");

      expect(place().peekOpen).toBe(false);
      expect(place().selectedId).toBeNull();
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

    it("shows the status type's name as a pill when the view has the Status type column", async () => {
      const inReview = row({ id: "5", key: "NW-1005", summary: "Check the footer links", style: "review", statusName: "Code review" });
      mocked.columnsCatalogue.mockResolvedValue([...COLUMNS, { id: "statusType", name: "Status type", custom: false }]);
      await renderList(viewData([groupOf([inReview])], [inReview], { columns: ["statusType"], groupBy: "none" }));

      expect(await screen.findByRole("columnheader", { name: "Status type" })).toBeInTheDocument();
      const issue = screen.getByRole("row", { name: /NW-1005/ });
      expect(within(issue).getByText("In review")).toHaveClass("status-pill", "status-pill--review");
      expect(within(issue).queryByText("Code review")).not.toBeInTheDocument();
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

    describe("cells without a value", () => {
      const bare = row({ id: "6", key: "NW-1006", summary: "Update the footer" });
      const cellsOf = async (issue: Row) => {
        await renderList(viewData([groupOf([issue])], [issue], { columns: ["assignee", "epic", "priority"], groupBy: "none" }));
        return within(await screen.findByRole("row", { name: new RegExp(issue.key) })).getAllByRole("gridcell").slice(1);
      };

      it("shows a dash in the assignee, epic and priority cells of an issue without them", async () => {
        const cells = await cellsOf(bare);

        expect(cells.map((c) => c.textContent)).toEqual(["—", "—", "—"]);
      });

      it("shows the values and no dash when the issue has them", async () => {
        const cells = await cellsOf(withEpic);

        expect(within(cells[0]).getByRole("img", { name: "Alex Morgan" })).toBeInTheDocument();
        expect(cells[1]).toHaveTextContent("Launch Q4");
        expect(cells[2]).toHaveTextContent("High");
        expect(screen.queryByText("—")).not.toBeInTheDocument();
      });
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

  describe("virtualization", () => {
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
