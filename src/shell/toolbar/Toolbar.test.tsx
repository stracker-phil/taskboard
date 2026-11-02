import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { MenuHost } from "../../components/menu";
import { installCommands, useCommandKeys } from "../../commands";
import { runPageAction } from "../../commands/pageActions";
import { paletteMenus } from "../../palette/menus";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import type { View } from "../../api";
import { requestNarrowing } from "../../queries/views";
import { narrowingKey, NO_VIEW_NARROWING } from "../../state/slices/narrowing";
import { filterOptions, listConditions } from "../../test/filters";
import { ALEX, SAM } from "../../test/issues";
import { behaviour, settings } from "../../test/settings";
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

/** Waits until the level's people have been asked for and delivered to the toolbar. */
async function peopleLoaded() {
  await vi.waitFor(() => expect(mocked.viewPeople).toHaveBeenCalled());
  await act(async () => {
    await mocked.viewPeople.mock.results[0]?.value;
  });
}

const filterField = () => screen.getByRole("textbox", { name: "Filter this view" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
  mocked.listMatchCount.mockResolvedValue({ open: 5, total: 9 });
  mocked.viewsOf.mockResolvedValue([view()]);
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

const meChip = () => screen.getByRole("button", { name: "Me" });
const revertViewButton = () => screen.queryByRole("button", { name: "Revert View" });
const moreButton = () => screen.queryByRole("button", { name: "More toolbar items" });

describe("Toolbar page actions", () => {
  describe("toolbar.me", () => {
    it("turns the Me narrowing on and off", () => {
      renderToolbar();
      expect(meChip()).toHaveAttribute("aria-pressed", "false");

      run("toolbar.me");
      expect(meChip()).toHaveAttribute("aria-pressed", "true");

      run("toolbar.me");
      expect(meChip()).toHaveAttribute("aria-pressed", "false");
    });

    it("toggles on ⌘⇧M", async () => {
      renderToolbar();

      await userEvent.keyboard("{Meta>}{Shift>}m{/Shift}{/Meta}");

      expect(meChip()).toHaveAttribute("aria-pressed", "true");
    });

    it("keeps one button named Me while on, and toggles back when clicked at the same spot", async () => {
      renderToolbar();

      await userEvent.click(meChip());
      expect(meChip()).toHaveAttribute("title", "Only my issues (⌘⇧M)");

      await userEvent.click(meChip());
      expect(meChip()).toHaveAttribute("aria-pressed", "false");
      expect(meChip()).toHaveAttribute("title", "Only my issues (⌘⇧M)");
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

    it("offers no Space or Client grouping on a Space level", async () => {
      renderToolbar(SPACE_LEVEL);

      run("toolbar.group");

      const menu = within(await screen.findByRole("menu", { name: "Group" }));
      expect(menu.getByRole("menuitemradio", { name: "Assignee" })).toBeInTheDocument();
      expect(menu.queryByRole("menuitemradio", { name: "Space" })).not.toBeInTheDocument();
      expect(menu.queryByRole("menuitemradio", { name: "Client" })).not.toBeInTheDocument();
    });

    it("offers the Space and Client groupings on Everything", async () => {
      renderToolbar({ kind: "home", node: listNode({ id: 1, kind: "everything", name: "Everything" }) });

      run("toolbar.group");

      const menu = within(await screen.findByRole("menu", { name: "Group" }));
      expect(menu.getByRole("menuitemradio", { name: "Space" })).toBeInTheDocument();
      expect(menu.getByRole("menuitemradio", { name: "Client" })).toBeInTheDocument();
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
    const BUTTON_NAME = /^(Subtasks|As items)/;
    const subtasksButton = () => screen.getByRole("button", { name: BUTTON_NAME });

    it("sits right after the Columns button", () => {
      renderToolbar();

      expect(subtasksButton().previousElementSibling).toContainElement(screen.getByRole("button", { name: /^Columns/ }));
    });

    it("is absent on a Board view", () => {
      renderToolbar(LIST_LEVEL, 0, { layout: "board" });

      expect(screen.queryByRole("button", { name: BUTTON_NAME })).not.toBeInTheDocument();
    });

    it("shows only its icon, labeled Subtasks: Hidden, while subtasks are hidden", () => {
      renderToolbar(LIST_LEVEL, 0, { subtasks: "hidden" });

      const button = screen.getByRole("button", { name: "Subtasks: Hidden" });
      expect(button).toHaveTextContent("");
      expect(button).toHaveAttribute("title", "Subtasks");
      expect(button.classList.contains("toolbar-btn--icon")).toBe(true);
    });

    it.each([
      ["hierarchy", "Subtasks"],
      ["list", "As items"],
    ] as const)("labels the button by the mode when subtasks show (%s)", (subtasks, label) => {
      renderToolbar(LIST_LEVEL, 0, { subtasks });

      expect(screen.getByRole("button", { name: label })).toHaveTextContent(label);
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
      expect(menu.getByRole("menuitemradio", { name: /^Hide subtasks/ })).toHaveAttribute("aria-checked", "false");
      expect(menu.getByRole("menuitemradio", { name: /^Show under parent/ })).toHaveAttribute("aria-checked", "true");
      expect(menu.getByRole("menuitemradio", { name: /^Show as items/ })).toHaveAttribute("aria-checked", "false");
    });

    it.each([
      ["Matching issues only"],
      ["Indented below their parent"],
      ["Separate rows, grouped on their own"],
    ])("explains each mode with the hint %s", async (hint) => {
      renderToolbar();

      await userEvent.click(subtasksButton());

      expect(within(await screen.findByRole("menu", { name: "Subtasks" })).getByText(hint)).toBeInTheDocument();
    });

    it.each([
      ["Hide subtasks", "hidden"],
      ["Show under parent", "hierarchy"],
      ["Show as items", "list"],
    ] as const)("saves the mode on the view when %s is chosen", async (entry, mode) => {
      mocked.viewSetSubtasks.mockResolvedValue(view({ subtasks: mode }));
      renderToolbar(LIST_LEVEL, 0, { subtasks: mode === "hidden" ? "list" : "hidden" });
      await userEvent.click(subtasksButton());

      await userEvent.click(await screen.findByRole("menuitemradio", { name: new RegExp(`^${entry}`) }));

      expect(mocked.viewSetSubtasks).toHaveBeenCalledWith(101, mode);
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
    it("offers no Save View, no Revert View, no More menu and no view actions while nothing changed", () => {
      renderToolbar(SPACE_LEVEL);

      expect(saveViewButton()).not.toBeInTheDocument();
      expect(revertViewButton()).not.toBeInTheDocument();
      expect(moreButton()).not.toBeInTheDocument();
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

    it("shows Revert View beside Save View, and no More menu, while the view is dirty", async () => {
      renderToolbar(SPACE_LEVEL);

      await userEvent.click(meChip());

      expect(await screen.findByRole("button", { name: "Revert View" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Save View" })).toBeInTheDocument();
      expect(moreButton()).not.toBeInTheDocument();
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

      expect(meChip()).toHaveAttribute("aria-pressed", "true");
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
      expect(meChip()).toHaveAttribute("aria-pressed", "true");
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
    it("drops the Me draft and both draft buttons", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "Revert View" }));

      expect(saveViewButton()).not.toBeInTheDocument();
      expect(revertViewButton()).not.toBeInTheDocument();
      expect(meChip()).toHaveAttribute("aria-pressed", "false");
      expect(mocked.viewSetOnlyMine).not.toHaveBeenCalled();
    });

    it("drops a drafted condition from the badge", async () => {
      renderToolbar(SPACE_LEVEL);
      await addCondition("This view");
      await screen.findByLabelText("1 condition");

      await userEvent.click(await screen.findByRole("button", { name: "Revert View" }));

      expect(screen.queryByLabelText(/condition/)).not.toBeInTheDocument();
      expect(mocked.viewSetFilter).not.toHaveBeenCalled();
    });

    it("keeps the text filter", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.type(filterField(), "pricing");
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "Revert View" }));

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

describe("Toolbar person chip", () => {
  const personChip = () => screen.getByRole("button", { name: /^(Person|Alex|Sam)/ });
  const personMenu = async () => within(await screen.findByRole("menu", { name: "Person" }));
  const narrowedTo = () => useStore.getState().narrowings[narrowingKey(11, 101)]?.person ?? null;

  async function pick(name: RegExp | string) {
    await peopleLoaded();
    await userEvent.click(personChip());
    await userEvent.click((await personMenu()).getByRole("menuitemradio", { name }));
  }

  beforeEach(() => {
    mocked.viewPeople.mockResolvedValue([ALEX, SAM]);
    mocked.listConditions.mockResolvedValue(listConditions({ kind: "space" }));
    mocked.viewSetPerson.mockImplementation(async (id, person) => view({ id, person }));
  });

  it("shows an unpressed Person chip next to Me while nobody is picked", () => {
    renderToolbar(SPACE_LEVEL);

    expect(personChip()).toHaveTextContent("Person");
    expect(personChip()).toHaveAttribute("aria-pressed", "false");
    expect(meChip().compareDocumentPosition(personChip()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("lists Anyone and the view's people under a caption, with Anyone checked", async () => {
    renderToolbar(SPACE_LEVEL);
    await peopleLoaded();

    await userEvent.click(personChip());

    const menu = await personMenu();
    expect(screen.getByText("Issues assigned to")).toBeInTheDocument();
    expect(menu.getByRole("menuitemradio", { name: "Anyone" })).toHaveAttribute("aria-checked", "true");
    expect(menu.getByRole("menuitemradio", { name: /Alex Morgan/ })).toHaveAttribute("aria-checked", "false");
    expect(menu.getByRole("menuitemradio", { name: /Sam Lee/ })).toHaveAttribute("aria-checked", "false");
    expect(mocked.viewPeople).toHaveBeenCalledWith(11, 101, expect.objectContaining({ text: "", epic: null }));
  });

  it("asks for the people again with the view's text filter, so the list follows what the view shows", async () => {
    renderToolbar(SPACE_LEVEL);
    await peopleLoaded();

    await userEvent.type(filterField(), "pricing");

    await vi.waitFor(() => expect(mocked.viewPeople).toHaveBeenCalledWith(11, 101, expect.objectContaining({ text: "pricing" })));
  });

  it("says there are no teammates yet, in a disabled row, when the view has none", async () => {
    mocked.viewPeople.mockResolvedValue([]);
    renderToolbar(SPACE_LEVEL);

    await userEvent.click(personChip());

    expect((await personMenu()).getByRole("menuitem", { name: "No teammates yet" })).toHaveAttribute("aria-disabled", "true");
  });

  it("narrows the view to the picked person and shows their first name on a pressed chip", async () => {
    renderToolbar(SPACE_LEVEL);

    await pick(/Sam Lee/);

    expect(narrowedTo()).toBe("demo-account-2");
    expect(personChip()).toHaveTextContent("Sam");
    expect(personChip()).not.toHaveTextContent("Lee");
    expect(personChip()).toHaveAttribute("aria-pressed", "true");
  });

  it("checks the picked person in the menu", async () => {
    renderToolbar(SPACE_LEVEL);
    await pick(/Sam Lee/);

    await userEvent.click(personChip());

    expect((await personMenu()).getByRole("menuitemradio", { name: /Sam Lee/ })).toHaveAttribute("aria-checked", "true");
  });

  it("clears the narrowing when Anyone is chosen", async () => {
    renderToolbar(SPACE_LEVEL);
    await pick(/Sam Lee/);

    await pick("Anyone");

    expect(narrowedTo()).toBeNull();
    expect(personChip()).toHaveTextContent("Person");
    expect(personChip()).toHaveAttribute("aria-pressed", "false");
  });

  it("turns the Me chip off when a person is picked", async () => {
    renderToolbar(SPACE_LEVEL);
    await userEvent.click(meChip());
    expect(meChip()).toHaveAttribute("aria-pressed", "true");

    await pick(/Alex Morgan/);

    expect(meChip()).toHaveAttribute("aria-pressed", "false");
    expect(personChip()).toHaveAttribute("aria-pressed", "true");
  });

  it("clears the person when Me is turned on", async () => {
    renderToolbar(SPACE_LEVEL);
    await pick(/Sam Lee/);

    await userEvent.click(meChip());

    expect(meChip()).toHaveAttribute("aria-pressed", "true");
    expect(personChip()).toHaveTextContent("Person");
    expect(personChip()).toHaveAttribute("aria-pressed", "false");
  });

  it("offers Save View and Revert View for a picked person, without saving anything yet", async () => {
    renderToolbar(SPACE_LEVEL);

    await pick(/Sam Lee/);

    expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
    expect(revertViewButton()).toBeInTheDocument();
    expect(mocked.viewSetPerson).not.toHaveBeenCalled();
    expect(mocked.viewSetOnlyMine).not.toHaveBeenCalled();
  });

  it("saves the picked person on the view and drops the Save View button", async () => {
    renderToolbar(SPACE_LEVEL);
    await pick(/Sam Lee/);

    await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

    await waitFor(() => expect(mocked.viewSetPerson).toHaveBeenCalledWith(101, "demo-account-2"));
    await waitFor(() => expect(saveViewButton()).not.toBeInTheDocument());
    expect(mocked.viewSetOnlyMine).not.toHaveBeenCalled();
  });

  it("drops the picked person with Revert View", async () => {
    renderToolbar(SPACE_LEVEL);
    await pick(/Sam Lee/);

    await userEvent.click(await screen.findByRole("button", { name: "Revert View" }));

    expect(personChip()).toHaveTextContent("Person");
    expect(personChip()).toHaveAttribute("aria-pressed", "false");
    expect(saveViewButton()).not.toBeInTheDocument();
    expect(mocked.viewSetPerson).not.toHaveBeenCalled();
  });

  it("hides Save View again when the person is changed back to what the view saved", async () => {
    renderToolbar(SPACE_LEVEL);
    await pick(/Sam Lee/);
    await screen.findByRole("button", { name: "Save View" });

    await pick("Anyone");

    expect(saveViewButton()).not.toBeInTheDocument();
  });

  it("keeps the draft and the Save View button when saving the person fails", async () => {
    mocked.viewSetPerson.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    renderToolbar(SPACE_LEVEL);
    await pick(/Sam Lee/);

    await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.kind)).toContain("error"));
    expect(screen.getByRole("button", { name: "Save View" })).toBeInTheDocument();
    expect(personChip()).toHaveTextContent("Sam");
  });

  describe("on a view that saved a person", () => {
    beforeEach(() => {
      mocked.viewsOf.mockResolvedValue([view({ person: "demo-account-2" })]);
    });

    it("shows the saved person on a pressed chip with nothing to save", async () => {
      renderToolbar(SPACE_LEVEL, 0, { person: "demo-account-2" });

      await waitFor(() => expect(personChip()).toHaveTextContent("Sam"));
      expect(personChip()).toHaveAttribute("aria-pressed", "true");
      expect(saveViewButton()).not.toBeInTheDocument();
    });

    it("saves Anyone, as a null account, when Anyone is chosen", async () => {
      renderToolbar(SPACE_LEVEL, 0, { person: "demo-account-2" });
      await waitFor(() => expect(personChip()).toHaveTextContent("Sam"));

      await pick("Anyone");
      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetPerson).toHaveBeenCalledWith(101, null));
    });

    it("turns the saved person off when Me is turned on, and saves Me", async () => {
      renderToolbar(SPACE_LEVEL, 0, { person: "demo-account-2" });
      await waitFor(() => expect(personChip()).toHaveTextContent("Sam"));

      await userEvent.click(meChip());

      expect(meChip()).toHaveAttribute("aria-pressed", "true");
      expect(personChip()).toHaveAttribute("aria-pressed", "false");

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetOnlyMine).toHaveBeenCalledWith(101, true));
      await waitFor(() => expect(mocked.viewSetPerson).toHaveBeenCalledWith(101, null));
    });
  });
});

describe("Toolbar Me mode", () => {
  const optionsButton = () => screen.getByRole("button", { name: "Me mode options" });
  const popover = async () => within(await screen.findByRole("dialog", { name: "Me mode" }));
  const narrowedMe = () => useStore.getState().narrowings[narrowingKey(11, 101)]?.me;

  /** Renders with Me on in every view; waits until the chip shows it. */
  async function renderEverywhere() {
    mocked.modesGet.mockResolvedValue({ meEverywhere: true, unreadOnly: false, bookmarksBar: false });
    renderToolbar(SPACE_LEVEL);
    await waitFor(() => expect(meChip()).toHaveAttribute("aria-pressed", "true"));
  }

  beforeEach(() => {
    mocked.getSettings.mockResolvedValue(settings());
    mocked.meUserFields.mockResolvedValue([{ field: "reporter", name: "Reporter" }]);
    mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: false, bookmarksBar: false });
    mocked.viewPeople.mockResolvedValue([ALEX, SAM]);
    mocked.listConditions.mockResolvedValue(listConditions({ kind: "space" }));
  });

  describe("options button", () => {
    it("opens the Me mode popover and reports it as expanded", async () => {
      renderToolbar(SPACE_LEVEL);
      expect(optionsButton()).toHaveAttribute("aria-expanded", "false");

      await userEvent.click(optionsButton());

      expect(await screen.findByRole("dialog", { name: "Me mode" })).toBeInTheDocument();
      expect(optionsButton()).toHaveAttribute("aria-expanded", "true");
    });

    it("opens the popover from the toolbar.meOptions page action", async () => {
      renderToolbar(SPACE_LEVEL);

      run("toolbar.meOptions");

      expect(await screen.findByRole("dialog", { name: "Me mode" })).toBeInTheDocument();
    });

    it("does not turn Me on", async () => {
      renderToolbar(SPACE_LEVEL);

      await userEvent.click(optionsButton());
      await screen.findByRole("dialog", { name: "Me mode" });

      expect(meChip()).toHaveAttribute("aria-pressed", "false");
    });
  });

  describe("rules", () => {
    it("lists the saved rules with their switch positions", async () => {
      mocked.getSettings.mockResolvedValue(
        settings({
          behaviour: behaviour({
            meRules: [
              { rule: { kind: "field", field: "reporter" }, on: true },
              { rule: { kind: "mentioned" }, on: false },
            ],
          }),
        }),
      );
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(optionsButton());

      const dialog = await popover();

      expect(await dialog.findByRole("button", { name: "Reporter" })).toHaveAttribute("aria-pressed", "true");
      expect(dialog.getByRole("button", { name: "Mentioned in a comment" })).toHaveAttribute("aria-pressed", "false");
    });

    it("sends an app-wide switch edit for the clicked rule while Me holds in every view", async () => {
      await renderEverywhere();
      await userEvent.click(optionsButton());

      await userEvent.click(await (await popover()).findByRole("button", { name: "Subtasks of my issues" }));

      await waitFor(() => expect(mocked.meRuleEdit).toHaveBeenCalledWith({ kind: "switch", rule: { kind: "subtasks" }, on: true }));
    });

    it("shows an error toast when an app-wide rule cannot be saved", async () => {
      mocked.meRuleEdit.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderEverywhere();
      await userEvent.click(optionsButton());

      await userEvent.click(await (await popover()).findByRole("button", { name: "Mentioned in a comment" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.kind)).toContain("error"));
    });

    it("opens the Me Mode settings from Fields…", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(optionsButton());

      await userEvent.click((await popover()).getByRole("button", { name: "Fields…" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:general:meMode");
    });
  });

  describe("rules in the view's own Me mode", () => {
    const MENTIONED = { kind: "mentioned" } as const;
    const REPORTER = { kind: "field", field: "reporter" } as const;
    const mentionedRow = async () => (await popover()).findByRole("button", { name: "Mentioned in a comment" });

    /** Renders a view that saved Me on with the given rules (null follows the app's). */
    async function renderSavedMe(meRules: (typeof MENTIONED | typeof REPORTER)[] | null) {
      mocked.viewsOf.mockResolvedValue([view({ onlyMine: true, meRules })]);
      renderToolbar(SPACE_LEVEL, 0, { onlyMine: true, meRules });
      await waitFor(() => expect(meChip()).toHaveAttribute("aria-pressed", "true"));
      await userEvent.click(optionsButton());
    }

    it("drafts a toggled rule on the view instead of editing the app's rules", async () => {
      await renderSavedMe(null);

      await userEvent.click(await mentionedRow());

      expect(await mentionedRow()).toHaveAttribute("aria-pressed", "true");
      expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
      expect(revertViewButton()).toBeInTheDocument();
      expect(mocked.meRuleEdit).not.toHaveBeenCalled();
    });

    it("asks for the rows with the rules that are on", async () => {
      await renderSavedMe(null);

      await userEvent.click(await mentionedRow());

      await waitFor(() => expect(mocked.viewPeople).toHaveBeenLastCalledWith(11, 101, expect.objectContaining({ me: true, meOn: [MENTIONED] })));
    });

    it("saves the rules that are on on the view with Save View", async () => {
      await renderSavedMe(null);
      await userEvent.click(await mentionedRow());

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetMeRules).toHaveBeenCalledWith(101, [MENTIONED]));
      await waitFor(() => expect(saveViewButton()).not.toBeInTheDocument());
      expect(mocked.meRuleEdit).not.toHaveBeenCalled();
    });

    it("drops the drafted rules with Revert View", async () => {
      await renderSavedMe(null);
      await userEvent.click(await mentionedRow());

      await userEvent.click(await screen.findByRole("button", { name: "Revert View" }));

      expect(saveViewButton()).not.toBeInTheDocument();
      expect(mocked.viewSetMeRules).not.toHaveBeenCalled();
      expect(useStore.getState().narrowings[narrowingKey(11, 101)]?.meRules).toBeUndefined();
    });

    it("shows the rules the view saved, whatever the app's switches are", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ meRules: [{ rule: REPORTER, on: true }, { rule: MENTIONED, on: false }] }) }));
      await renderSavedMe([MENTIONED]);

      expect(await mentionedRow()).toHaveAttribute("aria-pressed", "true");
      expect((await popover()).getByRole("button", { name: "Reporter" })).toHaveAttribute("aria-pressed", "false");
      expect(saveViewButton()).not.toBeInTheDocument();
    });

    it("saves the app's rules that are on when Me was just turned on with no rule drafted", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ meRules: [{ rule: REPORTER, on: true }, { rule: MENTIONED, on: false }] }) }));
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(meChip());

      await userEvent.click(await screen.findByRole("button", { name: "Save View" }));

      await waitFor(() => expect(mocked.viewSetOnlyMine).toHaveBeenCalledWith(101, true));
      await waitFor(() => expect(mocked.viewSetMeRules).toHaveBeenCalledWith(101, [REPORTER]));
    });

    it("keeps another view's saved rules and the app's rules when Me is turned on and a rule switched in this view", async () => {
      const viewA = view({ id: 101, name: "Mine", onlyMine: true, meRules: [MENTIONED] });
      mocked.viewsOf.mockResolvedValue([viewA, view({ id: 102, name: "Board" })]);
      renderToolbar(SPACE_LEVEL, 0, { id: 102, name: "Board" });
      await userEvent.click(meChip());
      await userEvent.click(optionsButton());

      await userEvent.click((await popover()).getByRole("button", { name: "Reporter" }));

      expect(await screen.findByRole("button", { name: "Save View" })).toBeInTheDocument();
      expect(mocked.meRuleEdit).not.toHaveBeenCalled();
      const narrowings = useStore.getState().narrowings;
      expect(requestNarrowing(narrowings[narrowingKey(11, 101)] ?? NO_VIEW_NARROWING, viewA).meOn).toEqual([MENTIONED]);
      expect(requestNarrowing(narrowings[narrowingKey(11, 102)] ?? NO_VIEW_NARROWING, view({ id: 102 })).meOn).toEqual([REPORTER]);
    });
  });

  describe("applies to", () => {
    it("starts with Only this view pressed", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(optionsButton());

      expect((await popover()).getByRole("button", { name: /^Only this view/ })).toHaveAttribute("aria-pressed", "true");
    });

    it("turns Me mode on in every view when All views is chosen", async () => {
      renderToolbar(SPACE_LEVEL);
      await userEvent.click(optionsButton());

      await userEvent.click((await popover()).getByRole("button", { name: /^All views/ }));

      expect(mocked.modeSet).toHaveBeenCalledWith("meEverywhere", true);
    });

    it("marks All views as pressed while Me mode holds everywhere", async () => {
      await renderEverywhere();

      await userEvent.click(optionsButton());

      expect((await popover()).getByRole("button", { name: /^All views/ })).toHaveAttribute("aria-pressed", "true");
    });

    it("keeps Me on in this view only when Only this view is chosen while everywhere", async () => {
      await renderEverywhere();
      await userEvent.click(optionsButton());

      await userEvent.click((await popover()).getByRole("button", { name: /^Only this view/ }));

      expect(mocked.modeSet).toHaveBeenCalledWith("meEverywhere", false);
      expect(narrowedMe()).toBe(true);
    });
  });

  describe("while Me mode holds in every view", () => {
    it("shows the chip pressed with the every-view tooltip", async () => {
      await renderEverywhere();

      expect(meChip()).toHaveAttribute("title", "Only my issues, in every view (⌘⇧M)");
    });

    it("turns Me mode off everywhere when the chip is clicked", async () => {
      await renderEverywhere();

      await userEvent.click(meChip());

      expect(mocked.modeSet).toHaveBeenCalledWith("meEverywhere", false);
      expect(narrowedMe()).toBe(false);
    });

    it("turns Me mode off everywhere when a person is picked", async () => {
      await renderEverywhere();
      await peopleLoaded();

      await userEvent.click(screen.getByRole("button", { name: /^Person/ }));
      await userEvent.click(within(await screen.findByRole("menu", { name: "Person" })).getByRole("menuitemradio", { name: /Sam Lee/ }));

      expect(mocked.modeSet).toHaveBeenCalledWith("meEverywhere", false);
    });
  });

  describe("when Me mode is only this view's", () => {
    it("leaves where Me applies alone when the chip is toggled", async () => {
      renderToolbar(SPACE_LEVEL);

      await userEvent.click(meChip());
      await userEvent.click(meChip());

      expect(mocked.modeSet).not.toHaveBeenCalled();
    });
  });
});

