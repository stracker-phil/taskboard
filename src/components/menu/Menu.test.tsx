import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { MouseEvent } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetStore } from "../../state/store";
import { MenuHost, openContextMenu, openMenu, type MenuAt, type MenuEntry } from ".";

function show(entries: MenuEntry[], at: MenuAt = { x: 20, y: 30 }, label?: string) {
  act(() => openMenu(entries, at, label));
}

function Anchored() {
  return (
    <>
      <button type="button">Actions</button>
      <MenuHost />
    </>
  );
}

const anchorButton = () => screen.getByRole("button", { name: "Actions" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("Menu", () => {
  describe("opening", () => {
    it("shows a menu named by its label with a row per item, at a point", () => {
      render(<MenuHost />);

      show([{ text: "Open" }, { text: "Copy link" }], { x: 40, y: 50 }, "Task actions");

      const menu = screen.getByRole("menu", { name: "Task actions" });
      expect(within(menu).getAllByRole("menuitem").map((row) => row.textContent)).toEqual(["Open", "Copy link"]);
    });

    it("names the menu 'Menu' when no label is given", () => {
      render(<MenuHost />);

      show([{ text: "Open" }]);

      expect(screen.getByRole("menu", { name: "Menu" })).toBeInTheDocument();
    });

    it("shows nothing until a menu is opened", () => {
      render(<MenuHost />);

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("opens under an anchor element with the first row focused", () => {
      render(<Anchored />);

      show([{ text: "Open" }, { text: "Copy link" }], anchorButton(), "Actions menu");

      expect(screen.getByRole("menuitem", { name: "Open" })).toHaveFocus();
    });

    it("skips a disabled first row when focusing the first row", () => {
      render(<Anchored />);

      show([{ text: "Open", enabled: false }, { text: "Copy link" }], anchorButton());

      expect(screen.getByRole("menuitem", { name: "Copy link" })).toHaveFocus();
    });

    it("closes when opened again on the same anchor element", () => {
      render(<Anchored />);
      show([{ text: "Open" }], anchorButton());

      show([{ text: "Open" }], anchorButton());

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("opens a context menu at the mouse and keeps the browser's own menu shut", () => {
      function Target() {
        return (
          <div data-testid="target" onContextMenu={(e: MouseEvent) => openContextMenu(e, [{ text: "Rename" }], "Row actions")}>
            Row
          </div>
        );
      }
      render(
        <>
          <Target />
          <MenuHost />
        </>,
      );

      const notPrevented = fireEvent.contextMenu(screen.getByTestId("target"), { clientX: 12, clientY: 18 });

      expect(notPrevented).toBe(false);
      expect(screen.getByRole("menu", { name: "Row actions" })).toBeInTheDocument();
    });
  });

  describe("choosing", () => {
    it("runs a clicked row and closes the menu", async () => {
      const run = vi.fn();
      render(<MenuHost />);
      show([{ text: "Open", run }, { text: "Copy link" }]);

      await userEvent.click(screen.getByRole("menuitem", { name: "Open" }));

      expect(run).toHaveBeenCalled();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("ignores a click on a disabled row and keeps the menu open", async () => {
      const run = vi.fn();
      render(<MenuHost />);
      show([{ text: "Delete", enabled: false, run }]);

      const row = screen.getByRole("menuitem", { name: "Delete" });
      await userEvent.click(row);

      expect(row).toHaveAttribute("aria-disabled", "true");
      expect(run).not.toHaveBeenCalled();
      expect(screen.getByRole("menu")).toBeInTheDocument();
    });

    it("leaves enabled rows without aria-disabled", () => {
      render(<MenuHost />);

      show([{ text: "Open" }]);

      expect(screen.getByRole("menuitem", { name: "Open" })).not.toHaveAttribute("aria-disabled");
    });
  });

  describe("keyboard", () => {
    const rows: MenuEntry[] = [{ text: "Alpha" }, { text: "Beta", enabled: false }, { text: "Gamma" }, { text: "Delta" }];

    it("moves down through enabled rows, skipping disabled ones, and wraps", async () => {
      render(<Anchored />);
      show(rows, anchorButton());

      await userEvent.keyboard("{ArrowDown}");
      expect(screen.getByRole("menuitem", { name: "Gamma" })).toHaveFocus();
      await userEvent.keyboard("{ArrowDown}");
      expect(screen.getByRole("menuitem", { name: "Delta" })).toHaveFocus();
      await userEvent.keyboard("{ArrowDown}");
      expect(screen.getByRole("menuitem", { name: "Alpha" })).toHaveFocus();
    });

    it("moves up through enabled rows and wraps to the last", async () => {
      render(<Anchored />);
      show(rows, anchorButton());

      await userEvent.keyboard("{ArrowUp}");
      expect(screen.getByRole("menuitem", { name: "Delta" })).toHaveFocus();
      await userEvent.keyboard("{ArrowUp}");
      expect(screen.getByRole("menuitem", { name: "Gamma" })).toHaveFocus();
    });

    it("focuses the first row on ArrowDown in a menu opened at a point", async () => {
      render(<MenuHost />);
      show(rows);

      await userEvent.keyboard("{ArrowDown}");
      expect(screen.getByRole("menuitem", { name: "Alpha" })).toHaveFocus();
    });

    it("jumps to the last enabled row on End and the first on Home", async () => {
      render(<Anchored />);
      show(rows, anchorButton());

      await userEvent.keyboard("{End}");
      expect(screen.getByRole("menuitem", { name: "Delta" })).toHaveFocus();
      await userEvent.keyboard("{Home}");
      expect(screen.getByRole("menuitem", { name: "Alpha" })).toHaveFocus();
    });

    it.each([["{Enter}"], [" "]])("runs the focused row on %j and closes", async (key) => {
      const run = vi.fn();
      render(<Anchored />);
      show([{ text: "Alpha" }, { text: "Gamma", run }], anchorButton());
      await userEvent.keyboard("{ArrowDown}");

      await userEvent.keyboard(key);

      expect(run).toHaveBeenCalled();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("focuses the next row starting with the typed letter", async () => {
      render(<Anchored />);
      show(rows, anchorButton());

      await userEvent.keyboard("d");

      expect(screen.getByRole("menuitem", { name: "Delta" })).toHaveFocus();
    });

    it("cycles through rows sharing the typed letter and ignores disabled ones", async () => {
      render(<Anchored />);
      show([{ text: "Copy" }, { text: "Cut", enabled: false }, { text: "Clear" }], anchorButton());

      await userEvent.keyboard("c");
      expect(screen.getByRole("menuitem", { name: "Clear" })).toHaveFocus();
      await userEvent.keyboard("c");
      expect(screen.getByRole("menuitem", { name: "Copy" })).toHaveFocus();
    });
  });

  describe("closing", () => {
    it("closes on Escape and returns the focus to the anchor button", async () => {
      render(<Anchored />);
      show([{ text: "Open" }], anchorButton());

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(anchorButton()).toHaveFocus();
    });

    it("closes when the mouse goes down outside", () => {
      render(<MenuHost />);
      show([{ text: "Open" }]);

      fireEvent.mouseDown(document.body);

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("stays open when the mouse goes down inside", () => {
      render(<MenuHost />);
      show([{ text: "Open" }]);

      fireEvent.mouseDown(screen.getByRole("menuitem", { name: "Open" }));

      expect(screen.getByRole("menu")).toBeInTheDocument();
    });

    it("leaves a press on its own anchor to the anchor", () => {
      render(<Anchored />);
      show([{ text: "Open" }], anchorButton());

      fireEvent.mouseDown(anchorButton());

      expect(screen.getByRole("menu")).toBeInTheDocument();
    });

    it("closes when something outside scrolls", () => {
      render(<MenuHost />);
      show([{ text: "Open" }]);

      fireEvent.scroll(document);

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("closes when the window loses focus", () => {
      render(<MenuHost />);
      show([{ text: "Open" }]);

      fireEvent.blur(window);

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  });

  describe("captions and separators", () => {
    it("groups the rows after a caption under the caption's name", () => {
      render(<MenuHost />);

      show([{ text: "Open" }, { caption: "Layout" }, { text: "List" }, { text: "Board" }]);

      const group = screen.getByRole("group", { name: "Layout" });
      expect(within(group).getAllByRole("menuitem").map((row) => row.textContent)).toEqual(["List", "Board"]);
      expect(within(group).queryByRole("menuitem", { name: "Open" })).not.toBeInTheDocument();
    });

    it("ends the group at a separator", () => {
      render(<MenuHost />);

      show([{ caption: "Layout" }, { text: "List" }, "separator", { text: "Settings" }]);

      const group = screen.getByRole("group", { name: "Layout" });
      expect(within(group).getByRole("menuitem", { name: "List" })).toBeInTheDocument();
      expect(within(group).queryByRole("menuitem", { name: "Settings" })).not.toBeInTheDocument();
      expect(screen.getByRole("menuitem", { name: "Settings" })).toBeInTheDocument();
      expect(screen.getByRole("separator")).toBeInTheDocument();
    });
  });

  describe("choices", () => {
    it("shows checked rows as radio items with their state", () => {
      render(<MenuHost />);

      show([
        { text: "List", checked: true },
        { text: "Board", checked: false },
      ]);

      expect(screen.getByRole("menuitemradio", { name: "List" })).toHaveAttribute("aria-checked", "true");
      expect(screen.getByRole("menuitemradio", { name: "Board" })).toHaveAttribute("aria-checked", "false");
    });

    it("shows toggle rows as checkbox items with their state", () => {
      render(<MenuHost />);

      show([
        { text: "Show closed", checked: true, toggle: true },
        { text: "Show empty", checked: false, toggle: true },
      ]);

      expect(screen.getByRole("menuitemcheckbox", { name: "Show closed" })).toHaveAttribute("aria-checked", "true");
      expect(screen.getByRole("menuitemcheckbox", { name: "Show empty" })).toHaveAttribute("aria-checked", "false");
    });
  });

  describe("submenus", () => {
    const entries = (run = vi.fn()): MenuEntry[] => [
      { text: "Open" },
      { text: "Move to", items: [{ text: "Backlog", run }, { text: "Done" }] },
    ];

    it("marks a row with items as opening a menu", () => {
      render(<MenuHost />);

      show(entries());

      expect(screen.getByRole("menuitem", { name: "Move to" })).toHaveAttribute("aria-haspopup", "menu");
      expect(screen.getByRole("menuitem", { name: "Open" })).not.toHaveAttribute("aria-haspopup");
    });

    it("opens the submenu on ArrowRight with its first row focused", async () => {
      render(<Anchored />);
      show(entries(), anchorButton());
      await userEvent.keyboard("{ArrowDown}");

      await userEvent.keyboard("{ArrowRight}");

      expect(screen.getByRole("menu", { name: "Move to" })).toBeInTheDocument();
      expect(screen.getByRole("menuitem", { name: "Backlog" })).toHaveFocus();
    });

    it("opens the submenu on Enter", async () => {
      render(<Anchored />);
      show(entries(), anchorButton());
      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(screen.getByRole("menu", { name: "Move to" })).toBeInTheDocument();
      expect(screen.getByRole("menuitem", { name: "Backlog" })).toHaveFocus();
    });

    it("closes the submenu on ArrowLeft and returns the focus to the parent row", async () => {
      render(<Anchored />);
      show(entries(), anchorButton());
      await userEvent.keyboard("{ArrowDown}{ArrowRight}");

      await userEvent.keyboard("{ArrowLeft}");

      expect(screen.queryByRole("menu", { name: "Move to" })).not.toBeInTheDocument();
      expect(screen.getByRole("menu", { name: "Menu" })).toBeInTheDocument();
      expect(screen.getByRole("menuitem", { name: "Move to" })).toHaveFocus();
    });

    it("opens the submenu when its row is hovered", async () => {
      render(<MenuHost />);
      show(entries());

      await userEvent.hover(screen.getByRole("menuitem", { name: "Move to" }));

      expect(screen.getByRole("menu", { name: "Move to" })).toBeInTheDocument();
    });

    it("runs a chosen submenu row and closes every menu", async () => {
      const run = vi.fn();
      render(<MenuHost />);
      show(entries(run));
      await userEvent.hover(screen.getByRole("menuitem", { name: "Move to" }));

      await userEvent.click(screen.getByRole("menuitem", { name: "Backlog" }));

      expect(run).toHaveBeenCalled();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  });
});
