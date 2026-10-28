import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type MenuEntry, type View } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { narrowingKey } from "../../state/slices/narrowing";
import { resetStore, useStore } from "../../state/store";
import { connectionView } from "../../test/jira";
import { space } from "../../test/spaces";
import { listNode, view } from "../../test/views";
import type { Level } from "../levels";
import { Toolbar } from "./Toolbar";

const mocked = vi.mocked(api, true);

const SPACE: Level = { kind: "space", space: space() };
const FILTER_LIST: Level = { kind: "list", space: space(), node: listNode({ id: 21, name: "My open issues", ownConditions: 2 }) };

function renderToolbar(level: Level = SPACE, current: View = view()) {
  const ui = (v: View) => (
    <QueryClientProvider client={client}>
      <Toolbar level={level} view={v} />
    </QueryClientProvider>
  );
  const client = createQueryClient();
  const result = render(ui(current));
  return { ...result, rerenderWith: (v: View) => result.rerender(ui(v)) };
}

type Entry = Exclude<MenuEntry, "separator">;
const entriesOf = (entries: MenuEntry[]) => entries.filter((e): e is Entry => e !== "separator");

function choose(text: string, submenu?: string) {
  const top = entriesOf(mocked.popupMenu.mock.lastCall![0]);
  const entries = submenu ? entriesOf(top.find((e) => e.text === submenu)!.items!) : top;
  const entry = entries.find((e) => e.text === text);
  if (!entry) throw new Error(`No menu entry "${text}"`);
  return act(async () => entry.run?.());
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.connectionGet.mockResolvedValue(connectionView());
  mocked.listMatchCount.mockResolvedValue({ open: 5, total: 7 });
});

describe("Toolbar", () => {
  describe("Group button", () => {
    it("names the view's current grouping", () => {
      renderToolbar(SPACE, view({ groupBy: "status" }));

      expect(screen.getByRole("button", { name: "Group: Status" })).toBeInTheDocument();
    });

    it("opens a menu with the groupings, the current one checked, and a Sort by submenu", async () => {
      renderToolbar(SPACE, view({ groupBy: "status" }));

      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      const entries = entriesOf(mocked.popupMenu.mock.lastCall![0]);
      expect(entries.filter((e) => e.checked).map((e) => e.text)).toEqual(["Status"]);
      expect(entries.map((e) => e.text)).toContain("Epic");
      expect(entries.find((e) => e.text === "Sort by")?.items).toBeDefined();
    });

    it("saves the grouping chosen from the menu to the view", async () => {
      mocked.viewSetGroup.mockResolvedValue(view({ groupBy: "epic" }));
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      await choose("Epic");

      expect(mocked.viewSetGroup).toHaveBeenCalledWith(101, "epic");
    });

    it("saves an ascending sort chosen from the Sort by submenu", async () => {
      mocked.viewSetSort.mockResolvedValue(view());
      renderToolbar(SPACE, view({ sort: { by: "key", descending: true } }));
      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      await choose("Ascending", "Sort by");

      expect(mocked.viewSetSort).toHaveBeenCalledWith(101, { by: "key", descending: false });
    });

    it("shows a toast when the view cannot be changed", async () => {
      mocked.viewSetGroup.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      await choose("Epic");

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change the grouping: Disk full"));
    });
  });

  describe("Filter button", () => {
    it("shows the number of the filter list's own conditions", () => {
      renderToolbar(FILTER_LIST);

      expect(screen.getByLabelText("2 conditions")).toHaveTextContent("2");
    });

    it("uses the singular for one condition", () => {
      renderToolbar({ kind: "list", space: space(), node: listNode({ ownConditions: 1 }) });

      expect(screen.getByLabelText("1 condition")).toBeInTheDocument();
    });

    it("shows no badge on a Space", () => {
      renderToolbar(SPACE);

      expect(screen.queryByLabelText(/condition/)).not.toBeInTheDocument();
    });
  });

  describe("popovers", () => {
    it("labels the columns button Card fields on a board", () => {
      renderToolbar(SPACE, view({ layout: "board" }));

      expect(screen.getByRole("button", { name: "Card fields" })).toBeInTheDocument();
    });

    it("opens the Columns popover and marks its button expanded", async () => {
      renderToolbar();
      const button = screen.getByRole("button", { name: "Columns" });
      expect(button).toHaveAttribute("aria-expanded", "false");

      await userEvent.click(button);

      expect(await screen.findByRole("dialog", { name: "Columns for this view" })).toBeInTheDocument();
      expect(button).toHaveAttribute("aria-expanded", "true");
    });

    it("closes the Columns popover when its button is pressed again", async () => {
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Columns" }));

      await userEvent.click(screen.getByRole("button", { name: "Columns" }));

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Columns" })).toHaveAttribute("aria-expanded", "false");
    });

    it("keeps only one popover open: Filter replaces Columns", async () => {
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Columns" }));

      await userEvent.click(screen.getByRole("button", { name: "Filter" }));

      expect(await screen.findByRole("dialog", { name: "Filter" })).toBeInTheDocument();
      expect(screen.queryByRole("dialog", { name: "Columns for this view" })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Filter" })).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByRole("button", { name: "Columns" })).toHaveAttribute("aria-expanded", "false");
    });
  });

  describe("Me chip", () => {
    const KEY = narrowingKey(11, 101);

    it("starts off as a plain Me button", () => {
      renderToolbar();

      expect(screen.getByRole("button", { name: "Me" })).toHaveAttribute("aria-pressed", "false");
      expect(useStore.getState().narrowings[KEY]).toBeUndefined();
    });

    it("turns on with the connected account's initials and narrows the view to me", async () => {
      renderToolbar();

      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      const chip = screen.getByRole("button", { name: "Only my issues, click to show all" });
      expect(chip).toHaveAttribute("aria-pressed", "true");
      expect(await screen.findByText("AM")).toBeInTheDocument();
      expect(useStore.getState().narrowings[KEY]).toMatchObject({ me: true });
    });

    it("turns off again when the chip is clicked", async () => {
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      await userEvent.click(screen.getByRole("button", { name: "Only my issues, click to show all" }));

      expect(screen.getByRole("button", { name: "Me" })).toHaveAttribute("aria-pressed", "false");
      expect(useStore.getState().narrowings[KEY]?.me).toBe(false);
    });

    it("is remembered per view", async () => {
      const { rerenderWith } = renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      rerenderWith(view({ id: 102, name: "Board" }));

      expect(screen.getByRole("button", { name: "Me" })).toHaveAttribute("aria-pressed", "false");
    });
  });

  describe("text filter", () => {
    it("narrows the view after typing and keeps the Me setting", async () => {
      renderToolbar();
      await userEvent.click(screen.getByRole("button", { name: "Me" }));

      await userEvent.type(screen.getByRole("textbox", { name: "Filter this view" }), "pricing");

      await waitFor(() => expect(useStore.getState().narrowings[narrowingKey(11, 101)]).toMatchObject({ me: true, text: "pricing" }));
    });

    it("shows the text already set for the view", () => {
      useStore.getState().setNarrowing(narrowingKey(11, 101), { text: "footer" });

      renderToolbar();

      expect(screen.getByRole("textbox", { name: "Filter this view" })).toHaveValue("footer");
    });

    it("starts empty on a different view", () => {
      useStore.getState().setNarrowing(narrowingKey(11, 101), { text: "footer" });
      const { rerenderWith } = renderToolbar();

      rerenderWith(view({ id: 102, name: "Board" }));

      expect(screen.getByRole("textbox", { name: "Filter this view" })).toHaveValue("");
    });
  });
});
