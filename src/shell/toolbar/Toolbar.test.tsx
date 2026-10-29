import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { MenuHost } from "../../components/menu";
import { installCommands, useCommandKeys } from "../../commands";
import { runPageAction } from "../../commands/pageActions";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import type { View } from "../../api";
import { filterOptions, listConditions } from "../../test/filters";
import { space } from "../../test/spaces";
import { listNode, view } from "../../test/views";
import type { Level } from "../levels";
import { Toolbar } from "./Toolbar";

const mocked = vi.mocked(api, true);

const LIST_LEVEL: Level = { kind: "list", space: space(), node: listNode() };
const SPACE_LEVEL: Level = { kind: "space", space: space() };

function Harness({ level, ownConditions, patch }: { level: Level; ownConditions: number; patch: Partial<View> }) {
  useCommandKeys();
  return <Toolbar level={level} view={view({ groupBy: "status", ownConditions, ...patch })} />;
}

function renderToolbar(level: Level = LIST_LEVEL, ownConditions = 0, patch: Partial<View> = {}) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness level={level} ownConditions={ownConditions} patch={patch} />
      <MenuHost />
    </QueryClientProvider>,
  );
}

const run = (id: string) => {
  let offered = false;
  act(() => {
    offered = runPageAction(id);
  });
  return offered;
};

const filterField = () => screen.getByRole("textbox", { name: "Filter this view" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
  mocked.listMatchCount.mockResolvedValue({ open: 5, total: 9 });
});

const IN_PROGRESS = { field: "status", op: "is", value: "3" } as const;
const NO_DUE_DATE = { field: "due", op: "isEmpty" } as const;
const saveViewButton = () => screen.queryByRole("button", { name: "Save View" });

/** Adds a condition in the named popover section, then closes the popover. */
async function addCondition(sectionName: "All views of this list" | "This view") {
  run("toolbar.filter");
  const dialog = await screen.findByRole("dialog", { name: "Filter" });
  const group = await within(dialog).findByRole("group", { name: sectionName });
  await userEvent.click(within(group).getByRole("button", { name: "+ Add condition" }));
  await userEvent.click(within(dialog).getByRole("button", { name: "Done" }));
}

const meChip = () => screen.getByRole("button", { name: /^(Me|Only my issues, click to show all)$/ });

