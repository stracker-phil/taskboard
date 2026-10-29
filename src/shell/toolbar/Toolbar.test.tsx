import { act, cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { MenuHost } from "../../components/menu";
import { installCommands, useCommandKeys } from "../../commands";
import { runPageAction } from "../../commands/pageActions";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { space } from "../../test/spaces";
import { listNode, view } from "../../test/views";
import type { Level } from "../levels";
import { Toolbar } from "./Toolbar";

const mocked = vi.mocked(api, true);

const LIST_LEVEL: Level = { kind: "list", space: space(), node: listNode() };
const SPACE_LEVEL: Level = { kind: "space", space: space() };

function Harness({ level, ownConditions }: { level: Level; ownConditions: number }) {
  useCommandKeys();
  return <Toolbar level={level} view={view({ groupBy: "status", ownConditions })} />;
}

function renderToolbar(level: Level = LIST_LEVEL, ownConditions = 0) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness level={level} ownConditions={ownConditions} />
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
});

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

  it("withdraws every toolbar action when the toolbar unmounts", () => {
    renderToolbar();

    cleanup();

    expect(["toolbar.me", "toolbar.search", "toolbar.filter", "toolbar.columns", "toolbar.group"].map(run)).toEqual([false, false, false, false, false]);
  });
});
