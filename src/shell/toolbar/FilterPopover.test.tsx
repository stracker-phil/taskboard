import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ListConditions } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
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

const HIGH_PRIORITY = { field: "priority", op: "is", value: "High" } as const;
const FILTER_LIST: ListConditions = listConditions({ kind: "list", own: [HIGH_PRIORITY] });
const SPACE: ListConditions = listConditions({ kind: "space", locked: [{ field: "Project", op: "=", value: "NW", source: "this Space" }] });

function Harness({ listId, name }: { listId: number; name: string }) {
  const anchor = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  return (
    <>
      <button ref={anchor} type="button" onClick={() => setOpen((o) => !o)}>
        Filter
      </button>
      {open && <FilterPopover listId={listId} listName={name} anchor={anchor} onClose={() => setOpen(false)} />}
    </>
  );
}

async function openFilter(conditions: ListConditions, listId = 21, name = "My open issues") {
  mocked.listConditions.mockResolvedValue(conditions);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness listId={listId} name={name} />
    </QueryClientProvider>,
  );
  await userEvent.click(screen.getByRole("button", { name: "Filter" }));
  await screen.findByText("Show issues where");
  return screen.getByRole("dialog", { name: "Filter" });
}

/** Unticks High and ticks Low in the open value picker. */
async function swapPriority(popover: HTMLElement) {
  await userEvent.click(within(popover).getByLabelText("Value"));
  await userEvent.click(screen.getByRole("option", { name: "High" }));
  await userEvent.click(screen.getByRole("option", { name: "Low" }));
}

const wait =(ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.filterOptions.mockResolvedValue(OPTIONS);
  mocked.listMatchCount.mockResolvedValue({ open: 5, total: 9 });
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
  mocked.listSetFilter.mockResolvedValue(undefined);
});