describe("Toolbar page actions", () => {
  describe("toolbar.me", () => {
    it("turns the Me narrowing on and off", () => {
      renderToolbar();
      expect(screen.getByRole("button", { name: "Me", pressed: false })).toBeInTheDocument();

      run("toolbar.me");
      expect(screen.getByRole("button", { name: "Only my issues, click to show all", pressed: true })).toBeInTheDocument();

      run("toolbar.me");
      expect(screen.getByRole("button", { name: "Me", pressed: false })).toBeInTheDocument();
    });

    it("toggles on ⌘⇧M", async () => {
      renderToolbar();

      await userEvent.keyboard("{Meta>}{Shift>}m{/Shift}{/Meta}");

      expect(screen.getByRole("button", { name: "Only my issues, click to show all" })).toBeInTheDocument();
    });
  });

  describe("toolbar.search", () => {
    it("focuses the filter field and selects what it holds", async () => {
      renderToolbar();
      await userEvent.type(filterField(), "pricing");
      await userEvent.click(document.body);
      expect(filterField()).not.toHaveFocus();

      run("toolbar.search");

      expect(filterField()).toHaveFocus();
      expect(filterField()).toHaveProperty("selectionStart", 0);
      expect(filterField()).toHaveProperty("selectionEnd", "pricing".length);
    });

    it("focuses the filter field on ⌘F", async () => {
      renderToolbar();

      await userEvent.keyboard("{Meta>}f{/Meta}");

      expect(filterField()).toHaveFocus();
    });
  });

  describe("toolbar.filter", () => {
    it("opens the Filter dialog", async () => {
      renderToolbar();

      run("toolbar.filter");

      expect(await screen.findByRole("dialog", { name: "Filter" })).toBeInTheDocument();
    });

    it("is offered on a Space level too", async () => {
      renderToolbar(SPACE_LEVEL);

      expect(run("toolbar.filter")).toBe(true);

      expect(await screen.findByRole("dialog", { name: "Filter" })).toBeInTheDocument();
    });
  });

  describe("Filter button badge", () => {
    it("shows no badge when neither the list nor the view has conditions", () => {
      renderToolbar(
        { kind: "list", space: space(), node: listNode({ ownConditions: 0 }) },
        0,
      );

      expect(screen.getByRole("button", { name: "Filter" })).toBeInTheDocument();
      expect(screen.queryByLabelText(/condition/)).not.toBeInTheDocument();
    });

    it("counts the list's and the view's conditions together", () => {
      renderToolbar({ kind: "list", space: space(), node: listNode({ ownConditions: 1 }) }, 2);

      expect(screen.getByLabelText("3 conditions")).toHaveTextContent("3");
    });

    it("counts only the view's conditions on a Space level", () => {
      renderToolbar(SPACE_LEVEL, 2);

      expect(screen.getByRole("button", { name: /^Filter/ })).toBeInTheDocument();
      expect(screen.getByLabelText("2 conditions")).toHaveTextContent("2");
    });

    it("says 1 condition for a single one", () => {
      renderToolbar(SPACE_LEVEL, 1);

      expect(screen.getByLabelText("1 condition")).toBeInTheDocument();
    });
  });

  describe("toolbar.columns", () => {
    it("opens the Columns dialog", async () => {
      renderToolbar();

      run("toolbar.columns");

      expect(await screen.findByRole("dialog", { name: "Columns for this view" })).toBeInTheDocument();
    });

    it("is offered on a Space level too", async () => {
      renderToolbar(SPACE_LEVEL);

      run("toolbar.columns");

      expect(await screen.findByRole("dialog", { name: "Columns for this view" })).toBeInTheDocument();
    });
  });

  describe("toolbar.group", () => {
    it("opens the group menu with the view's grouping checked", async () => {
      renderToolbar();

      run("toolbar.group");

      const menu = within(await screen.findByRole("menu", { name: "Group" }));
      expect(menu.getByRole("menuitemradio", { name: "Status" })).toHaveAttribute("aria-checked", "true");
      expect(menu.getByRole("menuitemradio", { name: "Priority" })).toHaveAttribute("aria-checked", "false");
      expect(menu.getByRole("menuitem", { name: "Sort by" })).toBeInTheDocument();
    });

    it("opens the group menu from the Group button", async () => {
      renderToolbar();

      await userEvent.click(screen.getByRole("button", { name: "Group: Status" }));

      expect(await screen.findByRole("menu", { name: "Group" })).toBeInTheDocument();
    });

    it("groups by the chosen entry", async () => {
      mocked.viewSetGroup.mockResolvedValue(view({ groupBy: "priority" }));
      renderToolbar();
      run("toolbar.group");

      await userEvent.click(await screen.findByRole("menuitemradio", { name: "Priority" }));

      expect(mocked.viewSetGroup).toHaveBeenCalledWith(expect.any(Number), "priority");
    });
  });

  describe("Subtasks button", () => {
    const subtasksButton = () => screen.getByRole("button", { name: "Subtasks" });

    it("sits right after the Columns button", () => {
      renderToolbar();

      expect(subtasksButton().previousElementSibling).toContainElement(screen.getByRole("button", { name: /^Columns/ }));
    });

    it("is absent on a Board view", () => {
      renderToolbar(LIST_LEVEL, 0, { layout: "board" });

      expect(screen.queryByRole("button", { name: "Subtasks" })).not.toBeInTheDocument();
    });

    it.each([
      ["hidden", false],
      ["hierarchy", true],
      ["list", true],
    ] as const)("styles the button as active only when subtasks show (%s)", (subtasks, active) => {
      renderToolbar(LIST_LEVEL, 0, { subtasks });

      expect(subtasksButton().classList.contains("toolbar-btn--active")).toBe(active);
    });

    it("offers the three modes with a check on the current one", async () => {
      renderToolbar(LIST_LEVEL, 0, { subtasks: "hierarchy" });

      await userEvent.click(subtasksButton());

      const menu = within(await screen.findByRole("menu", { name: "Subtasks" }));
      expect(menu.getByRole("menuitemradio", { name: /^Hidden/ })).toHaveAttribute("aria-checked", "false");
      expect(menu.getByRole("menuitemradio", { name: /^Hierarchy/ })).toHaveAttribute("aria-checked", "true");
      expect(menu.getByRole("menuitemradio", { name: /^List/ })).toHaveAttribute("aria-checked", "false");
      expect(menu.getByText("Under their parent")).toBeInTheDocument();
    });

    it("saves the chosen mode on the view", async () => {
      mocked.viewSetSubtasks.mockResolvedValue(view({ subtasks: "list" }));
      renderToolbar();
      await userEvent.click(subtasksButton());

      await userEvent.click(await screen.findByRole("menuitemradio", { name: /^List/ }));

      expect(mocked.viewSetSubtasks).toHaveBeenCalledWith(101, "list");
    });
  });

  it("withdraws every toolbar action when the toolbar unmounts", () => {
    renderToolbar();
    run("toolbar.me");

    cleanup();

    expect(["toolbar.me", "toolbar.search", "toolbar.filter", "toolbar.columns", "toolbar.group", "view.save", "view.revert"].map(run)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      false,
    ]);
  });
});