describe("Toolbar folding", () => {
  const rects: { mockRestore: () => void }[] = [];

  /** A bar of `width` px whose items are each 100px wide; the observer reports the size at once. */
  function narrowBar(width: number) {
    vi.stubGlobal(
      "ResizeObserver",
      class {
        callback: ResizeObserverCallback;
        constructor(callback: ResizeObserverCallback) {
          this.callback = callback;
        }
        observe() {
          this.callback([], this);
        }
        unobserve() {}
        disconnect() {}
      },
    );
    rects.push(vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(width));
    rects.push(vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ width: 100, height: 20, top: 0, left: 0, right: 100, bottom: 20, x: 0, y: 0, toJSON: () => ({}) }));
  }

  afterEach(() => {
    vi.unstubAllGlobals();
    rects.splice(0).forEach((spy) => spy.mockRestore());
  });

  it("shows the More button only once items fold, and lists them", async () => {
    narrowBar(500);
    renderToolbar(SPACE_LEVEL);

    await userEvent.click(await screen.findByRole("button", { name: "More toolbar items" }));

    const menu = within(await screen.findByRole("menu", { name: "More toolbar items" }));
    expect(menu.getByRole("menuitem", { name: /^Columns/ })).toBeInTheDocument();
    expect(menu.getByRole("menuitem", { name: /^Filter/ })).toBeInTheDocument();
  });

  describe("with the Me chip folded away", () => {
    const openMore = async () => {
      narrowBar(310);
      mocked.viewPeople.mockResolvedValue([ALEX, SAM]);
      renderToolbar(SPACE_LEVEL);
      await peopleLoaded();
      await userEvent.click(await screen.findByRole("button", { name: "More toolbar items" }));
      return within(await screen.findByRole("menu", { name: "More toolbar items" }));
    };

    it("offers Only my issues and Assigned to… in the More menu", async () => {
      const menu = await openMore();

      expect(menu.getByRole("menuitemcheckbox", { name: "Only my issues" })).toHaveAttribute("aria-checked", "false");
      expect(menu.getByRole("menuitem", { name: "Assigned to…" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Me" })).not.toBeInTheDocument();
    });

    it("opens the Me mode popover from Me Mode Options…", async () => {
      const menu = await openMore();

      await userEvent.click(menu.getByRole("menuitem", { name: "Me Mode Options…" }));

      expect(await screen.findByRole("dialog", { name: "Me mode" })).toBeInTheDocument();
    });

    it("opens the people menu from Assigned to… and narrows the view to the chosen person", async () => {
      const menu = await openMore();

      await userEvent.click(menu.getByRole("menuitem", { name: "Assigned to…" }));
      await userEvent.click(within(await screen.findByRole("menu", { name: "Person" })).getByRole("menuitemradio", { name: /Sam Lee/ }));

      expect(useStore.getState().narrowings[narrowingKey(11, 101)]?.person).toBe("demo-account-2");
    });
  });

  it("does not add a More button for a draft, nor Revert View to its menu", async () => {
    narrowBar(500);
    renderToolbar(SPACE_LEVEL);
    await userEvent.click(meChip());
    await screen.findByRole("button", { name: "Revert View" });

    await userEvent.click(screen.getByRole("button", { name: "More toolbar items" }));

    expect(within(await screen.findByRole("menu", { name: "More toolbar items" })).queryByRole("menuitem", { name: "Revert View" })).not.toBeInTheDocument();
  });

  it("folds the same items whether or not the view is dirty", async () => {
    narrowBar(500);
    renderToolbar(SPACE_LEVEL);
    const before = screen.queryAllByRole("button").map((b) => b.getAttribute("aria-label") ?? b.textContent);

    await userEvent.click(meChip());
    await screen.findByRole("button", { name: "Save View" });

    const after = screen.queryAllByRole("button").map((b) => b.getAttribute("aria-label") ?? b.textContent);
    expect(after.filter((name) => name !== "Revert View" && name !== "Save View")).toEqual(before);
  });
});

describe("Toolbar Unread only", () => {
  const unreadButton = () => screen.getByRole("button", { name: "Unread only" });

  beforeEach(() => {
    mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: false, bookmarksBar: false });
    mocked.modeSet.mockResolvedValue({ meEverywhere: false, unreadOnly: true, bookmarksBar: false });
  });

  it("sits after the Filter button and before the Me chip", async () => {
    renderToolbar(SPACE_LEVEL);

    expect(screen.getByRole("button", { name: /^Filter/ }).compareDocumentPosition(unreadButton()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(unreadButton().compareDocumentPosition(meChip()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("turns Unread only on when clicked while it is off", async () => {
    renderToolbar(SPACE_LEVEL);
    expect(unreadButton()).toHaveAttribute("aria-pressed", "false");

    await userEvent.click(unreadButton());

    expect(mocked.modeSet).toHaveBeenCalledWith("unreadOnly", true);
  });

  it("shows pressed while Unread only is on, and turns it off when clicked", async () => {
    mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: true, bookmarksBar: false });
    renderToolbar(SPACE_LEVEL);
    await waitFor(() => expect(unreadButton()).toHaveAttribute("aria-pressed", "true"));

    await userEvent.click(unreadButton());

    expect(mocked.modeSet).toHaveBeenCalledWith("unreadOnly", false);
  });

  it("turns Unread only on through the toolbar.unread page action", async () => {
    renderToolbar(SPACE_LEVEL);

    run("toolbar.unread");

    await waitFor(() => expect(mocked.modeSet).toHaveBeenCalledWith("unreadOnly", true));
  });

  it("turns Unread only on with ⌘⇧U", async () => {
    renderToolbar(SPACE_LEVEL);

    await userEvent.keyboard("{Meta>}{Shift>}u{/Shift}{/Meta}");

    await waitFor(() => expect(mocked.modeSet).toHaveBeenCalledWith("unreadOnly", true));
  });

  it("shows an error toast when the mode cannot be changed", async () => {
    mocked.modeSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    renderToolbar(SPACE_LEVEL);

    await userEvent.click(unreadButton());

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change Unread only: Disk full"));
  });

  describe("folded into the More menu", () => {
    const rects: { mockRestore: () => void }[] = [];

    beforeEach(() => {
      vi.stubGlobal(
        "ResizeObserver",
        class {
          callback: ResizeObserverCallback;
          constructor(callback: ResizeObserverCallback) {
            this.callback = callback;
          }
          observe() {
            this.callback([], this);
          }
          unobserve() {}
          disconnect() {}
        },
      );
      rects.push(vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(310));
      rects.push(vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ width: 100, height: 20, top: 0, left: 0, right: 100, bottom: 20, x: 0, y: 0, toJSON: () => ({}) }));
    });

    afterEach(() => {
      vi.unstubAllGlobals();
      rects.splice(0).forEach((spy) => spy.mockRestore());
    });

    const openMore = async () => {
      renderToolbar(SPACE_LEVEL);
      await vi.waitFor(() => expect(mocked.modesGet).toHaveBeenCalled());
      await act(async () => {
        await mocked.modesGet.mock.results[0]?.value;
      });
      await userEvent.click(await screen.findByRole("button", { name: "More toolbar items" }));
      return within(await screen.findByRole("menu", { name: "More toolbar items" }));
    };

    it("offers an unchecked Unread only toggle in place of the button", async () => {
      const menu = await openMore();

      expect(menu.getByRole("menuitemcheckbox", { name: "Unread only" })).toHaveAttribute("aria-checked", "false");
      expect(screen.queryByRole("button", { name: "Unread only" })).not.toBeInTheDocument();
    });

    it("shows the toggle checked when Unread only is on, and turns it off when chosen", async () => {
      mocked.modesGet.mockResolvedValue({ meEverywhere: false, unreadOnly: true, bookmarksBar: false });
      const menu = await openMore();
      expect(menu.getByRole("menuitemcheckbox", { name: "Unread only" })).toHaveAttribute("aria-checked", "true");

      await userEvent.click(menu.getByRole("menuitemcheckbox", { name: "Unread only" }));

      expect(mocked.modeSet).toHaveBeenCalledWith("unreadOnly", false);
    });
  });
});

describe("Toolbar palette entries", () => {
  const offered = (title: string) => paletteMenus().find((menu) => menu.title === title);
  const texts = (title: string) => (offered(title)?.entries() ?? []).flatMap((entry) => (typeof entry === "string" || "caption" in entry ? [] : [entry.text]));

  it("offers the Group menu under “This view”, with the current grouping checked", () => {
    renderToolbar();

    expect(offered("Group")?.section).toBe("This view");
    expect(texts("Group")).toEqual(expect.arrayContaining(["Status", "Epic", "Sort by"]));
    const status = offered("Group")!.entries().find((entry) => typeof entry !== "string" && "text" in entry && entry.text === "Status");
    expect(status).toMatchObject({ checked: true });
  });

  it("offers wrapping titles only on a list", () => {
    renderToolbar(LIST_LEVEL, 0, { layout: "list" });
    expect(texts("Show")).toEqual(["Labels inline", "Wrap long titles"]);

    cleanup();
    renderToolbar(LIST_LEVEL, 0, { layout: "board" });
    expect(texts("Show")).toEqual(["Labels inline"]);
  });

  it("withdraws its entries when the toolbar unmounts", () => {
    renderToolbar();

    cleanup();

    expect(offered("Group")).toBeUndefined();
    expect(offered("Show")).toBeUndefined();
  });
});