describe("FilterPopover", () => {
  describe("locked conditions", () => {
    it("shows the field, operator, value and source without editable controls", async () => {
      const popover = await openFilter(SPACE, 11, "Website relaunch");

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

    it("saves one condition with a list when a second status is ticked for is not", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "isNot", value: "3" }] }));
      await userEvent.click(within(popover).getByLabelText("Value"));

      await userEvent.click(screen.getByRole("option", { name: "Done" }));

      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenLastCalledWith(21, { conditions: [{ field: "status", op: "notIn", value: ["3", "4"] }] }));
    });

    it("saves is not with a single value again when the list is unticked back to one", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "notIn", value: ["3", "4"] }] }));
      await userEvent.click(within(popover).getByLabelText("Value"));

      await userEvent.click(screen.getByRole("option", { name: "Done" }));

      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenLastCalledWith(21, { conditions: [{ field: "status", op: "isNot", value: "3" }] }));
    });

    it("keeps the picked values when the operator switches between is and is not", async () => {
      const popover = await openFilter(listConditions({ kind: "list", own: [{ field: "status", op: "in", value: ["3", "4"] }] }));

      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "is not");

      expect(within(popover).getByLabelText("Value")).toHaveTextContent("In Progress, Done");
      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenLastCalledWith(21, { conditions: [{ field: "status", op: "notIn", value: ["3", "4"] }] }));
    });

    it("keeps a single picked value when the operator switches to is not", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "is not");

      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenLastCalledWith(21, { conditions: [{ field: "priority", op: "isNot", value: "High" }] }));
    });

    it("counts the matching issues in the header and names the list in the footer", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();
      expect(within(popover).getByText("Changes save to “My open issues”.")).toBeInTheDocument();
    });

    it("offers no Save as filter list", async () => {
      const popover = await openFilter(FILTER_LIST);

      expect(within(popover).queryByRole("button", { name: "Save as filter list" })).not.toBeInTheDocument();
    });

    it("saves a changed value to the list after a short delay", async () => {
      const popover = await openFilter(FILTER_LIST);

      await swapPriority(popover);

      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenCalledWith(21, { conditions: [{ field: "priority", op: "is", value: "Low" }] }));
    });

    it("does not save before the delay has passed", async () => {
      const popover = await openFilter(FILTER_LIST);

      await swapPriority(popover);

      expect(mocked.listSetFilter).not.toHaveBeenCalled();
    });

    it("adds a row with a ready-to-use condition and saves it", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      expect(within(popover).getAllByLabelText("Field")).toHaveLength(2);
      await waitFor(() =>
        expect(mocked.listSetFilter).toHaveBeenCalledWith(21, { conditions: [HIGH_PRIORITY, { field: "status", op: "is", value: "3" }] }),
      );
    });

    it("saves the list without a condition that is removed", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.click(within(popover).getByRole("button", { name: "Remove condition" }));

      expect(within(popover).queryByLabelText("Field")).not.toBeInTheDocument();
      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenCalledWith(21, { conditions: [] }));
    });

    it("does not save a condition whose text is still empty", async () => {
      const popover = await openFilter(listConditions({ kind: "list" }));
      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      await userEvent.selectOptions(within(popover).getByLabelText("Field"), "Review notes");
      await wait(500);

      expect(within(popover).getByLabelText("Value")).toHaveValue("");
      expect(mocked.listSetFilter).not.toHaveBeenCalled();
    });

    it("saves a text condition once it has text", async () => {
      const popover = await openFilter(FILTER_LIST);
      await userEvent.selectOptions(within(popover).getByLabelText("Field"), "Review notes");

      await userEvent.type(within(popover).getByLabelText("Value"), "pricing");

      await waitFor(() =>
        expect(mocked.listSetFilter).toHaveBeenLastCalledWith(21, { conditions: [{ field: "customfield_10071", op: "contains", value: "pricing" }] }),
      );
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
      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenCalledWith(21, { conditions: [{ field: "priority", op: "isEmpty" }] }));
    });

    it("starts a days condition at 7", async () => {
      const popover = await openFilter(FILTER_LIST);

      await userEvent.selectOptions(within(popover).getByLabelText("Field"), "Updated");
      await userEvent.selectOptions(within(popover).getByLabelText("Operator"), "in last");

      expect(within(popover).getByLabelText("Value")).toHaveValue(7);
    });
  });

  describe("on a Space", () => {
    it("counts the open issues before any condition is added", async () => {
      const popover = await openFilter(SPACE, 11, "Website relaunch");

      expect(await within(popover).findByText("5 open issues")).toBeInTheDocument();
      expect(within(popover).getByText("Adding a condition here creates a new filter list.")).toBeInTheDocument();
    });

    it("counts the matching issues once a condition is added", async () => {
      const popover = await openFilter(SPACE, 11, "Website relaunch");

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      expect(await within(popover).findByText("5 issues match")).toBeInTheDocument();
    });

    it("keeps Save as filter list disabled until a condition is added", async () => {
      const popover = await openFilter(SPACE, 11, "Website relaunch");
      expect(within(popover).getByRole("button", { name: "Save as filter list" })).toBeDisabled();

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      expect(within(popover).getByRole("button", { name: "Save as filter list" })).toBeEnabled();
    });

    it("never saves the draft to the Space itself", async () => {
      const popover = await openFilter(SPACE, 11, "Website relaunch");

      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));
      await wait(500);

      expect(mocked.listSetFilter).not.toHaveBeenCalled();
    });

    it("creates a filter list in the Space from the draft and opens it", async () => {
      mocked.listCreate.mockResolvedValue(77);
      const popover = await openFilter(SPACE, 11, "Website relaunch");
      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      await userEvent.click(within(popover).getByRole("button", { name: "Save as filter list" }));

      expect(mocked.listCreate).toHaveBeenCalledWith(11, "New filter list", { conditions: [{ field: "status", op: "is", value: "3" }] });
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(77, null));
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("creates the filter list under Everything when drafted there", async () => {
      mocked.listCreate.mockResolvedValue(78);
      mocked.sidebarTree.mockResolvedValue({ ...sidebarTree([space()]), everything: { ...sidebarTree([]).everything, id: 1 } });
      const popover = await openFilter(listConditions({ kind: "everything" }), 1, "Everything");
      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      await userEvent.click(within(popover).getByRole("button", { name: "Save as filter list" }));

      expect(mocked.listCreate).toHaveBeenCalledWith(1, "New filter list", { conditions: [{ field: "status", op: "is", value: "3" }] });
    });

    it("keeps the popover open and shows a toast when the list cannot be created", async () => {
      mocked.listCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const popover = await openFilter(SPACE, 11, "Website relaunch");
      await userEvent.click(within(popover).getByRole("button", { name: "+ Add condition" }));

      await userEvent.click(within(popover).getByRole("button", { name: "Save as filter list" }));

      await waitFor(() => expect(mocked.listCreate).toHaveBeenCalled());
      expect(screen.getByRole("dialog", { name: "Filter" })).toBeInTheDocument();
      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });

  describe("built-in lists", () => {
    it.each<ListConditions["kind"]>(["recent", "pinned"])("says a %s list is built in and offers no editing", async (kind) => {
      const popover = await openFilter(listConditions({ kind }));

      expect(within(popover).getByText("This list is built in.")).toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "+ Add condition" })).not.toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Save as filter list" })).not.toBeInTheDocument();
    });
  });

  it("closes from Done", async () => {
    const popover = await openFilter(FILTER_LIST);

    await userEvent.click(within(popover).getByRole("button", { name: "Done" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