describe("Toolbar view drafts", () => {
  beforeEach(() => {
    mocked.filterOptions.mockResolvedValue(filterOptions({ statuses: [{ id: "3", name: "In Progress" }] }));
    mocked.listConditions.mockResolvedValue(listConditions({ kind: "space" }));
    mocked.viewSetOnlyMine.mockImplementation(async (id, on) => view({ id, onlyMine: on }));
  });

  describe("Me chip", () => {
    it("offers no Save View, no More menu and no view actions while nothing changed", () => {
      renderToolbar(SPACE_LEVEL);

      expect(saveViewButton()).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "More toolbar items" })).not.toBeInTheDocument();
      expect(run("view.save")).toBe(false);
      expect(run("view.revert")).toBe(false);
    });

    it("shows Save View once Me is toggled, without saving anything", async () => {
      renderToolbar(SPACE_LEVEL);

      await userEvent.click(meChip());

      expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
      expect(mocked.viewSetOnlyMine).not.toHaveBeenCalled();
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
    });

    it("hides Save View again when Me is toggled back to what the view saved", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(meChip());
      await screen.findByRole("button", { name: "Save View" });

      await userEvent.click(meChip());

      expect(saveViewButton()).not.toBeInTheDocument();
    });

    it("saves the drafted Me chip on the view and drops the Save View button", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetOnlyMine).toHaveBeenCalledWith(101, true));
      await waitFor(() => expect(saveViewButton()).not.toBeInTheDocument());
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
      expect(mocked.listSetFilter).not.toHaveBeenCalled();
    });

    it("shows a view that saved Me with the chip on and nothing to save", () => {
      renderToolbar(SPACE_LEVEL, 0, { onlyMine: true });

      expect(screen.getByRole("button", { name: "Only my issues, click to show all", pressed: true })).toBeInTheDocument();
      expect(saveViewButton()).not.toBeInTheDocument();
    });

    it("saves Me off on a view that had it on", async () => {
      renderToolbar(SPACE_LEVEL, 0, { onlyMine: true });
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetOnlyMine).toHaveBeenCalledWith(101, false));
    });

    it("keeps the draft and the Save View button when saving fails", async () => {
      mocked.viewSetOnlyMine.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.kind)).toContain("error"));
      expect(screen.getByRole("button", { name: "Save View" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Only my issues, click to show all", pressed: true })).toBeInTheDocument();
    });
  });

  describe("conditions", () => {
    it("shows Save View for a complete condition added in the Filter popover, without saving", async () => {
      renderToolbar(SPACE_LEVEL);

      await addCondition("This view");

      expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
    });

    it("offers no Save View for a condition that still lacks its value", async () => {
      mocked.filterOptions.mockResolvedValue(filterOptions());
      renderToolbar(SPACE_LEVEL);

      await addCondition("This view");

      expect(saveViewButton()).not.toBeInTheDocument();
    });

    it("counts the drafted condition in the Filter badge", async () => {
      renderToolbar(SPACE_LEVEL);
      expect(screen.queryByLabelText(/condition/)).not.toBeInTheDocument();

      await addCondition("This view");

      expect(await screen.findByLabelText("1 condition")).toBeInTheDocument();
    });

    it("drops a removed saved condition from the Filter badge", async () => {
      mocked.listConditions.mockResolvedValue(listConditions({ kind: "space", view: [NO_DUE_DATE] }));
      renderToolbar(SPACE_LEVEL, 1);
      expect(screen.getByLabelText("1 condition")).toBeInTheDocument();
      run("toolbar.filter");
      const dialog = await screen.findByRole("dialog", { name: "Filter" });

      await userEvent.click(await within(dialog).findByRole("button", { name: "Remove condition" }));
      await userEvent.click(within(dialog).getByRole("button", { name: "Done" }));

      expect(screen.queryByLabelText(/condition/)).not.toBeInTheDocument();
      expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
    });

    it("saves a view condition on the view only", async () => {
      renderToolbar(SPACE_LEVEL);
      await addCondition("This view");

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetFilter).toHaveBeenCalledWith(101, { conditions: [IN_PROGRESS] }));
      await waitFor(() => expect(saveViewButton()).not.toBeInTheDocument());
      expect(mocked.listSetFilter).not.toHaveBeenCalled();
      expect(mocked.viewSetOnlyMine).not.toHaveBeenCalled();
    });

    it("saves a condition of All views of this list on the list only", async () => {
      mocked.listConditions.mockResolvedValue(listConditions({ kind: "list" }));
      renderToolbar();
      await addCondition("All views of this list");

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.listSetFilter).toHaveBeenCalledWith(11, { conditions: [IN_PROGRESS] }));
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
    });

    it("saves the list, the view and the Me chip together when all three changed", async () => {
      mocked.listConditions.mockResolvedValue(listConditions({ kind: "list" }));
      renderToolbar();
      await addCondition("All views of this list");
      await addCondition("This view");
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetOnlyMine).toHaveBeenCalledWith(101, true));
      expect(mocked.listSetFilter).toHaveBeenCalledWith(11, { conditions: [IN_PROGRESS] });
      expect(mocked.viewSetFilter).toHaveBeenCalledWith(101, { conditions: [IN_PROGRESS] });
    });
  });

  describe("Revert View", () => {
    it("drops the Me draft from the More menu", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "More toolbar items" }));
      await userEvent.click(await screen.findByRole("menuitem", { name: "Revert View" }));

      expect(saveViewButton()).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Me", pressed: false })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "More toolbar items" })).not.toBeInTheDocument();
      expect(mocked.viewSetOnlyMine).not.toHaveBeenCalled();
    });

    it("drops a drafted condition from the badge", async () => {
      renderToolbar(SPACE_LEVEL);
      await addCondition("This view");
      await screen.findByLabelText("1 condition");

      await userEvent.click(screen.getByRole("button", { name: "More toolbar items" }));
      await userEvent.click(await screen.findByRole("menuitem", { name: "Revert View" }));

      expect(screen.queryByLabelText(/condition/)).not.toBeInTheDocument();
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
    });

    it("keeps the text filter", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.type(filterField(), "pricing");
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "More toolbar items" }));
      await userEvent.click(await screen.findByRole("menuitem", { name: "Revert View" }));

      expect(filterField()).toHaveValue("pricing");
    });
  });

  describe("view.save and view.revert", () => {
    it("saves from the view.save page action while the view has drafts", async () => {
      renderToolbar(SPACE_LEVEL);
      run("toolbar.me");
      await screen.findByRole("button", { name: "Save View" });

      expect(run("view.save")).toBe(true);

      await waitFor(() => expect(mocked.viewSetOnlyMine).toHaveBeenCalledWith(101, true));
    });

    it("saves on ⌘S", async () => {
      renderToolbar(SPACE_LEVEL);
      run("toolbar.me");
      await screen.findByRole("button", { name: "Save View" });

      await userEvent.keyboard("{Meta>}s{/Meta}");

      await waitFor(() => expect(mocked.viewSetOnlyMine).toHaveBeenCalledWith(101, true));
    });

    it("reverts from the view.revert page action while the view has drafts", async () => {
      renderToolbar(SPACE_LEVEL);
      run("toolbar.me");
      await screen.findByRole("button", { name: "Save View" });

      expect(run("view.revert")).toBe(true);

      expect(saveViewButton()).not.toBeInTheDocument();
    });

    it("withdraws both actions again once the drafts are gone", async () => {
      renderToolbar(SPACE_LEVEL);
      run("toolbar.me");
      await screen.findByRole("button", { name: "Save View" });
      run("view.revert");

      expect([run("view.save"), run("view.revert")]).toEqual([false, false]);
    });
  });
});
