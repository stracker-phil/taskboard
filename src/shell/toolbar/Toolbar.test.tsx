import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type MenuEntry } from "../../api";
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

function Harness({ level }: { level: Level }) {
  useCommandKeys();
  return <Toolbar level={level} view={view({ groupBy: "status" })} />;
}

function renderToolbar(level: Level = LIST_LEVEL) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness level={level} />
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

    it("is not offered on a Space level, which has no Filter button", () => {
      renderToolbar(SPACE_LEVEL);

      expect(run("toolbar.filter")).toBe(false);
      expect(screen.queryByRole("button", { name: "Filter" })).not.toBeInTheDocument();
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
    it("opens the group menu with the view's grouping checked", () => {
      renderToolbar();

      run("toolbar.group");

      const entries = mocked.popupMenu.mock.lastCall![0].filter((e: MenuEntry): e is Exclude<MenuEntry, "separator"> => e !== "separator");
      expect(entries.find((e) => e.checked)?.text).toBe("Status");
      expect(entries.map((e) => e.text)).toContain("Sort by");
    });
  });

  it("withdraws every toolbar action when the toolbar unmounts", () => {
    renderToolbar();

    cleanup();

    expect(["toolbar.me", "toolbar.search", "toolbar.filter", "toolbar.columns", "toolbar.group"].map(run)).toEqual([false, false, false, false, false]);
  });
});
