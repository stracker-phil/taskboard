import { QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Settings as SettingsIcon } from "lucide-react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type ListNode, type MenuCommand, type Nav, type PaletteResults, type SidebarTree } from "../api";
import { installCommands, useCommandKeys } from "../commands";
import { registerCommands } from "../commands/registry";
import type { MenuEntry } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { NAV_KEY, SIDEBAR_KEY } from "../queries/sidebar";
import { registerSettingsPage } from "../registry/settingsPages";
import { resetStore, useStore } from "../state/store";
import { connectionView } from "../test/jira";
import { issueHit, listHit, menuCommand, parts, viewHit } from "../test/palette";
import { sidebarTree, space } from "../test/spaces";
import { CommandPalette } from "./CommandPalette";
import { usePaletteMenu, type PaletteMenu } from "./menus";

const mocked = vi.mocked(api, true);

const SPRINT: ListNode = { id: 21, kind: "list", name: "Current sprint", count: null, ownConditions: 1, description: "", error: null, icon: null, color: null };
const TREE: SidebarTree = sidebarTree([space({ lists: [SPRINT] })]);
const NW_ID = 11;
const NEW_LIST_ID = 31;

interface Setup {
  results?: Partial<PaletteResults>;
  tree?: SidebarTree;
  location?: Nav["location"];
  siteUrl?: string;
  /** What the Rust menus offer. */
  menu?: MenuCommand[];
  /** A part of the page's own menus. */
  pageMenu?: PaletteMenu;
}

function Keys() {
  useCommandKeys();
  return null;
}

function PageMenu({ menu }: { menu: PaletteMenu }) {
  usePaletteMenu("test.menu", menu);
  return null;
}

async function renderPalette({ results = {}, tree = TREE, location = null, siteUrl = "", menu = [], pageMenu }: Setup = {}) {
  mocked.paletteQuery.mockResolvedValue({ issues: [], lists: [], views: [], ...results });
  mocked.sidebarTree.mockResolvedValue(tree);
  mocked.navCurrent.mockResolvedValue({ location, expanded: [] });
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl }));
  mocked.menuCommands.mockResolvedValue(menu);
  mocked.listCreate.mockResolvedValue(NEW_LIST_ID);
  const client = createQueryClient();
  render(
    <QueryClientProvider client={client}>
      <Keys />
      {pageMenu && <PageMenu menu={pageMenu} />}
      <CommandPalette />
    </QueryClientProvider>,
  );
  if (useStore.getState().paletteOpen) {
    await waitFor(() => {
      expect(client.getQueryData(SIDEBAR_KEY)).toBeDefined();
      expect(client.getQueryData(NAV_KEY)).toBeDefined();
      expect(client.getQueryData(["menuCommands"])).toBeDefined();
    });
  }
  return client;
}

/** Opens the palette after the page has mounted, as ⌘K does, so the page's menus are registered. */
async function openOnPage(setup: Setup) {
  resetStore({ paletteOpen: false });
  await renderPalette(setup);
  act(() => useStore.getState().setPaletteOpen(true));
  await screen.findByRole("dialog", { name: "Command palette" });
}

const input = () => screen.getByRole<HTMLInputElement>("combobox");
const options = (region?: string) => within(region ? screen.getByRole("region", { name: region }) : screen.getByRole("listbox")).getAllByRole("option");
const selected = () => screen.getAllByRole("option").filter((o) => o.getAttribute("aria-selected") === "true");
const headings = () => screen.getAllByRole("region").map((r) => r.getAttribute("aria-label"));

async function search(text: string) {
  await userEvent.type(input(), text);
}

const FILE_MENU = [menuCommand(), menuCommand({ label: "Full sync", command: "sync:full" })];
const items = (count: number, menu = "Go"): MenuCommand[] => Array.from({ length: count }, (_, i) => menuCommand({ menu, label: `Item ${i + 1}`, command: `bus:${i + 1}` }));

registerSettingsPage({ id: "zebra", group: "Jira", label: "Zebra stripes", icon: SettingsIcon, Body: () => null, sections: [{ id: "site", title: "Site address" }] });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore({ paletteOpen: true });
  installCommands("main");
});

