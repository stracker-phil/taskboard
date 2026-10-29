import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Column, type IssueOpening, type View } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { settings } from "../../test/settings";
import { view } from "../../test/views";
import { ColumnsPopover } from "./ColumnsPopover";

const mocked = vi.mocked(api, true);

const CATALOG: Column[] = [
  { id: "status", name: "Status", custom: false },
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
  mocked.getSettings.mockResolvedValue(settings());
  mocked.columnsCatalogue.mockResolvedValue(CATALOG);
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
    expect(within(popover).getByRole("button", { name: "Labels (column)" })).toHaveAttribute("aria-pressed", "false");
  });

  it("saves the view's columns with the toggled-on column added", async () => {
    const popover = await openColumns();

    await userEvent.click(within(popover).getByRole("button", { name: "Labels (column)" }));

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

    await userEvent.click(within(popover).getByRole("button", { name: "Labels (column)" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change the columns: Disk full"));
  });

  it("lists the labels field as a card field in a board view", async () => {
    const popover = await openColumns(view({ layout: "board" }));

    expect(within(popover).getByRole("button", { name: "Labels (card field)" })).toBeInTheDocument();
    expect(within(popover).queryByRole("button", { name: "Labels (column)" })).not.toBeInTheDocument();
  });

  it.each(["list", "table"] as const)("lists the labels field as a column in a %s view", async (layout) => {
    const popover = await openColumns(view({ layout }));

    expect(within(popover).getByRole("button", { name: "Labels (column)" })).toBeInTheDocument();
    expect(within(popover).queryByRole("button", { name: "Labels (card field)" })).not.toBeInTheDocument();
  });

  it("saves the labels card field of a board view when it is toggled on", async () => {
    const popover = await openColumns(view({ layout: "board", columns: ["assignee"] }));

    await userEvent.click(within(popover).getByRole("button", { name: "Labels (card field)" }));

    expect(mocked.viewSetColumns).toHaveBeenCalledWith(101, ["assignee", "labels"]);
  });

  it("lists Status among the Jira fields, off when the view lacks it", async () => {
    const popover = await openColumns();

    expect(within(popover).getByRole("button", { name: "Status" })).toHaveAttribute("aria-pressed", "false");
  });

  describe("Wrap long titles", () => {
    it("is on, with the hint on, for a list view that wraps", async () => {
      const popover = await openColumns(view({ wrapTitles: true }));

      const toggle = within(popover).getByRole("button", { name: /^Wrap long titles ?on$/ });
      expect(toggle).toHaveAttribute("aria-pressed", "true");
    });

    it("is off, with the hint truncate, for a list view that does not wrap", async () => {
      const popover = await openColumns(view({ wrapTitles: false }));

      const toggle = within(popover).getByRole("button", { name: /^Wrap long titles ?truncate$/ });
      expect(toggle).toHaveAttribute("aria-pressed", "false");
    });

    it.each([
      [true, false],
      [false, true],
    ])("saves the opposite of the current setting (%s -> %s)", async (current, next) => {
      mocked.viewSetWrap.mockResolvedValue(view({ wrapTitles: next }));
      const popover = await openColumns(view({ wrapTitles: current }));

      await userEvent.click(within(popover).getByRole("button", { name: /^Wrap long titles/ }));

      expect(mocked.viewSetWrap).toHaveBeenCalledWith(101, next);
    });

    it("shows a toast when the setting cannot be saved", async () => {
      mocked.viewSetWrap.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const popover = await openColumns();

      await userEvent.click(within(popover).getByRole("button", { name: /^Wrap long titles/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change how titles show: Disk full"));
    });

    it.each(["table", "board"] as const)("is not offered in a %s view", async (layout) => {
      const popover = await openColumns(view({ layout }));

      expect(within(popover).queryByRole("button", { name: /^Wrap long titles/ })).not.toBeInTheDocument();
    });
  });

  describe("Labels (inline)", () => {
    it("is on, with the hint after the title, for a view that shows labels inline", async () => {
      const popover = await openColumns(view({ inlineLabels: true }));

      expect(within(popover).getByRole("button", { name: /^Labels \(inline\) ?after the title$/ })).toHaveAttribute("aria-pressed", "true");
    });

    it("is off for a view that does not show labels inline", async () => {
      const popover = await openColumns(view({ inlineLabels: false }));

      expect(within(popover).getByRole("button", { name: /^Labels \(inline\)/ })).toHaveAttribute("aria-pressed", "false");
    });

    it("says it is off while the Labels column shows", async () => {
      const popover = await openColumns(view({ columns: ["labels"] }));

      expect(within(popover).getByRole("button", { name: /^Labels \(inline\) ?off while the column shows$/ })).toBeInTheDocument();
    });

    it.each([
      [true, false],
      [false, true],
    ])("saves the opposite of the current setting (%s -> %s)", async (current, next) => {
      mocked.viewSetInlineLabels.mockResolvedValue(view({ inlineLabels: next }));
      const popover = await openColumns(view({ inlineLabels: current }));

      await userEvent.click(within(popover).getByRole("button", { name: /^Labels \(inline\)/ }));

      expect(mocked.viewSetInlineLabels).toHaveBeenCalledWith(101, next);
    });

    it("shows a toast when the setting cannot be saved", async () => {
      mocked.viewSetInlineLabels.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const popover = await openColumns();

      await userEvent.click(within(popover).getByRole("button", { name: /^Labels \(inline\)/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change how labels show: Disk full"));
    });

    it.each(["list", "table", "board"] as const)("is offered in a %s view", async (layout) => {
      const popover = await openColumns(view({ layout }));

      expect(within(popover).getByRole("button", { name: /^Labels \(inline\)/ })).toBeInTheDocument();
    });

    it("saves the opposite of the current setting in a board view", async () => {
      mocked.viewSetInlineLabels.mockResolvedValue(view({ layout: "board", inlineLabels: false }));
      const popover = await openColumns(view({ layout: "board", inlineLabels: true }));

      await userEvent.click(within(popover).getByRole("button", { name: /^Labels \(inline\)/ }));

      expect(mocked.viewSetInlineLabels).toHaveBeenCalledWith(101, false);
    });

    it("says it is off while the card field shows in a board view", async () => {
      const popover = await openColumns(view({ layout: "board", columns: ["labels"] }));

      expect(within(popover).getByRole("button", { name: /^Labels \(inline\) ?off while the card field shows$/ })).toBeInTheDocument();
    });
  });

  describe("Open issues in", () => {
    const group = (popover: HTMLElement) => within(popover).getByRole("group", { name: "Open issues in" });

    it.each(["list", "table"] as const)("offers Default, Preview and Full view in a %s view", async (layout) => {
      const popover = await openColumns(view({ layout }));

      expect(within(group(popover)).getAllByRole("button").map((b) => b.textContent)).toEqual(["Default (Preview)", "Preview", "Full view"]);
    });

    it("is not offered in a board view", async () => {
      const popover = await openColumns(view({ layout: "board" }));

      expect(within(popover).queryByRole("group", { name: "Open issues in" })).not.toBeInTheDocument();
    });

    it("names the app setting in the Default button", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "untilUnpinned", openIssuesIn: "full" } }));
      const popover = await openColumns();

      expect(await within(group(popover)).findByRole("button", { name: "Default (Full view)" })).toBeInTheDocument();
    });

    it.each<[IssueOpening | null, string]>([
      [null, "Default (Preview)"],
      ["preview", "Preview"],
      ["full", "Full view"],
    ])("presses the button of the view's choice %s", async (openWith, name) => {
      const popover = await openColumns(view({ openWith }));

      expect(within(group(popover)).getByRole("button", { pressed: true })).toHaveAccessibleName(name);
    });

    it.each<[string, IssueOpening | null, IssueOpening | null]>([
      ["Default (Preview)", "full", null],
      ["Preview", null, "preview"],
      ["Full view", null, "full"],
    ])("saves %s as the view's choice", async (name, current, saved) => {
      mocked.viewSetOpenWith.mockResolvedValue(view({ openWith: saved }));
      const popover = await openColumns(view({ openWith: current }));

      await userEvent.click(within(group(popover)).getByRole("button", { name }));

      expect(mocked.viewSetOpenWith).toHaveBeenCalledWith(101, saved);
    });

    it("shows a toast when the choice cannot be saved", async () => {
      mocked.viewSetOpenWith.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const popover = await openColumns();

      await userEvent.click(within(group(popover)).getByRole("button", { name: "Full view" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change how issues open: Disk full"));
    });
  });

  describe("search", () => {
    const search = async (popover: HTMLElement, text: string) => userEvent.type(within(popover).getByRole("textbox", { name: "Search fields" }), text);

    it("keeps the fields whose shown name contains the text, ignoring case", async () => {
      const popover = await openColumns();

      await search(popover, "ASSIGN");

      expect(within(popover).getByRole("button", { name: "Assignee" })).toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Priority" })).not.toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: /^Reviewer/ })).not.toBeInTheDocument();
    });

    it("matches the due column by its shown name Due date", async () => {
      const popover = await openColumns();

      await search(popover, "due da");

      expect(within(popover).getByRole("button", { name: "Due date" })).toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Assignee" })).not.toBeInTheDocument();
    });

    it.each([
      ["cf_10071", /^Reviewer ?cf_10071$/],
      ["customfield_10071", /^Reviewer ?cf_10071$/],
      ["fix version", /^Release ?fix version$/],
      ["PARENT", /^Epic ?parent$/],
    ])("finds a field by its Jira id or hint: %s", async (text, expected) => {
      const popover = await openColumns();

      await search(popover, text);

      expect(within(popover).getByRole("button", { name: expected })).toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Assignee" })).not.toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Labels (column)" })).not.toBeInTheDocument();
    });

    it("drops the section header of a section without matches", async () => {
      const popover = await openColumns();

      await search(popover, "rev");

      expect(within(popover).getByText("Custom fields · this project")).toBeInTheDocument();
      expect(within(popover).queryByText("Jira fields")).not.toBeInTheDocument();

      await userEvent.clear(within(popover).getByRole("textbox", { name: "Search fields" }));
      await search(popover, "assign");

      expect(within(popover).getByText("Jira fields")).toBeInTheDocument();
      expect(within(popover).queryByText("Custom fields · this project")).not.toBeInTheDocument();
    });

    it("says so when no field matches, quoting the trimmed text", async () => {
      const popover = await openColumns();

      await search(popover, " xyz ");

      expect(within(popover).getByText("No field matches “xyz”.")).toBeInTheDocument();
      expect(within(popover).queryByRole("button", { name: "Assignee" })).not.toBeInTheDocument();
    });

    it("shows every field again once the search is cleared", async () => {
      const popover = await openColumns();
      await search(popover, "xyz");

      await userEvent.clear(within(popover).getByRole("textbox", { name: "Search fields" }));

      expect(within(popover).getByRole("button", { name: "Assignee" })).toBeInTheDocument();
      expect(within(popover).getByRole("button", { name: /^Reviewer ?cf_10071$/ })).toBeInTheDocument();
    });

    it("saves the view's columns plus the toggled custom field in catalog order", async () => {
      const popover = await openColumns();
      await search(popover, "rev");

      await userEvent.click(within(popover).getByRole("button", { name: /^Reviewer ?cf_10071$/ }));

      expect(mocked.viewSetColumns).toHaveBeenCalledWith(101, ["assignee", "priority", "customfield_10071"]);
    });

    it("keeps the view's own column order when toggling a column on while searching", async () => {
      const popover = await openColumns(view({ columns: ["priority", "assignee"] }));
      await search(popover, "epic");

      await userEvent.click(within(popover).getByRole("button", { name: /^Epic/ }));

      expect(mocked.viewSetColumns).toHaveBeenCalledWith(101, ["priority", "assignee", "epic"]);
    });

    it("saves the view's columns without the toggled-off column while searching", async () => {
      const popover = await openColumns();
      await search(popover, "prio");

      await userEvent.click(within(popover).getByRole("button", { name: "Priority" }));

      expect(mocked.viewSetColumns).toHaveBeenCalledWith(101, ["assignee"]);
    });
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

    it("does not reorder while a search is filtering the fields", async () => {
      const popover = await openColumns();
      await userEvent.type(within(popover).getByRole("textbox", { name: "Search fields" }), "e");
      const assignee = within(popover).getByRole("button", { name: "Assignee" });

      fireEvent.pointerDown(assignee, { button: 0, clientY: 10 });
      fireEvent.pointerMove(assignee, { buttons: 1, clientY: 70 });
      fireEvent.pointerUp(assignee);

      expect(mocked.viewSetColumns).not.toHaveBeenCalled();
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
