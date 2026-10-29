import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ListConditions } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { narrowingKey } from "../../state/slices/narrowing";
import { resetStore, useStore } from "../../state/store";
import { filterOptions, listConditions } from "../../test/filters";
import { sidebarTree, space } from "../../test/spaces";
import { FilterPopover } from "./FilterPopover";

const mocked = vi.mocked(api, true);

const OPTIONS = filterOptions({
  fields: [...filterOptions().fields, { id: "customfield_10071", name: "Review notes", kind: "text" }],
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
  mocked.listMatchCount.mockResolvedValue({ open: 5, total: 9 });
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
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
      expect(within(popover).queryByRole("combobox")).not.toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Remove condition" })).not.toBeInTheDocument();
    });

    it("shows why the stored filter cannot be read", async () => {
      const popover = await openFilter(listConditions({ kind: "list", error: "The filter is damaged" }));

      expect(within(popover).getByText("The filter is damaged")).toBeInTheDocument();
    });
  });

  describe("on a filter list", () => {
    it("shows each own condition as field, operator and value selects", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(within(popover).getByLabelText("Field")).toHaveDisplayValue("Priority");
      expect(within(popover).getByLabelText("Operator")).toHaveDisplayValue("is");
      expect(within(popover).getByLabelText("Value")).toHaveTextContent("High");
    });

    it("shows a stored in condition as is, with all its values in the field", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "in", value: ["3", "4"] }] }));

      expect(within(popover).getByLabelText("Operator")).toHaveDisplayValue("is");
      expect(within(popover).getByLabelText("Value")).toHaveTextContent("In Progress, Done");
    });

    it("shows a stored not in condition as is not", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "notIn", value: ["3", "4"] }] }));

      expect(within(popover).getByLabelText("Operator")).toHaveDisplayValue("is not");
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

      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "is not");

      expect(within(popover).getByLabelText("Value")).toHaveTextContent("In Progress, Done");
      expect(draft()?.list).toEqual([{ field: "status", op: "notIn", value: ["3", "4"] }]);
    });

    it("keeps a single picked value when the operator switches to is not", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "is not");

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
      mocked.listMatchCount.mockImplementation(async (_listId, { conditions }) => ({ open: conditions.length, total: 9 }));
      const popover = await openFilter(FILTER_LIST);
      await userEvent.click(within(section(popover, LIST_SECTION)).getByRole("button", { name: "+ Add condition" }));
      expect(await within(popover).findByText("2 issues match")).toBeInTheDocument();

      await userEvent.selectOptions(within(section(popover, LIST_SECTION)).getAllByLabelText("Field")[1], "Review notes");

      expect(within(popover).getAllByLabelText("Value")[1]).toHaveValue("");
      expect(await within(popover).findByText("1 issue matches")).toBeInTheDocument();
    });

    it("drafts a text condition once it has text", async () => {
      const popover = await openFilter(FILTER_LIST);
      await userEvent.selectOptions(within(popover).getByLabelText("Field"), "Review notes");

      await userEvent.type(within(popover).getByLabelText("Value"), "pricing");

      expect(draft()?.list).toEqual([{ field: "customfield_10071", op: "contains", value: "pricing" }]);
    });

    it("keeps the value of a text condition when the operator changes between value operators", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "customfield_10071", op: "contains", value: "pricing" }] }));

      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "is");

      expect(within(popover).getByLabelText("Value")).toHaveValue("pricing");
    });

    it("hides the value input for an operator that needs none", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "is empty");

      expect(within(popover).queryByLabelText("Value")).not.toBeInTheDocument();
      expect(draft()?.list).toEqual([{ field: "priority", op: "isEmpty" }]);
    });

    it("starts a days condition at 7", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.selectOptions(within(popover).getByLabelText("Field"), "Updated");
      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "in last");

      expect(within(popover).getByLabelText("Value")).toHaveValue(7);
    });

    it("counts the drafted conditions instead of the saved ones", async () => {
      const popover = await openFilter(FILTER_LIST);
      await waitFor(() => expect(mocked.listMatchCount).toHaveBeenLastCalledWith(21, { conditions: [HIGH_PRIORITY] }));

      await swapPriority(popover);

      await waitFor(() => expect(mocked.listMatchCount).toHaveBeenLastCalledWith(21, { conditions: [LOW_PRIORITY] }));
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
    it("counts the open issues before any condition is added", async () => {
      const popover = await openFilter(SPACE, 11);

      expect(await within(popover).findByText("5 open issues")).toBeInTheDocument();
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

  describe("the Project field", () => {
    const fieldNames = (popover: HTMLElement) => within(within(popover).getByLabelText("Field")).getAllByRole("option").map((o) => o.textContent);

    it("is offered on a global filter list", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(fieldNames(popover)).toContain("Project");
    });

    it("is not offered in a condition of a filter list inside a Space", async () => {
      const popover = await openFilter({ ...FILTER_LIST, projectFixed: true });

      expect(fieldNames(popover)).toContain("Priority");
      expect(fieldNames(popover)).not.toContain("Project");
    });

    it("is not the field of a condition added inside a Space", async () => {
      const popover = await openFilter(listConditions({ kind: "list", projectFixed: true }));

      await userEvent.click(within(section(popover, LIST_SECTION)).getByRole("button", { name: "+ Add condition" }));

      expect(within(popover).getByLabelText("Field")).toHaveDisplayValue("Status");
      expect(fieldNames(popover)).not.toContain("Project");
    });
  });

  describe("sections", () => {
    it("shows both All views of this list and This view on a filter list, each with its own conditions", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [HIGH_PRIORITY], view: [NO_DUE_DATE] }));

      const list = section(popover, LIST_SECTION);
      const own = section(popover, VIEW_SECTION);
      expect(within(list).getByLabelText("Field")).toHaveDisplayValue("Priority");
      expect(within(own).getByLabelText("Field")).toHaveDisplayValue("Due date");
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

      await userEvent.selectOptions(within(section(popover, VIEW_SECTION)).getByLabelText("Operator"), "is set");

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

    it("counts the issues matching the list and view conditions together", async () => {
      await openFilter(listConditions({ kind: "list", own: [HIGH_PRIORITY], view: [NO_DUE_DATE] }));

      await waitFor(() => expect(mocked.listMatchCount).toHaveBeenCalledWith(21, { conditions: [HIGH_PRIORITY, NO_DUE_DATE] }));
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
    it.each<ListConditions["kind"]>(["everything", "space", "recent", "pinned"])(
      "offers only This view on a %s level and drafts its conditions for the view",
      async (kind) => {
        const popover = await openFilter(listConditions({ kind, view: [NO_DUE_DATE] }));

        expect(within(popover).queryByRole("group", { name: LIST_SECTION })).not.toBeInTheDocument();
        expect(within(popover).getAllByRole("button", { name: "+ Add condition" })).toHaveLength(1);
        expect(within(section(popover, VIEW_SECTION)).getByLabelText("Field")).toHaveDisplayValue("Due date");

        await userEvent.click(within(popover).getByRole("button", { name: "Remove condition" }));

        expect(draft()?.view).toEqual([]);
        expect(mocked.viewSetFilter).not.toHaveBeenCalled();
        expect(mocked.listSetFilter).not.toHaveBeenCalled();
      },
    );

    it.each<ListConditions["kind"]>(["recent", "pinned"])("offers no Save as filter list on a %s level", async (kind) => {
      const popover = await openFilter(listConditions({ kind }));

      expect(within(popover).queryByRole("button", { name: "Save as filter list" })).not.toBeInTheDocument();
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