describe("CommandPalette", () => {
  describe("opening", () => {
    it("shows nothing while closed", async () => {
      resetStore({ paletteOpen: false });
      await renderPalette();

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("shows the command palette with a focused search field when open", async () => {
      await renderPalette();

      expect(screen.getByRole("dialog", { name: "Command palette" })).toBeInTheDocument();
      expect(input()).toHaveFocus();
    });

    it("opens on ⌘K", async () => {
      resetStore({ paletteOpen: false });
      await renderPalette();
      await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));

      await userEvent.keyboard("{Meta>}k{/Meta}");

      expect(await screen.findByRole("dialog", { name: "Command palette" })).toBeInTheDocument();
    });

    it("closes on Escape", async () => {
      await renderPalette();

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(useStore.getState().paletteOpen).toBe(false);
    });

    it("closes when the dimmed window around it is clicked", async () => {
      await renderPalette();

      await userEvent.click(screen.getByRole("dialog").parentElement!);

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("stays open when the palette itself is clicked", async () => {
      await renderPalette();

      await userEvent.click(screen.getByRole("dialog"));

      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });
  });

  describe("without text", () => {
    it("offers the three prefixes to narrow to, and no results", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [listHit()] }, menu: FILE_MENU });

      expect(headings()).toEqual(["Narrow to"]);
      expect(options("Narrow to").map((o) => o.textContent)).toEqual([
        expect.stringContaining("Commands"),
        expect.stringContaining("Issues"),
        expect.stringContaining("Lists"),
      ]);
      expect(mocked.paletteQuery).not.toHaveBeenCalled();
    });

    it.each([
      ["Commands", ">", "Run a menu command or a view option"],
      ["Issues", "#", "Find an issue by key or text"],
      ["Lists", "/", "Go to a Space or a filter list"],
    ])("puts the %s prefix into the field when its row is clicked, and stays open", async (label, char, placeholder) => {
      await renderPalette();

      await userEvent.click(within(screen.getByRole("region", { name: "Narrow to" })).getByRole("option", { name: new RegExp(label) }));

      expect(input()).toHaveValue(char);
      expect(input()).toHaveFocus();
      expect(input()).toHaveAttribute("placeholder", placeholder);
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    it("puts the selected prefix into the field on ↩", async () => {
      await renderPalette();

      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(input()).toHaveValue("#");
    });

    it("searches all kinds under the default placeholder", async () => {
      await renderPalette();

      expect(input()).toHaveAttribute("placeholder", "Search issues, lists and commands");
      expect(input()).toHaveAccessibleName("Search issues, lists and commands");
    });
  });

  describe("footer prefix buttons", () => {
    it("has one button per prefix, none pressed at first", async () => {
      await renderPalette();

      expect(screen.getByRole("button", { name: /Commands/, pressed: false })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Issues/, pressed: false })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Lists/, pressed: false })).toBeInTheDocument();
    });

    it("narrows to the clicked prefix and marks it pressed", async () => {
      await renderPalette();

      await userEvent.click(screen.getByRole("button", { name: /Issues/ }));

      expect(input()).toHaveValue("#");
      expect(screen.getByRole("button", { name: /Issues/ })).toHaveAttribute("aria-pressed", "true");
    });

    it("clears the field when the active prefix is clicked again", async () => {
      await renderPalette();
      await userEvent.click(screen.getByRole("button", { name: /Lists/ }));

      await userEvent.click(screen.getByRole("button", { name: /Lists/ }));

      expect(input()).toHaveValue("");
      expect(screen.getByRole("button", { name: /Lists/ })).toHaveAttribute("aria-pressed", "false");
    });

    it("swaps the prefix when another button is clicked", async () => {
      await renderPalette();
      await search("#form");

      await userEvent.click(screen.getByRole("button", { name: /Commands/ }));

      expect(input()).toHaveValue(">");
    });
  });

  describe("searching", () => {
    it("asks Rust for the typed text in every kind", async () => {
      await renderPalette();

      await search("contact");

      await waitFor(() => expect(mocked.paletteQuery).toHaveBeenLastCalledWith("contact", "all"));
    });

    it("asks once for text typed in a burst, not for each prefix", async () => {
      await renderPalette();

      await userEvent.type(input(), "abc", { delay: null });

      await waitFor(() => expect(mocked.paletteQuery).toHaveBeenCalledWith("abc", "all"));
      expect(mocked.paletteQuery.mock.calls.map(([text]) => text)).toEqual(["abc"]);
    });

    it("keeps the previous results listed while the next ones load", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] } });
      await search("contact");
      await screen.findByRole("option", { name: /NW-43/ });
      mocked.paletteQuery.mockReturnValue(new Promise(() => {}));

      await userEvent.type(input(), "s");
      await waitFor(() => expect(mocked.paletteQuery).toHaveBeenLastCalledWith("contacts", "all"));

      expect(screen.getByRole("option", { name: /NW-43/ })).toBeInTheDocument();
    });

    it("lists an issue with its key, summary, status and the matched words in bold", async () => {
      await renderPalette({ results: { issues: [issueHit({ summary: parts("Fix the ", ["contact"], " form") })], lists: [] } });

      await search("contact");

      const option = await screen.findByRole("option", { name: /NW-43/ });
      expect(within(option).getByText("contact").tagName).toBe("STRONG");
      expect(option.textContent).toContain("Fix the contact form");
      expect(within(option).getByText("In Progress")).toBeInTheDocument();
      expect(within(option).queryByText("gone")).not.toBeInTheDocument();
    });

    it("marks an issue that is gone", async () => {
      await renderPalette({ results: { issues: [issueHit({ gone: true })], lists: [] } });

      await search("contact");

      expect(within(await screen.findByRole("option", { name: /NW-43/ })).getByText("gone")).toBeInTheDocument();
    });

    it("shows the words around a match in a comment", async () => {
      const comment = parts("asked for a ", ["refund"], " twice");
      await renderPalette({ results: { issues: [issueHit({ matchedIn: "comment", comment })], lists: [] } });

      await search("refund");

      const option = await screen.findByRole("option", { name: /NW-43/ });
      expect(option.textContent).toContain("Matched in comment: “asked for a refund twice”");
      expect(within(option).getByText("refund").tagName).toBe("STRONG");
    });

    it("lists a filter list under its Space's name", async () => {
      await renderPalette({ results: { issues: [], lists: [listHit()] } });

      await search("sprint");

      const row = await screen.findByRole("option", { name: /Current sprint/ });
      expect(within(row).getByText("Website relaunch › Current sprint")).toBeInTheDocument();
      expect(within(row).getByText("filter list")).toBeInTheDocument();
    });

    it("lists a Space by its name", async () => {
      await renderPalette({ results: { issues: [], lists: [listHit({ id: NW_ID, name: "Website relaunch", isSpace: true })] } });

      await search("website");

      const row = await screen.findByRole("option", { name: /Website relaunch/ });
      expect(within(row).getByText("Website relaunch")).toBeInTheDocument();
      expect(within(row).getByText("Space")).toBeInTheDocument();
    });

    it("lists a list outside any Space by its name alone", async () => {
      await renderPalette({ results: { issues: [], lists: [listHit({ id: 2, name: "Unread", projectId: null })] } });

      await search("recent");

      expect(within(await screen.findByRole("option", { name: /Unread/ })).getByText("Unread")).toBeInTheDocument();
    });

    it("lists a view after the lists, as Space › list › view", async () => {
      await renderPalette({ results: { lists: [listHit()], views: [viewHit()] } });

      await search("board");

      await screen.findByRole("option", { name: /Sprint board/ });
      const rows = options("Filter lists");
      expect(rows.map((o) => o.textContent)).toEqual([expect.stringContaining("Current sprint"), expect.stringContaining("Sprint board")]);
      expect(within(rows[1]).getByText("Website relaunch › Current sprint › Sprint board")).toBeInTheDocument();
      expect(within(rows[1]).getByText("view")).toBeInTheDocument();
    });

    it("lists a view of a Space as Space › view", async () => {
      const hit = viewHit({ list: listHit({ id: NW_ID, name: "Website relaunch", isSpace: true }), name: "All issues", layout: "list" });
      await renderPalette({ results: { views: [hit] } });

      await search("all");

      const row = await screen.findByRole("option", { name: /All issues/ });
      expect(within(row).getByText("Website relaunch › All issues")).toBeInTheDocument();
    });

    it("counts a view as a list", async () => {
      await renderPalette({ results: { lists: [listHit()], views: [viewHit()] } });

      await search("sprint");

      await screen.findByRole("option", { name: /Sprint board/ });
      expect(screen.getByText(/2 lists/)).toBeInTheDocument();
    });

    it("counts the issues, lists and commands", async () => {
      await renderPalette({ results: { issues: [issueHit(), issueHit({ id: "10044", key: "NW-44" })], lists: [listHit()] } });

      await search("form");

      await screen.findAllByRole("option", { name: /NW-4/ });
      const commands = options("Commands").length;
      expect(screen.getByText(`2 issues · 1 list · ${commands} ${commands === 1 ? "command" : "commands"}`)).toBeInTheDocument();
    });

    it("leaves a part out of the count when there is none", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] } });

      await search("contact");

      await screen.findByRole("option", { name: /NW-43/ });
      expect(screen.getByText("1 issue · 1 command")).toBeInTheDocument();
    });

    it("offers to save the text as a filter list", async () => {
      await renderPalette();

      await search("contact");

      expect(screen.getByRole("option", { name: /Save “contact” as a filter list/ })).toBeInTheDocument();
    });

    it("lists the commands that match the text with their menu and key, and keeps the save entry", async () => {
      await renderPalette({ menu: [menuCommand({ accelerator: "CmdOrCtrl+Shift+R" }), ...FILE_MENU.slice(1)] });

      await search("refresh");

      const refresh = screen.getByRole("option", { name: /Refresh now/ });
      expect(within(refresh).getByText("File")).toBeInTheDocument();
      expect(within(refresh).getByText("⌘⇧R")).toBeInTheDocument();
      expect(screen.getByRole("option", { name: /Save “refresh”/ })).toBeInTheDocument();
      expect(screen.queryByRole("option", { name: /Full sync/ })).not.toBeInTheDocument();
    });

    it("shows at most six matching commands", async () => {
      await renderPalette({ menu: items(8) });

      await search("item");

      expect(options("Commands").filter((o) => /^Item/.test(o.textContent ?? ""))).toHaveLength(6);
    });

    it("ranks a command starting with the text before one that only has it in a word", async () => {
      await renderPalette({ menu: [menuCommand({ label: "Show sidebar", command: "a" }), menuCommand({ label: "Sidebar tools", command: "b" })] });

      await search("side");

      const names = options("Commands").map((o) => o.textContent ?? "");
      expect(names.findIndex((n) => n.startsWith("Sidebar tools"))).toBeLessThan(names.findIndex((n) => n.startsWith("Show sidebar")));
    });

    it("finds a command by an abbreviation of its letters", async () => {
      await renderPalette({ menu: [menuCommand({ label: "Toggle Sidebar", command: "a" })] });

      await search("tsb");

      expect(screen.getByRole("option", { name: /Toggle Sidebar/ })).toBeInTheDocument();
    });

    it("offers to open the first issue in Jira when a site is connected", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] }, siteUrl: "https://example.atlassian.net/" });

      await search("contact");

      expect(await screen.findByRole("option", { name: /Open NW-43 in Jira/ })).toBeInTheDocument();
    });

    it("offers no Jira action without a connected site", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] } });

      await search("contact");

      await screen.findByRole("option", { name: /NW-43/ });
      expect(screen.queryByRole("option", { name: /in Jira/ })).not.toBeInTheDocument();
    });
  });

  describe("commands only (>)", () => {
    const MENUS = [...FILE_MENU, menuCommand({ menu: "Go", label: "Go to inbox", command: "go:inbox" })];

    it("groups the menu commands under their menus, in order, and never asks Rust for results", async () => {
      await renderPalette({ menu: MENUS });

      await search(">");

      expect(headings().slice(0, 2)).toEqual(["File", "Go"]);
      expect(options("File").map((o) => o.textContent)).toEqual([expect.stringContaining("Refresh now"), expect.stringContaining("Full sync")]);
      expect(options("Go")).toHaveLength(1);
      expect(mocked.paletteQuery).not.toHaveBeenCalled();
    });

    it("lists the Settings pages under their own heading", async () => {
      await renderPalette({ menu: MENUS });

      await search(">");

      expect(within(screen.getByRole("region", { name: "Settings" })).getByRole("option", { name: /Settings: Zebra stripes/ })).toBeInTheDocument();
    });

    it("shows one ranked list of all matches under “Commands” once there is a query", async () => {
      await renderPalette({ menu: [...items(8), menuCommand({ menu: "View", label: "Show sidebar", command: "x" }), menuCommand({ menu: "View", label: "Sidebar tools", command: "y" })] });

      await search(">item");
      expect(headings()).toEqual(["Commands"]);
      expect(options("Commands").filter((o) => /^Item/.test(o.textContent ?? ""))).toHaveLength(8);

      await userEvent.clear(input());
      await search(">side");
      const names = options("Commands").map((o) => o.textContent ?? "");
      expect(names[0]).toMatch(/^Sidebar tools/);
      expect(names[1]).toMatch(/^Show sidebar/);
    });

    it("offers no filter-list save and no issues", async () => {
      await renderPalette({ menu: MENUS, results: { issues: [issueHit()], lists: [listHit()] } });

      await search(">refresh");

      expect(screen.getByRole("option", { name: /Refresh now/ })).toBeInTheDocument();
      expect(screen.queryByRole("option", { name: /Save “/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("option", { name: /NW-43/ })).not.toBeInTheDocument();
    });

    it("runs a Rust menu item through the command bus with its id", async () => {
      await renderPalette({ menu: MENUS });
      await search(">inbox");

      await userEvent.click(screen.getByRole("option", { name: /Go to inbox/ }));

      expect(mocked.runCommand).toHaveBeenCalledWith("go:inbox");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("marks a menu item that is checked", async () => {
      await renderPalette({ menu: [menuCommand({ menu: "View", label: "Show sidebar", command: "view:sidebar", checked: true }), menuCommand({ menu: "View", label: "Show toolbar", command: "view:toolbar" })] });

      await search(">show");

      expect(within(screen.getByRole("option", { name: /Show sidebar/ })).getByLabelText("On")).toBeInTheDocument();
      expect(within(screen.getByRole("option", { name: /Show toolbar/ })).queryByLabelText("On")).not.toBeInTheDocument();
    });

    it("runs a Settings page through the command bus", async () => {
      await renderPalette();
      await search(">zebra");

      await userEvent.click(screen.getByRole("option", { name: /Settings: Zebra stripes/ }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:zebra");
    });

    it("finds a Settings page by the title of one of its groups", async () => {
      await renderPalette();

      await search(">site address");

      expect(screen.getByRole("option", { name: /Settings: Zebra stripes/ })).toBeInTheDocument();
    });

    it("says so when no command matches", async () => {
      await renderPalette({ menu: MENUS });

      await search(">qqqq");

      expect(await screen.findByText("No results")).toBeInTheDocument();
      expect(mocked.paletteQuery).not.toHaveBeenCalled();
    });
  });

  describe("page commands in the menus", () => {
    const NEXT = menuCommand({ menu: "Go", label: "Next view", command: "page:test.next" });

    it("runs the registered page command and closes", async () => {
      const run = vi.fn();
      registerCommands([{ id: "test.next", title: "Next view", scope: "main", run }]);
      await renderPalette({ menu: [NEXT] });
      await search(">next");

      await userEvent.click(screen.getByRole("option", { name: /Next view/ }));

      expect(run).toHaveBeenCalled();
      expect(mocked.runCommand).not.toHaveBeenCalled();
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("shows the command's key",async () => {
      registerCommands([{ id: "test.next", title: "Next view", scope: "main", defaultBinding: "Mod+]", run: vi.fn() }]);
      await renderPalette({ menu: [NEXT] });

      await search(">next");

      expect(within(screen.getByRole("option", { name: /Next view/ })).getByText("⌘]")).toBeInTheDocument();
    });

    it("leaves the item out when its command is disabled", async () => {
      registerCommands([{ id: "test.next", title: "Next view", scope: "main", when: () => false, run: vi.fn() }]);
      await renderPalette({ menu: [NEXT] });

      await search(">next");

      expect(await screen.findByText("No results")).toBeInTheDocument();
    });

    it("leaves the item out when no such command exists", async () => {
      await renderPalette({ menu: [menuCommand({ label: "Next view", command: "page:test.missing" })] });

      await search(">next view");

      expect(await screen.findByText("No results")).toBeInTheDocument();
    });
  });

  describe("the page's own menus", () => {
    const run = vi.fn();
    const menu = (entries: MenuEntry[]): PaletteMenu => ({ section: "This view", title: "Group", entries: () => entries });

    it("lists a page menu's items under its section, submenus flattened into a path",async () => {
      await openOnPage({ pageMenu: menu([{ text: "Status", run }, { text: "Sort by", items: [{ text: "Updated", run }] }]) });

      await search(">");

      expect(options("This view").map((o) => o.textContent)).toEqual([expect.stringContaining("Group: Status"), expect.stringContaining("Group: Sort by › Updated")]);
    });

    it("runs a page menu item and closes", async () => {
      await openOnPage({ pageMenu: menu([{ text: "Status", run }]) });
      await search(">group status");

      await userEvent.click(screen.getByRole("option", { name: /Group: Status/ }));

      expect(run).toHaveBeenCalled();
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("marks the option that is on", async () => {
      await openOnPage({ pageMenu: menu([{ text: "Status", checked: true, run }, { text: "Epic", checked: false, run }]) });

      await search(">group");

      expect(within(screen.getByRole("option", { name: /Group: Status/ })).getByLabelText("On")).toBeInTheDocument();
      expect(within(screen.getByRole("option", { name: /Group: Epic/ })).queryByLabelText("On")).not.toBeInTheDocument();
    });

    it("leaves out greyed rows", async () => {
      await openOnPage({ pageMenu: menu([{ text: "Status", enabled: false, run }, { text: "Epic", run }]) });

      await search(">group");

      expect(screen.queryByRole("option", { name: /Group: Status/ })).not.toBeInTheDocument();
      expect(screen.getByRole("option", { name: /Group: Epic/ })).toBeInTheDocument();
    });

    it("shows a menu item without a prefix as well, with its section", async () => {
      await openOnPage({ pageMenu: menu([{ text: "Status", run }]) });

      await search("status");

      expect(within(screen.getByRole("option", { name: /Group: Status/ })).getByText("This view")).toBeInTheDocument();
    });
  });

  describe("issues only (#)", () => {
    it("asks Rust for issues and shows no lists or other commands", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [listHit()] }, menu: FILE_MENU });

      await search("#contact");

      await screen.findByRole("option", { name: /NW-43/ });
      expect(mocked.paletteQuery).toHaveBeenLastCalledWith("contact", "issues");
      expect(screen.queryByRole("region", { name: "Filter lists" })).not.toBeInTheDocument();
      expect(screen.queryByRole("option", { name: /Refresh now/ })).not.toBeInTheDocument();
      expect(screen.getByRole("option", { name: /Save “contact” as a filter list/ })).toBeInTheDocument();
    });

    it("still offers to open the issue in Jira", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] }, siteUrl: "https://example.atlassian.net/" });

      await search("#contact");

      expect(await screen.findByRole("option", { name: /Open NW-43 in Jira/ })).toBeInTheDocument();
    });

    it("shows no “No results” for a bare #", async () => {
      await renderPalette();

      await search("#");
      await new Promise((resolve) => setTimeout(resolve, 150));

      expect(screen.queryByText("No results")).not.toBeInTheDocument();
      expect(mocked.paletteQuery).not.toHaveBeenCalled();
    });

  });

  describe("lists only (/)", () => {
    it("asks Rust for every list when nothing follows the slash", async () => {
      await renderPalette({ results: { issues: [], lists: [listHit()] } });

      await search("/");

      expect(await screen.findByRole("option", { name: /Current sprint/ })).toBeInTheDocument();
      expect(mocked.paletteQuery).toHaveBeenLastCalledWith("", "lists");
    });

    it("shows the lists under “Lists” and nothing else for the typed text", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [listHit()] }, menu: FILE_MENU });

      await search("/sprint");

      await screen.findByRole("option", { name: /Current sprint/ });
      expect(mocked.paletteQuery).toHaveBeenLastCalledWith("sprint", "lists");
      expect(headings()).toEqual(["Lists"]);
    });

    it("shows the views after the lists under “Lists” once text follows the slash", async () => {
      await renderPalette({ results: { lists: [listHit()], views: [viewHit()] } });

      await search("/sprint");

      await screen.findByRole("option", { name: /Sprint board/ });
      expect(headings()).toEqual(["Lists"]);
      expect(options("Lists")).toHaveLength(2);
    });

    it("shows no views for a bare slash", async () => {
      await renderPalette({ results: { lists: [listHit()], views: [viewHit()] } });

      await search("/");

      await screen.findByRole("option", { name: /Current sprint/ });
      expect(screen.queryByRole("option", { name: /Sprint board/ })).not.toBeInTheDocument();
    });

    it("shows no views when only issues are searched", async () => {
      await renderPalette({ results: { issues: [issueHit()], views: [viewHit()] } });

      await search("#contact");

      await screen.findByRole("option", { name: /NW-43/ });
      expect(screen.queryByRole("option", { name: /Sprint board/ })).not.toBeInTheDocument();
    });

    it("shows no “No results” for a bare / that finds no list", async () => {
      await renderPalette();

      await search("/");
      await waitFor(() => expect(mocked.paletteQuery).toHaveBeenCalled());
      await new Promise((resolve) => setTimeout(resolve, 50));

      expect(screen.queryByText("No results")).not.toBeInTheDocument();
    });

    it("says so when the text finds no list", async () => {
      await renderPalette();

      await search("/qqqq");

      expect(await screen.findByText("No results")).toBeInTheDocument();
    });
  });

  describe("keyboard selection", () => {
    it("starts on the first entry", async () => {
      await renderPalette();

      expect(selected()).toEqual([screen.getAllByRole("option")[0]]);
    });

    it("moves down and up with the arrow keys", async () => {
      await renderPalette();
      const all = screen.getAllByRole("option");

      await userEvent.keyboard("{ArrowDown}{ArrowDown}");
      expect(selected()).toEqual([all[2]]);

      await userEvent.keyboard("{ArrowUp}");
      expect(selected()).toEqual([all[1]]);
    });

    it("wraps from the first entry to the last on ArrowUp", async () => {
      await renderPalette();
      const all = screen.getAllByRole("option");

      await userEvent.keyboard("{ArrowUp}");

      expect(selected()).toEqual([all[all.length - 1]]);
    });

    it("wraps from the last entry to the first on ArrowDown", async () => {
      await renderPalette();
      const all = screen.getAllByRole("option");

      await userEvent.keyboard("{ArrowUp}{ArrowDown}");

      expect(selected()).toEqual([all[0]]);
    });
  });

  describe("choosing an entry", () => {
    it("opens an issue in the full Issue view over its Space's view, with the issue selected and no peek", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] } });
      await search("contact");
      await screen.findByRole("option", { name: /NW-43/ });

      await userEvent.keyboard("{Enter}");

      await waitFor(() => expect(useStore.getState().issueView).toEqual({ origin: { listId: NW_ID, viewId: 0 }, stack: ["10043"] }));
      expect(useStore.getState().places["11:0"]).toMatchObject({ selectedId: "10043", peekOpen: false });
      expect(mocked.navGo).toHaveBeenCalledWith(NW_ID, null);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("closes without navigating when the issue's project has no Space", async () => {
      await renderPalette({ results: { issues: [issueHit({ projectId: "99999" })], lists: [] } });
      await search("contact");
      await screen.findByRole("option", { name: /NW-43/ });

      await userEvent.keyboard("{Enter}");

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(mocked.navGo).not.toHaveBeenCalled();
      expect(useStore.getState().issueView).toBeNull();
    });

    it("opens a filter list", async () => {
      await renderPalette({ results: { issues: [], lists: [listHit()] } });
      await search("sprint");
      await userEvent.click(await screen.findByRole("option", { name: /Current sprint/ }));

      expect(mocked.navGo).toHaveBeenCalledWith(21, null);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("opens a view of a list, passing the list and the view", async () => {
      await renderPalette({ results: { views: [viewHit()] } });
      await search("board");
      await userEvent.click(await screen.findByRole("option", { name: /Sprint board/ }));

      expect(mocked.navGo).toHaveBeenCalledWith(21, 201);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("runs the chosen command and closes", async () => {
      await renderPalette({ menu: FILE_MENU });
      await search("refresh");

      await userEvent.click(screen.getByRole("option", { name: /Refresh now/ }));

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:refresh");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("runs the selected command on Enter", async () => {
      await renderPalette({ menu: FILE_MENU });
      await search(">");

      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:full");
    });

    it("refreshes on ⌘R", async () => {
      await renderPalette();

      await userEvent.keyboard("{Meta>}r{/Meta}");

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:refresh");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
  });

  describe("issue shortcuts", () => {
    const withIssue = async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] }, siteUrl: "https://example.atlassian.net/" });
      await search("contact");
      await screen.findByRole("option", { name: /Open NW-43 in Jira/ });
    };

    it("opens the active issue in Jira on ⌘↩", async () => {
      await withIssue();

      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-43");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("keeps the palette open on ⌘↩ without a connected site", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] } });
      await search("contact");
      await screen.findByRole("option", { name: /NW-43/ });

      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      expect(mocked.openUrl).not.toHaveBeenCalled();
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    it("copies the active issue's key on ⌘C", async () => {
      await withIssue();

      await userEvent.keyboard("{Meta>}c{/Meta}");

      expect(mocked.copyText).toHaveBeenCalledWith("NW-43");
    });

    it("leaves ⌘C to the field while some of its text is selected", async () => {
      await withIssue();
      input().setSelectionRange(0, 3);

      await userEvent.keyboard("{Meta>}c{/Meta}");

      expect(mocked.copyText).not.toHaveBeenCalled();
    });
  });

  describe("saving the search", () => {
    it.each<[string, Nav["location"], number]>([
      ["the current Space", { listId: NW_ID, viewId: 1 }, NW_ID],
      ["the Space of the current list", { listId: SPRINT.id, viewId: 1 }, NW_ID],
      ["Everything when no Space is open", null, TREE.everything.id],
    ])("creates a filter list under %s and opens it on ⌘S", async (_, location, parent) => {
      await renderPalette({ location });
      await search("contact");

      await userEvent.keyboard("{Meta>}s{/Meta}");

      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(NEW_LIST_ID, null));
      expect(mocked.listCreate).toHaveBeenCalledWith(parent, "Search: contact", { conditions: [{ field: "text", op: "contains", value: "contact" }] });
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("saves from the action entry as well", async () => {
      await renderPalette();
      await search("contact");

      await userEvent.click(screen.getByRole("option", { name: /Save “contact”/ }));

      await waitFor(() => expect(mocked.navGo).toHaveBeenCalledWith(NEW_LIST_ID, null));
    });

    it("does nothing on ⌘S without text", async () => {
      await renderPalette();

      await userEvent.keyboard("{Meta>}s{/Meta}");

      expect(mocked.listCreate).not.toHaveBeenCalled();
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    it("shows the error and opens nothing when the list cannot be created", async () => {
      await renderPalette();
      mocked.listCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await search("contact");

      await userEvent.keyboard("{Meta>}s{/Meta}");

      await waitFor(() => expect(mocked.listCreate).toHaveBeenCalled());
      expect(mocked.navGo).not.toHaveBeenCalled();
    });
  });
});
