import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ListConditions } from "../../api";
import { MenuHost } from "../../components/menu";
import { createQueryClient } from "../../queries/queryClient";
import { narrowingKey } from "../../state/slices/narrowing";
import { resetStore, useStore } from "../../state/store";
import { boardData } from "../../test/board";
import { filterOptions, listConditions } from "../../test/filters";
import { viewData } from "../../test/issues";
import { sidebarTree, space } from "../../test/spaces";
import { view } from "../../test/views";
import { FilterPopover } from "./FilterPopover";

const mocked = vi.mocked(api, true);

const OPTIONS = filterOptions({
  fields: [
    ...filterOptions().fields,
    { id: "customfield_10071", name: "Review notes", kind: "text" },
    { id: "issueKey", name: "Issue ID", kind: "issueKeys" },
    { id: "parentKey", name: "Parent ID", kind: "parentKeys" },
  ],
  statuses: [
    { id: "3", name: "In Progress" },
    { id: "4", name: "Done" },
  ],
  priorities: [
    { id: "High", name: "High" },
    { id: "Low", name: "Low" },
  ],
});

const VIEW_ID = 101;
const HIGH_PRIORITY = { field: "priority", op: "is", value: "High" } as const;
const LOW_PRIORITY = { field: "priority", op: "is", value: "Low" } as const;
const IN_PROGRESS = { field: "status", op: "is", value: "3" } as const;
const NO_DUE_DATE = { field: "due", op: "isEmpty" } as const;
const FILTER_LIST: ListConditions = listConditions({ kind: "list", own: [HIGH_PRIORITY] });
const SPACE: ListConditions = listConditions({ kind: "space", locked: [{ field: "Project", op: "=", value: "NW", source: "this Space" }] });

const onSave = vi.fn();

/** What the popover has drafted for the view; undefined while it changed nothing. */
const draft = (listId = 21) => useStore.getState().narrowings[narrowingKey(listId, VIEW_ID)];

function Harness({ listId, dirty }: { listId: number; dirty: boolean }) {
  const anchor = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  return (
    <>
      <button ref={anchor} type="button" onClick={() => setOpen((o) => !o)}>
        Filter
      </button>
      {open && <FilterPopover listId={listId} viewId={VIEW_ID} anchor={anchor} dirty={dirty} onSave={onSave} onClose={() => setOpen(false)} />}
      <MenuHost />
    </>
  );
}

async function openFilter(conditions: ListConditions, listId = 21, dirty = false) {
  mocked.listConditions.mockResolvedValue(conditions);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness listId={listId} dirty={dirty} />
    </QueryClientProvider>,
  );
  await userEvent.click(screen.getByRole("button", { name: "Filter" }));
  await screen.findByText("Show issues where");
  return screen.getByRole("dialog", { name: "Filter" });
}

const LIST_SECTION = "All views of this list";
const VIEW_SECTION = "This view";
const section = (popover: HTMLElement, name: string) => within(popover).getByRole("group", { name });

/** Opens a pop-up button's menu and chooses the row with the given name. */
async function pick(button: HTMLElement, row: string) {
  await userEvent.click(button);
  await userEvent.click(screen.getByRole("menuitemradio", { name: row }));
}

/** The rows a pop-up button's menu offers, in order; the menu is closed again afterwards. */
async function rowsOf(button: HTMLElement) {
  await userEvent.click(button);
  const rows = screen.getAllByRole("menuitemradio").map((row) => row.textContent);
  await userEvent.keyboard("{Escape}");
  return rows;
}

/** Unticks High and ticks Low in the open value picker. */
async function swapPriority(popover: HTMLElement) {
  await userEvent.click(within(popover).getByLabelText("Value"));
  await userEvent.click(screen.getByRole("option", { name: "High" }));
  await userEvent.click(screen.getByRole("option", { name: "Low" }));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.filterOptions.mockResolvedValue(OPTIONS);
  mocked.viewRows.mockResolvedValue(viewData([], [], { total: 5 }));
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
  mocked.viewsOf.mockResolvedValue([view({ id: VIEW_ID })]);
});

