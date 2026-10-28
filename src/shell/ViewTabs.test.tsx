import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type MenuEntry } from "../api";
import { installCommands, useCommandKeys } from "../commands";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { view } from "../test/views";
import { ViewTabs } from "./ViewTabs";

const mocked = vi.mocked(api, true);

const LIST = view({ id: 101, name: "List", layout: "list" });
const BOARD = view({ id: 102, name: "Sprint board", layout: "board", position: 1 });
const TABLE = view({ id: 103, name: "Overview", layout: "table", position: 2 });

function Harness({ viewId }: { viewId: number }) {
  useCommandKeys();
  return <ViewTabs listId={11} viewId={viewId} />;
}

async function renderTabs(views = [LIST, BOARD], viewId = 101) {
  mocked.viewsOf.mockResolvedValue(views);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness viewId={viewId} />
    </QueryClientProvider>,
  );
  await screen.findByRole("tab", { name: views[0].name });
}

type Entry = Exclude<MenuEntry, "separator">;
const entriesOf = (entries: MenuEntry[]) => entries.filter((e): e is Entry => e !== "separator");

/** Runs the entry with this text from the native menu that was last popped up (or from one of its submenus). */
function choose(text: string, submenu?: string) {
  const top = entriesOf(mocked.popupMenu.mock.lastCall![0]);
  const entries = submenu ? entriesOf(top.find((e) => e.text === submenu)!.items!) : top;
  const entry = entries.find((e) => e.text === text);
  if (!entry) throw new Error(`No menu entry "${text}"`);
  return act(async () => entry.run?.());
}

const entry = (text: string, submenu?: string) => {
  const top = entriesOf(mocked.popupMenu.mock.lastCall![0]);
  return entriesOf(submenu ? top.find((e) => e.text === submenu)!.items! : top).find((e) => e.text === text)!;
};

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
});

describe("ViewTabs", () => {
  describe("tabs", () => {
    it("lists the level's views as tabs and marks the current one selected", async () => {
      await renderTabs();

      expect(screen.getByRole("tab", { name: "List" })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("tab", { name: "Sprint board" })).toHaveAttribute("aria-selected", "false");
    });

    it("opens a view when its tab is clicked", async () => {
      await renderTabs();

      await userEvent.click(screen.getByRole("tab", { name: "Sprint board" }));

      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
    });
  });

  describe("keys", () => {
    it("opens the second view on ⌘2", async () => {
      await renderTabs();

      await userEvent.keyboard("{Meta>}2{/Meta}");

      expect(mocked.navGo).toHaveBeenCalledWith(11, 102);
    });

    it("does nothing on a number key beyond the last tab", async () => {
      await renderTabs();

      await userEvent.keyboard("{Meta>}3{/Meta}");

      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });

  describe("+ View", () => {
    it("offers the three layouts", async () => {
      await renderTabs();

      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      expect(entriesOf(mocked.popupMenu.mock.lastCall![0]).map((e) => e.text)).toEqual(["List", "Board", "Table"]);
    });

    it.each([
      ["List", "list"],
      ["Board", "board"],
      ["Table", "table"],
    ] as const)("adds a %s view and opens it", async (name, layout) => {
      mocked.viewCreate.mockResolvedValue(view({ id: 200, name, layout }));
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await choose(name);

      expect(mocked.viewCreate).toHaveBeenCalledWith(11, name, layout);
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(11, 200));
    });

    it("shows an error toast and stays put when the view cannot be added", async () => {
      mocked.viewCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderTabs();
      await userEvent.click(screen.getByRole("button", { name: "+ View" }));

      await choose("Board");

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not add the view: Disk full"));
      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });

  describe("context menu", () => {
    it("renames a view in place", async () => {
      mocked.viewRename.mockResolvedValue({ ...BOARD, name: "Release board" });
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      await choose("Rename");
      const field = await screen.findByRole("textbox", { name: "View name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Release board{Enter}");

      expect(mocked.viewRename).toHaveBeenCalledWith(102, "Release board");
      expect(screen.queryByRole("textbox", { name: "View name" })).not.toBeInTheDocument();
    });

    it("leaves a view's name alone when the rename is cancelled", async () => {
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      await choose("Rename");
      await userEvent.type(await screen.findByRole("textbox", { name: "View name" }), "Other{Escape}");

      expect(mocked.viewRename).not.toHaveBeenCalled();
      expect(screen.getByRole("tab", { name: "Sprint board" })).toBeInTheDocument();
    });

    it("duplicates a view", async () => {
      mocked.viewDuplicate.mockResolvedValue(view({ id: 300, name: "Sprint board copy", layout: "board" }));
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      await choose("Duplicate");

      expect(mocked.viewDuplicate).toHaveBeenCalledWith(102);
    });

    it("changes a view's layout, with its current layout disabled", async () => {
      mocked.viewSetLayout.mockResolvedValue({ ...BOARD, layout: "table" });
      await renderTabs();
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Sprint board" }) });

      expect(entry("Board", "Change layout").enabled).toBe(false);
      expect(entry("List", "Change layout").enabled).toBe(true);
      expect(entry("Table", "Change layout").enabled).toBe(true);
      await choose("Table", "Change layout");

      expect(mocked.viewSetLayout).toHaveBeenCalledWith(102, "table");
    });

    it("disables Delete for the only view", async () => {
      await renderTabs([LIST]);

      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      expect(entry("Delete").enabled).toBe(false);
    });

    it("deletes the current view and opens the level's default view", async () => {
      mocked.viewDelete.mockResolvedValue([BOARD]);
      await renderTabs([LIST, BOARD], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      expect(entry("Delete").enabled).toBe(true);
      await choose("Delete");

      expect(mocked.viewDelete).toHaveBeenCalledWith(101);
      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(11, null));
    });

    it("stays on the current view when another view is deleted", async () => {
      mocked.viewDelete.mockResolvedValue([LIST]);
      await renderTabs([LIST, BOARD, TABLE], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "Overview" }) });

      await choose("Delete");

      expect(mocked.viewDelete).toHaveBeenCalledWith(103);
      expect(mocked.navGo).not.toHaveBeenCalled();
    });

    it("shows an error toast when a view cannot be deleted", async () => {
      mocked.viewDelete.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderTabs([LIST, BOARD], 101);
      await userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("tab", { name: "List" }) });

      await choose("Delete");

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not delete the view: Disk full"));
      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });
});
