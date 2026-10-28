import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Column, type View } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { view } from "../../test/views";
import { ColumnsPopover } from "./ColumnsPopover";

const mocked = vi.mocked(api, true);

const CATALOGUE: Column[] = [
  { id: "assignee", name: "Assignee", custom: false },
  { id: "priority", name: "Priority", custom: false },
  { id: "epic", name: "Epic", custom: false },
  { id: "release", name: "Release", custom: false },
  { id: "labels", name: "Labels", custom: false },
  { id: "due", name: "Due", custom: false },
  { id: "customfield_10071", name: "Reviewer", custom: true },
];

function Harness({ current, projectId }: { current: View; projectId: string | null }) {
  const anchor = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  return (
    <>
      <button ref={anchor} type="button" onClick={() => setOpen((o) => !o)}>
        Columns
      </button>
      {open && <ColumnsPopover view={current} spaceProjectId={projectId} anchor={anchor} onClose={() => setOpen(false)} />}
    </>
  );
}

async function openColumns(current: View = view({ columns: ["assignee", "priority"] }), projectId: string | null = "10001") {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness current={current} projectId={projectId} />
    </QueryClientProvider>,
  );
  await userEvent.click(screen.getByRole("button", { name: "Columns" }));
  await screen.findByRole("button", { name: /^Reviewer ?cf_10071$/ });
  return screen.getByRole("dialog", { name: "Columns for this view" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.columnsCatalogue.mockResolvedValue(CATALOGUE);
  mocked.viewSetColumns.mockResolvedValue(view());
});

describe("ColumnsPopover", () => {
  it("lists the Jira fields with hints for the ones that map to another Jira name", async () => {
    const popover = await openColumns();

    expect(within(popover).getByRole("button", { name: "Assignee" })).toBeInTheDocument();
    expect(within(popover).getByRole("button", { name: /^Epic ?parent$/ })).toBeInTheDocument();
    expect(within(popover).getByRole("button", { name: /^Release ?fix version$/ })).toBeInTheDocument();
  });

  it("calls the due column Due date", async () => {
    const popover = await openColumns();

    expect(within(popover).getByRole("button", { name: "Due date" })).toBeInTheDocument();
  });

  it("lists custom fields in their own section with the field id as a hint", async () => {
    const popover = await openColumns();

    expect(within(popover).getByText("Custom fields · this project")).toBeInTheDocument();
    expect(within(popover).getByRole("button", { name: /^Reviewer ?cf_10071$/ })).toBeInTheDocument();
  });

  it("shows the view's columns as on and the rest as off", async () => {
    const popover = await openColumns();

    expect(within(popover).getByRole("button", { name: "Assignee" })).toHaveAttribute("aria-pressed", "true");
    expect(within(popover).getByRole("button", { name: "Labels" })).toHaveAttribute("aria-pressed", "false");
  });

  it("saves the view's columns with the toggled-on column added", async () => {
    const popover = await openColumns();

    await userEvent.click(within(popover).getByRole("button", { name: "Labels" }));

    expect(mocked.viewSetColumns).toHaveBeenCalledWith(101, ["assignee", "priority", "labels"]);
  });

  it("saves the view's columns without the toggled-off column", async () => {
    const popover = await openColumns();

    await userEvent.click(within(popover).getByRole("button", { name: "Assignee" }));

    expect(mocked.viewSetColumns).toHaveBeenCalledWith(101, ["priority"]);
  });

  it("shows a toast when the columns cannot be saved", async () => {
    mocked.viewSetColumns.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    const popover = await openColumns();

    await userEvent.click(within(popover).getByRole("button", { name: "Labels" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change the columns: Disk full"));
  });

  describe("Use as Space default", () => {
    it("saves the view's columns as the Space's default", async () => {
      mocked.spaceDefaultColumns.mockResolvedValue(undefined);
      const popover = await openColumns(view({ columns: ["assignee", "priority"] }), "10001");

      await userEvent.click(within(popover).getByRole("button", { name: "Use as Space default" }));

      expect(mocked.spaceDefaultColumns).toHaveBeenCalledWith("10001", ["assignee", "priority"]);
      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("New views in this Space start with these columns"));
    });

    it("is not offered outside a Space", async () => {
      const popover = await openColumns(view(), null);

      expect(within(popover).queryByRole("button", { name: "Use as Space default" })).not.toBeInTheDocument();
    });
  });

  it("closes on Escape and returns focus to the Columns button", async () => {
    await openColumns();

    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Columns" })).toHaveFocus();
  });

  describe("drag to reorder", () => {
    let rects: { mockRestore: () => void };

    // Items are 40px tall, stacked from the top by their drag index.
    beforeEach(() => {
      rects = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        const index = this.getAttribute("data-drag-index");
        const top = index === null ? 0 : Number(index) * 40;
        const height = index === null ? 0 : 40;
        return { top, bottom: top + height, height, left: 0, right: 0, width: 0, x: 0, y: top, toJSON: () => ({}) };
      });
    });

    afterEach(() => rects.mockRestore());

    it("saves the new order of the view's columns when a row is dropped lower", async () => {
      const popover = await openColumns();
      const assignee = within(popover).getByRole("button", { name: "Assignee" });

      fireEvent.pointerDown(assignee, { button: 0, clientY: 10 });
      fireEvent.pointerMove(assignee, { buttons: 1, clientY: 70 });
      fireEvent.pointerUp(assignee);

      expect(mocked.viewSetColumns).toHaveBeenCalledWith(101, ["priority", "assignee"]);
    });

    it("does not toggle the column whose drag just ended", async () => {
      const popover = await openColumns();
      const assignee = within(popover).getByRole("button", { name: "Assignee" });

      fireEvent.pointerDown(assignee, { button: 0, clientY: 10 });
      fireEvent.pointerMove(assignee, { buttons: 1, clientY: 70 });
      fireEvent.pointerUp(assignee);
      mocked.viewSetColumns.mockClear();
      fireEvent.click(assignee);

      expect(mocked.viewSetColumns).not.toHaveBeenCalled();
    });
  });
});