describe("FilterPopover", () => {
  describe("locked conditions", () => {
    it("shows the field, operator, value and source without editable controls", async () => {
      const popover = await openFilter(SPACE, 11);

      const locked = within(popover).getByText("Project").closest(".locked-condition") as HTMLElement;
      expect(locked).toHaveTextContent("Project");
      expect(within(locked).getByText("=")).toBeInTheDocument();
      expect(within(locked).getByText("NW")).toBeInTheDocument();
      expect(within(locked).getByText("this Space")).toBeInTheDocument();
      expect(within(popover).queryByLabelText("Field")).not.toBeInTheDocument();
      expect(within(popover).queryByLabelText("Operator")).not.toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Remove condition" })).not.toBeInTheDocument();
    });

    it("shows why the stored filter cannot be read", async () => {
      const popover = await openFilter(listConditions({ kind: "list", error: "The filter is damaged" }));

      expect(within(popover).getByText("The filter is damaged")).toBeInTheDocument();
    });
  });

  describe("on a filter list", () => {
    it("shows each own condition as field, operator and value controls", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(within(popover).getByLabelText("Field")).toHaveTextContent(/^Priority$/);
      expect(within(popover).getByLabelText("Operator")).toHaveTextContent(/^is$/);
      expect(within(popover).getByLabelText("Value")).toHaveTextContent("High");
    });

    it("shows a stored in condition as is, with all its values in the field", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "in", value: ["3", "4"] }] }));

      expect(within(popover).getByLabelText("Operator")).toHaveTextContent(/^is$/);
      expect(within(popover).getByLabelText("Value")).toHaveTextContent("In Progress, Done");
    });

    it("shows a stored not in condition as is not", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "notIn", value: ["3", "4"] }] }));

      expect(within(popover).getByLabelText("Operator")).toHaveTextContent(/^is not$/);
    });

    it("drafts one condition with a list when a second status is ticked for is not", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "isNot", value: "3" }] }));
      await userEvent.click(within(popover).getByLabelText("Value"));

      await userEvent.click(screen.getByRole("option", { name: "Done" }));

      expect(draft()?.list).toEqual([{ field: "status", op: "notIn", value: ["3", "4"] }]);
    });

    it("drafts is not with a single value again when the list is unticked back to one", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "notIn", value: ["3", "4"] }] }));
      await userEvent.click(within(popover).getByLabelText("Value"));

      await userEvent.click(screen.getByRole("option", { name: "Done" }));

      expect(draft()?.list).toEqual([{ field: "status", op: "isNot", value: "3" }]);
    });

    it("keeps the picked values when the operator switches between is and is not", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "in", value: ["3", "4"] }] }));

      await pick(within(popover).getByLabelText("Operator"), "is not");

      expect(within(popover).getByLabelText("Value")).toHaveTextContent("In Progress, Done");
      expect(draft()?.list).toEqual([{ field: "status", op: "notIn", value: ["3", "4"] }]);
    });

    it("keeps a single picked value when the operator switches to is not", async () => {
      const popover = await openFilter(FILTER_LIST);

      await pick(within(popover).getByLabelText("Operator"), "is not");

      expect(draft()?.list).toEqual([{ field: "priority", op: "isNot", value: "High" }]);
    });

    it("counts the matching issues in the header", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();
    });

    it("offers no Save as filter list", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(within(popover).queryByRole("button", { name: "Save as filter list" })).not.toBeInTheDocument();
    });

    it("drafts a changed value without saving it", async () => {
      const popover = await openFilter(FILTER_LIST);

      await swapPriority(popover);

      expect(draft()?.list).toEqual([LOW_PRIORITY]);
      expect(mocked.listSetFilter).not.toHaveBeenCalled();
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
    });

    it("shows a drafted condition in place of the saved one when the popover opens again", async () => {
      useStore.getState().setNarrowing(narrowingKey(21, VIEW_ID), { list: [LOW_PRIORITY] });

      const popover = await openFilter(FILTER_LIST);

      expect(within(popover).getByLabelText("Value")).toHaveTextContent("Low");
    });

    it("drafts nothing until a condition is edited", async () => {
      await openFilter(FILTER_LIST);

      expect(draft()).toBeUndefined();
    });

    it("adds a row with a ready-to-use condition to the draft", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.click(within(section(popover, LIST_SECTION)).getByRole("button", { name: "+ Add condition" }));

      expect(within(section(popover, LIST_SECTION)).getAllByLabelText("Field")).toHaveLength(2);
      expect(draft()?.list).toEqual([HIGH_PRIORITY, IN_PROGRESS]);
    });

    it("drafts the list without a condition that is removed", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.click(within(popover).getByRole("button", { name: "Remove condition" }));

      expect(within(popover).queryByLabelText("Field")).not.toBeInTheDocument();
      expect(draft()?.list).toEqual([]);
    });

    it("keeps a condition whose text is still empty out of the match count", async () => {
      mocked.viewRows.mockImplementation(async (_list, _view, narrowing) => viewData([], [], { total: narrowing.listDraft?.length ?? 1 }));
      const popover = await openFilter(FILTER_LIST);
      await userEvent.click(within(section(popover, LIST_SECTION)).getByRole("button", { name: "+ Add condition" }));
      expect(await within(popover).findByText("2 issues match")).toBeInTheDocument();

      await pick(within(section(popover, LIST_SECTION)).getAllByLabelText("Field")[1], "Review notes");

      expect(within(popover).getAllByLabelText("Value")[1]).toHaveValue("");
      expect(await within(popover).findByText("1 issue matches")).toBeInTheDocument();
    });

    it("drafts a text condition once it has text", async () => {
      const popover = await openFilter(FILTER_LIST);
      await pick(within(popover).getByLabelText("Field"), "Review notes");

      await userEvent.type(within(popover).getByLabelText("Value"), "pricing");

      expect(draft()?.list).toEqual([{ field: "customfield_10071", op: "contains", value: "pricing" }]);
    });

    it("keeps the value of a text condition when the operator changes between value operators", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "customfield_10071", op: "contains", value: "pricing" }] }));

      await pick(within(popover).getByLabelText("Operator"), "is");

      expect(within(popover).getByLabelText("Value")).toHaveValue("pricing");
    });

    it("hides the value input for an operator that needs none", async () => {
      const popover = await openFilter(FILTER_LIST);

      await pick(within(popover).getByLabelText("Operator"), "is empty");

      expect(within(popover).queryByLabelText("Value")).not.toBeInTheDocument();
      expect(draft()?.list).toEqual([{ field: "priority", op: "isEmpty" }]);
    });

    it("starts a days condition at 7", async () => {
      const popover = await openFilter(FILTER_LIST);

      await pick(within(popover).getByLabelText("Field"), "Updated");
      await pick(within(popover).getByLabelText("Operator"), "in last");

      expect(within(popover).getByLabelText("Value")).toHaveValue(7);
    });

    it("starts an hours condition at 7 for a timestamp field", async () => {
      const popover = await openFilter(FILTER_LIST);

      await pick(within(popover).getByLabelText("Field"), "Last view");
      await pick(within(popover).getByLabelText("Operator"), "in last (hours)");

      expect(within(popover).getByLabelText("Value")).toHaveValue(7);
      expect(draft()?.list).toEqual([{ field: "lastView", op: "withinLastHours", value: 7 }]);
    });

    it("offers the hours operator for Last interaction but not for a date field", async () => {
      const popover = await openFilter(FILTER_LIST);

      await pick(within(popover).getByLabelText("Field"), "Last interaction");
      expect(await rowsOf(within(popover).getByLabelText("Operator"))).toContain("in last (hours)");
      await pick(within(popover).getByLabelText("Field"), "Due date");

      expect(await rowsOf(within(popover).getByLabelText("Operator"))).not.toContain("in last (hours)");
    });

    it("takes a calendar date for a timestamp field's before operator", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "lastView", op: "before", value: "2026-10-01" }] }));

      expect(within(popover).getByLabelText("Value")).toHaveAttribute("type", "date");
      expect(within(popover).getByLabelText("Value")).toHaveValue("2026-10-01");
    });

    it("shows the total of the new rows answer once a condition is drafted", async () => {
      mocked.viewRows.mockImplementation(async (_list, _view, narrowing) => viewData([], [], { total: narrowing.listDraft?.[0]?.value === "Low" ? 3 : 5 }));
      const popover = await openFilter(FILTER_LIST);
      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();

      await swapPriority(popover);

      expect(await within(popover).findByText("3 issues match")).toBeInTheDocument();
    });
  });

  describe("a keys field", () => {
    const operatorNames = (popover: HTMLElement) => rowsOf(within(popover).getByLabelText("Operator"));

    async function pickField(name: string) {
      const popover = await openFilter(FILTER_LIST);
      await pick(within(popover).getByLabelText("Field"), name);
      return popover;
    }

    it("takes typed text for Issue ID and offers only is and is not", async () => {
      const popover = await pickField("Issue ID");

      expect(within(popover).getByLabelText("Value")).toHaveAttribute("placeholder", "NW-123, NW-124");
      expect(within(popover).getByLabelText("Value")).toHaveAttribute("type", "text");
      expect(await operatorNames(popover)).toEqual(["is", "is not"]);
    });

    it("offers is empty and is set for Parent ID besides is and is not", async () => {
      const popover = await pickField("Parent ID");

      expect(await operatorNames(popover)).toEqual(["is", "is not", "is empty", "is set"]);
    });

    it("drafts several typed keys as in, and shows them normalized, on Enter", async () => {
      mocked.filterParseKeys.mockResolvedValue(["NW-1", "NW-2"]);
      const popover = await pickField("Issue ID");

      await userEvent.type(within(popover).getByLabelText("Value"), "nw-1, nw-2{Enter}");

      expect(mocked.filterParseKeys).toHaveBeenCalledWith("nw-1, nw-2");
      await waitFor(() => expect(draft()?.list).toEqual([{ field: "issueKey", op: "in", value: ["NW-1", "NW-2"] }]));
      expect(within(popover).getByLabelText("Value")).toHaveValue("NW-1, NW-2");
    });

    it("drafts one typed key as is with a single value when the field loses focus", async () => {
      mocked.filterParseKeys.mockResolvedValue(["NW-1"]);
      const popover = await pickField("Issue ID");

      await userEvent.type(within(popover).getByLabelText("Value"), "nw-1");
      await userEvent.tab();

      await waitFor(() => expect(draft()?.list).toEqual([{ field: "issueKey", op: "is", value: "NW-1" }]));
    });

    it("keeps is not when new keys are typed into a negated condition", async () => {
      mocked.filterParseKeys.mockResolvedValue(["NW-3", "NW-4"]);
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "parentKey", op: "isNot", value: "NW-1" }] }));
      const input = within(popover).getByLabelText("Value");
      expect(input).toHaveValue("NW-1");

      await userEvent.clear(input);
      await userEvent.type(input, "NW-3, NW-4{Enter}");

      await waitFor(() => expect(draft()?.list).toEqual([{ field: "parentKey", op: "notIn", value: ["NW-3", "NW-4"] }]));
    });

    it("shows the stored keys of an in condition joined by commas", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "issueKey", op: "in", value: ["NW-1", "NW-2"] }] }));

      expect(within(popover).getByLabelText("Value")).toHaveValue("NW-1, NW-2");
    });

    it("shows why the keys are refused and keeps the condition", async () => {
      mocked.filterParseKeys.mockRejectedValue({ kind: "invalid_input", message: "“nw 1” is not an issue key" });
      const popover = await pickField("Issue ID");

      await userEvent.type(within(popover).getByLabelText("Value"), "nw 1{Enter}");

      expect(await within(popover).findByText("“nw 1” is not an issue key")).toBeInTheDocument();
      expect(within(popover).getByLabelText("Value")).toHaveValue("nw 1");
      expect(draft()?.list ?? []).not.toContainEqual(expect.objectContaining({ op: "in" }));
    });

    it("clears the message when the field changes", async () => {
      mocked.filterParseKeys.mockRejectedValue({ kind: "invalid_input", message: "“nw 1” is not an issue key" });
      const popover = await pickField("Issue ID");
      await userEvent.type(within(popover).getByLabelText("Value"), "nw 1{Enter}");
      await within(popover).findByText("“nw 1” is not an issue key");

      await pick(within(popover).getByLabelText("Field"), "Parent ID");

      expect(within(popover).queryByText("“nw 1” is not an issue key")).not.toBeInTheDocument();
    });
  });

  describe("footer", () => {
    it("says changes show until the view is left while nothing is unsaved", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(within(popover).getByText("Changes show until you leave this view.")).toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Save View" })).not.toBeInTheDocument();
    });

    it("says what is unsaved and offers Save View while the view has changes", async () => {
      const popover = await openFilter(FILTER_LIST, 21, true);

      expect(within(popover).getByText(/^Not saved:/)).toBeInTheDocument();
      expect(within(popover).getByText(/keeps it\.$/)).toBeInTheDocument();
      expect(within(popover).queryByText("Changes show until you leave this view.")).not.toBeInTheDocument();
    });

    it("saves the view from its Save View link", async () => {
      const popover = await openFilter(FILTER_LIST, 21, true);

      await userEvent.click(within(popover).getByRole("button", { name: "Save View" }));

      expect(onSave).toHaveBeenCalledOnce();
    });
  });

  describe("on a Space", () => {
    it("shows the view's total as plain issues while nothing narrows", async () => {
      mocked.viewRows.mockResolvedValue(viewData([], [], { total: 12 }));
      const popover = await openFilter(SPACE, 11);

      expect(await within(popover).findByText("12 issues")).toBeInTheDocument();
    });

    it("says a single issue in the singular", async () => {
      mocked.viewRows.mockResolvedValue(viewData([], [], { total: 1 }));
      const popover = await openFilter(SPACE, 11);

      expect(await within(popover).findByText("1 issue")).toBeInTheDocument();
    });

    it("says a single matching issue in the singular once narrowed", async () => {
      mocked.viewRows.mockResolvedValue(viewData([], [], { total: 1 }));
      const popover = await openFilter(SPACE, 11);

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      expect(await within(popover).findByText("1 issue matches")).toBeInTheDocument();
    });

    it("shows the board's total on a board view", async () => {
      mocked.viewsOf.mockResolvedValue([view({ id: VIEW_ID, layout: "board" })]);
      mocked.viewBoard.mockResolvedValue(boardData([], [], { total: 7 }));
      const popover = await openFilter(SPACE, 11);

      expect(await within(popover).findByText("7 issues")).toBeInTheDocument();
    });

    it("shows no count until the rows load", async () => {
      mocked.viewRows.mockReturnValue(new Promise(() => {}));
      const popover = await openFilter(SPACE, 11);

      await waitFor(() => expect(mocked.viewRows).toHaveBeenCalled());
      expect(within(popover).queryByText(/\d issues?/)).not.toBeInTheDocument();
    });

    it("counts the matching issues once a condition is added", async () => {
      const popover = await openFilter(SPACE, 11);

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();
    });

    it("keeps Save as filter list disabled until a condition is added", async () => {
      const popover = await openFilter(SPACE, 11);
      expect(within(popover).getByRole("button", { name: "Save as filter list" })).toBeDisabled();

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      expect(within(popover).getByRole("button", { name: "Save as filter list" })).toBeEnabled();
    });

    it("drafts the condition on the view and saves neither the view nor the Space", async () => {
      const popover = await openFilter(SPACE, 11);

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      expect(draft(11)).toMatchObject({ view: [IN_PROGRESS], list: null });
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
      expect(mocked.listSetFilter).not.toHaveBeenCalled();
    });

    it("creates a filter list in the Space from the draft and opens it", async () => {
      mocked.listCreate.mockResolvedValue(77);
      const popover = await openFilter(SPACE, 11);
      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      await userEvent.click(within(popover).getByRole("button", { name: "Save as filter list" }));

      expect(mocked.listCreate).toHaveBeenCalledWith(11, "New filter list", { conditions: [IN_PROGRESS] });
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(77, null));
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("creates the filter list under Everything when drafted there", async () => {
      mocked.listCreate.mockResolvedValue(78);
      mocked.sidebarTree.mockResolvedValue({ ...sidebarTree([space()]), everything: { ...sidebarTree([]).everything, id: 1 } });
      const popover = await openFilter(listConditions({ kind: "everything" }), 1);
      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      await userEvent.click(within(popover).getByRole("button", { name: "Save as filter list" }));

      expect(mocked.listCreate).toHaveBeenCalledWith(1, "New filter list", { conditions: [IN_PROGRESS] });
    });

    it("keeps the popover open and shows a toast when the list cannot be created", async () => {
      mocked.listCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const popover = await openFilter(SPACE, 11);
      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      await userEvent.click(within(popover).getByRole("button", { name: "Save as filter list" }));

      await waitFor(() => expect(mocked.listCreate).toHaveBeenCalled());
      expect(screen.getByRole("dialog", { name: "Filter" })).toBeInTheDocument();
      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });

  describe("the Me chip", () => {
    const savedOnlyMine = (onlyMine: boolean, listId = 11) => mocked.viewsOf.mockResolvedValue([view({ id: VIEW_ID, listId, onlyMine })]);

    it("counts with Me on and says issues match, even without conditions, for a view that saved Me", async () => {
      savedOnlyMine(true);
      const popover = await openFilter(SPACE, 11);

      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();
      expect(within(popover).queryByText("5 issues")).not.toBeInTheDocument();
    });

    it("says plain issues for a view that did not save Me", async () => {
      const popover = await openFilter(SPACE, 11);

      expect(await within(popover).findByText("5 issues")).toBeInTheDocument();
    });

    it.each([
      ["on over a view that saved Me off", true, false, "4 issues match"],
      ["off over a view that saved Me on", false, true, "5 issues"],
    ])("shows the total of the rows asked with the drafted Me chip %s", async (_name, drafted, saved, expected) => {
      mocked.viewRows.mockImplementation(async (_list, _view, narrowing) => viewData([], [], { total: narrowing.me ? 4 : 5 }));
      savedOnlyMine(saved);
      useStore.getState().setNarrowing(narrowingKey(11, VIEW_ID), { me: drafted });

      const popover = await openFilter(SPACE, 11);

      expect(await within(popover).findByText(expected)).toBeInTheDocument();
    });

    it("shows the total of the rows asked with Me together with the drafted conditions", async () => {
      mocked.viewRows.mockImplementation(async (_list, _view, narrowing) => viewData([], [], { total: narrowing.me ? 2 : 5 }));
      savedOnlyMine(true, 21);
      const popover = await openFilter(FILTER_LIST);

      expect(await within(popover).findByText("2 issues match")).toBeInTheDocument();
    });
  });

  describe("the Project field", () => {
    const fieldNames = (popover: HTMLElement) => rowsOf(within(popover).getByLabelText("Field"));

    it("is offered on a global filter list", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(await fieldNames(popover)).toContain("Project");
    });

    it("is not offered in a condition of a filter list inside a Space", async () => {
      const popover = await openFilter({ ...FILTER_LIST, projectFixed: true });

      const names = await fieldNames(popover);
      expect(names).toContain("Priority");
      expect(names).not.toContain("Project");
    });

    it("is not the field of a condition added inside a Space", async () => {
      const popover = await openFilter(listConditions({ kind: "list", projectFixed: true }));

      await userEvent.click(within(section(popover, LIST_SECTION)).getByRole("button", { name: "+ Add condition" }));

      expect(within(popover).getByLabelText("Field")).toHaveTextContent(/^Status$/);
      expect(await fieldNames(popover)).not.toContain("Project");
    });
  });

  describe("a field the Space hides", () => {
    const REVIEW_NOTES = { id: "customfield_10071", name: "Review notes", kind: "text" } as const;
    const WITH_HIDDEN = filterOptions({ hiddenFields: [REVIEW_NOTES] });
    beforeEach(() => {
      mocked.filterOptions.mockResolvedValue(WITH_HIDDEN);
    });

    it("is not offered in the field menu of a new condition", async () => {
      const popover = await openFilter(FILTER_LIST);
      await userEvent.click(within(section(popover, LIST_SECTION)).getByRole("button", { name: "+ Add condition" }));

      for (const button of within(popover).getAllByLabelText("Field")) expect(await rowsOf(button)).not.toContain("Review notes");
    });

    it("shows its name selected in a list condition that already uses it", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: REVIEW_NOTES.id, op: "contains", value: "pricing" }] }));

      expect(within(section(popover, LIST_SECTION)).getByLabelText("Field")).toHaveTextContent("Review notes");
      expect(within(popover).getByLabelText("Value")).toHaveValue("pricing");
    });

    it("shows its name selected in a view condition that already uses it, and offers it in no other row", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [HIGH_PRIORITY], view: [{ field: REVIEW_NOTES.id, op: "contains", value: "pricing" }] }));

      const viewField = within(section(popover, VIEW_SECTION)).getByLabelText("Field");
      expect(viewField).toHaveTextContent("Review notes");
      expect(await rowsOf(viewField)).toContain("Review notes");
      expect(await rowsOf(within(section(popover, LIST_SECTION)).getByLabelText("Field"))).not.toContain("Review notes");
    });
  });

  describe("the options it asks for", () => {
    it("are those of its own level", async () => {
      mocked.filterOptions.mockImplementation(async (listId) =>
        filterOptions({ fields: [{ id: "status", name: listId === 21 ? "Status of 21" : "Status of other", kind: "status" }] }),
      );

      const popover = await openFilter(listConditions({ kind: "list", own: [IN_PROGRESS] }), 21);

      expect(within(popover).getByLabelText("Field")).toHaveTextContent("Status of 21");
    });
  });

  describe("sections", () => {
    it("shows both All views of this list and This view on a filter list, each with its own conditions", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [HIGH_PRIORITY], view: [NO_DUE_DATE] }));

      const list = section(popover, LIST_SECTION);
      const own = section(popover, VIEW_SECTION);
      expect(within(list).getByLabelText("Field")).toHaveTextContent("Priority");
      expect(within(own).getByLabelText("Field")).toHaveTextContent("Due date");
      expect(within(list).getByRole("button", { name: "+ Add condition" })).toBeInTheDocument();
      expect(within(own).getByRole("button", { name: "+ Add condition" })).toBeInTheDocument();
    });

    it("drafts a condition added in This view for the view and not the list", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.click(within(section(popover, VIEW_SECTION)).getByRole("button", { name: "+ Add condition" }));

      expect(draft()).toMatchObject({ view: [IN_PROGRESS], list: null });
    });

    it("drafts an edited view condition for the view and not the list", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [HIGH_PRIORITY], view: [NO_DUE_DATE] }));

      await pick(within(section(popover, VIEW_SECTION)).getByLabelText("Operator"), "is set");

      expect(draft()).toMatchObject({ view: [{ field: "due", op: "isNotEmpty" }], list: null });
    });

    it("drafts a condition added in All views of this list for the list and not the view", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.click(within(section(popover, LIST_SECTION)).getByRole("button", { name: "+ Add condition" }));

      expect(draft()).toMatchObject({ list: [HIGH_PRIORITY, IN_PROGRESS], view: null });
    });

    it("drafts the view without a view condition that is removed", async () => {
      const popover = await openFilter(listConditions({ kind: "list", view: [NO_DUE_DATE] }));

      await userEvent.click(within(section(popover, VIEW_SECTION)).getByRole("button", { name: "Remove condition" }));

      expect(draft()?.view).toEqual([]);
    });

    it("shows the total of the rows asked once a condition is added in This view", async () => {
      mocked.viewRows.mockImplementation(async (_list, _view, narrowing) => viewData([], [], { total: narrowing.viewDraft?.length ? 8 : 5 }));
      const popover = await openFilter(FILTER_LIST);
      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();

      await userEvent.click(within(section(popover, VIEW_SECTION)).getByRole("button", { name: "+ Add condition" }));

      expect(await within(popover).findByText("8 issues match")).toBeInTheDocument();
    });

    it("shows a damaged view filter in This view and not in the list section", async () => {
      const popover = await openFilter(listConditions({ kind: "list", viewError: "The view filter is damaged" }));

      expect(within(section(popover, VIEW_SECTION)).getByText("The view filter is damaged")).toBeInTheDocument();
      expect(within(section(popover, LIST_SECTION)).queryByText("The view filter is damaged")).not.toBeInTheDocument();
    });

    it("shows a damaged list filter in the list section", async () => {
      const popover = await openFilter(listConditions({ kind: "list", error: "The filter is damaged" }));

      expect(within(section(popover, LIST_SECTION)).getByText("The filter is damaged")).toBeInTheDocument();
    });

    it("shows a damaged filter above the sections on a level without a list section", async () => {
      const popover = await openFilter(listConditions({ kind: "space", error: "The filter is damaged" }));

      expect(within(popover).getByText("The filter is damaged")).toBeInTheDocument();
      expect(within(section(popover, VIEW_SECTION)).queryByText("The filter is damaged")).not.toBeInTheDocument();
    });
  });

  describe("built-in levels", () => {
    it.each<ListConditions["kind"]>(["everything", "space", "pinned"])(
      "offers only This view on a %s level and drafts its conditions for the view",
      async (kind) => {
        const popover = await openFilter(listConditions({ kind, view: [NO_DUE_DATE] }));

        expect(within(popover).queryByRole("group", { name: LIST_SECTION })).not.toBeInTheDocument();
        expect(within(popover).getAllByRole("button", { name: "+ Add condition" })).toHaveLength(1);
        expect(within(section(popover, VIEW_SECTION)).getByLabelText("Field")).toHaveTextContent("Due date");

        await userEvent.click(within(popover).getByRole("button", { name: "Remove condition" }));

        expect(draft()?.view).toEqual([]);
        expect(mocked.viewSetFilter).not.toHaveBeenCalled();
        expect(mocked.listSetFilter).not.toHaveBeenCalled();
      },
    );

    it("offers no Save as filter list on a pinned level", async () => {
      const popover = await openFilter(listConditions({ kind: "pinned" }));

      expect(within(popover).queryByRole("button", { name: "Save as filter list" })).not.toBeInTheDocument();
    });
  });

  describe("on Recently viewed", () => {
    const WINDOW = { field: "lastView", op: "withinLastHours", value: 24 } as const;
    const VIEWED = listConditions({ kind: "viewed", fixed: WINDOW });
    const THIS_PAGE = "This page";

    it("shows This page before This view, with the fixed condition", async () => {
      const popover = await openFilter(VIEWED);

      const page = section(popover, THIS_PAGE);
      expect(page.compareDocumentPosition(section(popover, VIEW_SECTION)) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(within(page).getByLabelText("Value")).toHaveValue(24);
    });

    it("fixes the field and offers no way to remove the condition", async () => {
      const popover = await openFilter(VIEWED);

      const page = section(popover, THIS_PAGE);
      expect(within(page).getByLabelText("Field")).toHaveTextContent("Last view");
      expect(within(page).getByLabelText("Field")).toBeDisabled();
      expect(within(page).queryByRole("button", { name: "Remove condition" })).not.toBeInTheDocument();
      expect(within(page).queryByRole("button", { name: "+ Add condition" })).not.toBeInTheDocument();
    });

    it("offers only the hours and days operators", async () => {
      const popover = await openFilter(VIEWED);

      expect(await rowsOf(within(section(popover, THIS_PAGE)).getByLabelText("Operator"))).toEqual(["in last (hours)", "in last"]);
    });

    it("keeps the other sections editable and shows no list section", async () => {
      const popover = await openFilter(listConditions({ kind: "viewed", fixed: WINDOW, view: [NO_DUE_DATE] }));

      expect(within(popover).queryByRole("group", { name: LIST_SECTION })).not.toBeInTheDocument();
      expect(within(section(popover, VIEW_SECTION)).getByRole("button", { name: "Remove condition" })).toBeEnabled();
      expect(within(section(popover, VIEW_SECTION)).getByLabelText("Field")).toBeEnabled();
    });

    it("saves the new window at once when the count changes", async () => {
      const popover = await openFilter(VIEWED);
      const value = within(section(popover, THIS_PAGE)).getByLabelText("Value");

      await userEvent.clear(value);
      await userEvent.type(value, "36");

      expect(mocked.listSetWindow).toHaveBeenLastCalledWith(21, "withinLastHours", 36);
      expect(draft()).toBeUndefined();
    });

    it("keeps the span when hours switch to days: 24 hours become 1 day", async () => {
      const popover = await openFilter(VIEWED);

      await pick(within(section(popover, THIS_PAGE)).getByLabelText("Operator"), "in last");

      expect(mocked.listSetWindow).toHaveBeenCalledWith(21, "withinLastDays", 1);
      expect(within(section(popover, THIS_PAGE)).getByLabelText("Operator")).toHaveTextContent(/^in last$/);
    });

    it("keeps the span when days switch to hours: 2 days become 48 hours", async () => {
      const popover = await openFilter(listConditions({ kind: "viewed", fixed: { field: "lastView", op: "withinLastDays", value: 2 } }));

      await pick(within(section(popover, THIS_PAGE)).getByLabelText("Operator"), "in last (hours)");

      expect(mocked.listSetWindow).toHaveBeenCalledWith(21, "withinLastHours", 48);
      expect(within(section(popover, THIS_PAGE)).getByLabelText("Value")).toHaveValue(48);
    });

    it("saves nothing while the count is empty", async () => {
      const popover = await openFilter(VIEWED);
      const value = within(section(popover, THIS_PAGE)).getByLabelText("Value");

      await userEvent.clear(value);

      expect(mocked.listSetWindow).not.toHaveBeenCalled();
    });

    it("saves nothing for a fractional count", async () => {
      const popover = await openFilter(VIEWED);
      const value = within(section(popover, THIS_PAGE)).getByLabelText("Value");

      await userEvent.clear(value);
      await userEvent.type(value, "1.5");

      expect(mocked.listSetWindow).not.toHaveBeenLastCalledWith(21, "withinLastHours", 1.5);
    });

    it("keeps the popover open when the window cannot be saved", async () => {
      mocked.listSetWindow.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const popover = await openFilter(VIEWED);

      await userEvent.type(within(section(popover, THIS_PAGE)).getByLabelText("Value"), "0");

      await waitFor(() => expect(mocked.listSetWindow).toHaveBeenCalled());
      expect(screen.getByRole("dialog", { name: "Filter" })).toBeInTheDocument();
    });

    it("shows the view's total and says the issues match, because of the fixed condition", async () => {
      const popover = await openFilter(VIEWED);

      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();
    });
  });

  it("closes from Done", async () => {
    const popover = await openFilter(FILTER_LIST);

    await userEvent.click(within(popover).getByRole("button", { name: "Done" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("keeps the drafts when it closes", async () => {
    const popover = await openFilter(FILTER_LIST);
    await swapPriority(popover);

    await userEvent.click(within(popover).getByRole("button", { name: "Done" }));

    expect(draft()?.list).toEqual([LOW_PRIORITY]);
  });
});
