import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { MouseEvent } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetStore } from "../../state/store";
import { MenuHost, openContextMenu, openMenu, type MenuAt, type MenuEntry } from ".";

function show(entries: MenuEntry[], at: MenuAt = { x: 20, y: 30 }, label?: string, options?: Parameters<typeof openMenu>[3]) {
  act(() => openMenu(entries, at, label, options));
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

  describe("search", () => {
    const team: MenuEntry[] = [{ caption: "Team" }, { text: "Alex Morgan" }, { text: "Sam Lee" }, "separator", { text: "Unassigned" }];
    const showSearch = (entries: MenuEntry[] = team) => show(entries, { x: 20, y: 30 }, "People", { search: "Search people" });
    const field = () => screen.getByRole("textbox", { name: "Search people" });
    const shown = () => screen.getAllByRole("menuitem").map((row) => row.textContent);

    it("opens with the field focused and the first enabled row highlighted", () => {
      render(<MenuHost />);

      showSearch([{ text: "Archived", enabled: false }, { text: "Alex Morgan" }, { text: "Sam Lee" }]);

      expect(field()).toHaveFocus();
      expect(screen.getByRole("menuitem", { name: "Alex Morgan" })).toHaveAttribute("data-active", "true");
      expect(screen.getByRole("menuitem", { name: "Sam Lee" })).not.toHaveAttribute("data-active");
    });

    it("narrows the rows to those containing the typed text, ignoring case", async () => {
      render(<MenuHost />);
      showSearch();

      await userEvent.type(field(), "SAM");

      expect(shown()).toEqual(["Sam Lee"]);
    });

    it("keeps a caption while one of its rows matches", async () => {
      render(<MenuHost />);
      showSearch();

      await userEvent.type(field(), "alex");

      expect(screen.getByRole("group", { name: "Team" })).toBeInTheDocument();
    });

    it("drops a caption and the separators when none of its rows match", async () => {
      render(<MenuHost />);
      showSearch();

      await userEvent.type(field(), "unass");

      expect(shown()).toEqual(["Unassigned"]);
      expect(screen.queryByRole("group", { name: "Team" })).not.toBeInTheDocument();
      expect(screen.queryByRole("separator")).not.toBeInTheDocument();
    });

    it("highlights the first matching enabled row while narrowing", async () => {
      render(<MenuHost />);
      showSearch([{ text: "Alpha", enabled: false }, { text: "Beta" }, { text: "Alps" }, { text: "Alder" }]);

      await userEvent.type(field(), "al");

      expect(screen.getByRole("menuitem", { name: "Alps" })).toHaveAttribute("data-active", "true");
      expect(screen.getByRole("menuitem", { name: "Alder" })).not.toHaveAttribute("data-active");
    });

    it("moves the highlight among the shown rows with the arrows while the field keeps the focus", async () => {
      render(<MenuHost />);
      showSearch();

      await userEvent.keyboard("{ArrowDown}");
      expect(screen.getByRole("menuitem", { name: "Sam Lee" })).toHaveAttribute("data-active", "true");
      await userEvent.keyboard("{ArrowDown}");
      expect(screen.getByRole("menuitem", { name: "Unassigned" })).toHaveAttribute("data-active", "true");
      await userEvent.keyboard("{ArrowUp}");
      expect(screen.getByRole("menuitem", { name: "Sam Lee" })).toHaveAttribute("data-active", "true");
      expect(field()).toHaveFocus();
    });

    it("runs the highlighted row on Enter and closes the menu", async () => {
      const run = vi.fn();
      render(<MenuHost />);
      showSearch([{ text: "Alex Morgan" }, { text: "Sam Lee", run }, { text: "Sam Park" }]);
      await userEvent.type(field(), "sam");

      await userEvent.keyboard("{Enter}");

      expect(run).toHaveBeenCalled();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("types letters and spaces into the field instead of jumping to rows", async () => {
      render(<MenuHost />);
      showSearch();

      await userEvent.keyboard("x m");

      expect(field()).toHaveValue("x m");
      expect(shown()).toEqual(["Alex Morgan"]);
    });

    it("says nothing matches, naming the text, and shows every row again once it is cleared", async () => {
      render(<MenuHost />);
      showSearch();

      await userEvent.type(field(), "zzz");
      expect(screen.getByText("No match for “zzz”")).toBeInTheDocument();
      expect(screen.queryAllByRole("menuitem")).toHaveLength(0);

      await userEvent.clear(field());

      expect(screen.queryByText(/No match for/)).not.toBeInTheDocument();
      expect(shown()).toEqual(["Alex Morgan", "Sam Lee", "Unassigned"]);
    });

    it("closes on Escape", async () => {
      render(<MenuHost />);
      showSearch();

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("has no field in a menu without search, where a letter still jumps to a row", async () => {
      render(<Anchored />);
      show(team, anchorButton());

      await userEvent.keyboard("s");

      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(screen.getByRole("menuitem", { name: "Sam Lee" })).toHaveFocus();
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
