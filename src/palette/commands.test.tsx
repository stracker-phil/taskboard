import { render } from "@testing-library/react";
import { Settings as SettingsIcon } from "lucide-react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type MenuCommand } from "../api";
import { registerCoreScopes } from "../commands";
import { clearCommands, registerCommands, setOverrides, type Command } from "../commands/registry";
import type { MenuEntry } from "../components/menu";
import { registerSettingsPage } from "../registry/settingsPages";
import { resetStore, useStore } from "../state/store";
import { collectCommands, snapshotPage } from "./commands";
import { menuCommand } from "../test/palette";
import { usePaletteMenu, type PaletteMenu } from "./menus";

const mocked = vi.mocked(api, true);

const command = (id: string, patch: Partial<Command> = {}): Command => ({ id, title: id, scope: "main", run: vi.fn(), ...patch });
const item = menuCommand;
const collectAll = (menu: MenuCommand[]) => collectCommands(menu, snapshotPage());
/** Without the Settings pages, which every test shares. */
const collect = (menu: MenuCommand[]) => collectAll(menu).filter((c) => c.section !== "Settings");
const byLabel = (list: ReturnType<typeof collect>, label: string) => list.find((c) => c.label === label);

function Offer({ id, menu }: { id: string; menu: PaletteMenu | null }) {
  usePaletteMenu(id, menu);
  return null;
}
const offer = (id: string, menu: PaletteMenu | null) => render(<Offer id={id} menu={menu} />);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  clearCommands();
  registerCoreScopes();
});

describe("collectCommands", () => {
  describe("menu items from Rust", () => {
    it("runs the Rust command by its id", () => {
      collect([item()])[0].run();

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:refresh");
    });

    it("keeps the menu as the section and shows the accelerator as its key", () => {
      const [refresh] = collect([item({ accelerator: "CmdOrCtrl+Shift+R" })]);

      expect(refresh).toMatchObject({ label: "Refresh now", section: "File", shortcut: "⌘⇧R" });
    });

    it("shows no key for an item without an accelerator", () => {
      expect(collect([item()])[0].shortcut).toBeUndefined();
    });

    it("runs the page's twin command instead, with the twin's key", () => {
      const twin = command("sync.refresh", { bus: "sync:refresh", defaultBinding: "Mod+R" });
      registerCommands([twin]);

      const [refresh] = collect([item({ accelerator: "CmdOrCtrl+Shift+R" })]);
      refresh.run();

      expect(twin.run).toHaveBeenCalled();
      expect(mocked.runCommand).not.toHaveBeenCalled();
      expect(refresh.shortcut).toBe("⌘R");
    });
  });

  describe("menu items for page commands", () => {
    const pageItem = item({ menu: "Go", label: "Next view", command: "page:view.next" });

    it("runs the registered page command", () => {
      const next = command("view.next");
      registerCommands([next]);

      collect([pageItem])[0].run();

      expect(next.run).toHaveBeenCalled();
      expect(mocked.runCommand).not.toHaveBeenCalled();
    });

    it("shows the command's key, a user override included", () => {
      registerCommands([command("view.next", { defaultBinding: "Mod+]" })]);
      expect(collect([pageItem])[0].shortcut).toBe("⌘]");

      setOverrides({ "view.next": "Mod+Shift+N" });
      expect(collect([pageItem])[0].shortcut).toBe("⌘⇧N");
    });

    it("leaves the item out when no such command is registered", () => {
      expect(collect([pageItem])).toEqual([]);
    });

    it("leaves the item out when the command could not run at snapshot time", () => {
      registerCommands([command("view.next", { when: () => false })]);

      expect(collect([pageItem])).toEqual([]);
    });

    it("uses the snapshot, not the state when the entries are collected", () => {
      let enabled = true;
      registerCommands([command("view.next", { when: () => enabled })]);
      const snapshot = snapshotPage();
      enabled = false;

      expect(collectCommands([pageItem], snapshot).map((c) => c.label)).toContain("Next view");
    });
  });

  describe("the page's own menus", () => {
    const sortMenu = (entries: MenuEntry[]): PaletteMenu => ({ section: "This view", title: "Group", entries: () => entries });

    it("lists a menu's items as “Title: Item” under its section, after the Rust items", () => {
      const run = vi.fn();
      offer("t.group", sortMenu([{ text: "Status", run }]));

      const list = collect([item()]);
      byLabel(list, "Group: Status")!.run();

      expect(list.map((c) => c.label)).toEqual(["Refresh now", "Group: Status"]);
      expect(byLabel(list, "Group: Status")!.section).toBe("This view");
      expect(run).toHaveBeenCalled();
    });

    it("flattens a submenu into a path", () => {
      offer("t.group", sortMenu([{ text: "Sort by", items: [{ text: "Updated", run: vi.fn() }] }]));

      expect(collect([]).map((c) => c.label)).toEqual(["Group: Sort by › Updated"]);
    });

    it("carries the checked state and the hint as its detail", () => {
      offer("t.group", sortMenu([{ text: "Status", checked: true, hint: "current", run: vi.fn() }]));

      expect(collect([])[0]).toMatchObject({ checked: true, detail: "current" });
    });

    it("drops captions, separators, greyed rows and rows that do nothing", () => {
      offer("t.group", sortMenu([{ caption: "Layout" }, "separator", { text: "Delete", enabled: false, run: vi.fn() }, { text: "Inert" }, { text: "Keep", run: vi.fn() }]));

      expect(collect([]).map((c) => c.label)).toEqual(["Group: Keep"]);
    });

    it("reads the entries when the palette opens", () => {
      let label = "Before";
      offer("t.live", { section: "S", title: "T", entries: () => [{ text: label, run: vi.fn() }] });
      label = "After";

      expect(collect([]).map((c) => c.label)).toEqual(["T: After"]);
    });

    it("lists the latest registration under an id", () => {
      const view = render(<Offer id="t.group" menu={sortMenu([{ text: "Old", run: vi.fn() }])} />);
      view.rerender(<Offer id="t.group" menu={sortMenu([{ text: "New", run: vi.fn() }])} />);

      expect(collect([]).map((c) => c.label)).toEqual(["Group: New"]);
    });

    it("offers nothing for null or once unmounted", () => {
      offer("t.none", null);
      const mounted = offer("t.group", sortMenu([{ text: "Status", run: vi.fn() }]));
      expect(collect([])).toHaveLength(1);

      mounted.unmount();

      expect(collect([])).toEqual([]);
    });
  });

  describe("Settings pages", () => {
    registerSettingsPage({
      id: "palette-test",
      group: "Jira",
      label: "Palette demo",
      icon: SettingsIcon,
      Body: () => null,
      sections: [
        { id: "site", title: "Site" },
        { id: "token", title: "Token" },
      ],
    });

    it("lists a page as “Settings: label” with its section titles as detail", () => {
      expect(byLabel(collectAll([]), "Settings: Palette demo")).toMatchObject({ section: "Settings", detail: "Site, Token" });
    });

    it("opens the page through the Rust command for it", () => {
      byLabel(collectAll([]), "Settings: Palette demo")!.run();

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:palette-test");
    });
  });

  it("toasts the reason when a Rust command fails", async () => {
    mocked.runCommand.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

    collect([item()])[0].run();

    await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not run “Refresh now”: Disk full"));
  });
});
