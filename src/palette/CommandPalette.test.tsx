import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type ListNode, type Nav, type PaletteResults, type SidebarTree } from "../api";
import { installCommands, useCommandKeys } from "../commands";
import { createQueryClient } from "../queries/queryClient";
import { NAV_KEY, SIDEBAR_KEY } from "../queries/sidebar";
import { resetStore, useStore } from "../state/store";
import { connectionView } from "../test/jira";
import { issueHit, listHit, parts } from "../test/palette";
import { sidebarTree, space } from "../test/spaces";
import { CommandPalette } from "./CommandPalette";

const mocked = vi.mocked(api, true);

const SPRINT: ListNode = { id: 21, kind: "list", name: "Current sprint", count: null, ownConditions: 1, description: "", error: null, icon: null, color: null };
const TREE: SidebarTree = sidebarTree([space({ lists: [SPRINT] })]);
const NW_ID = 11;
const NEW_LIST_ID = 31;

interface Setup {
  results?: PaletteResults;
  tree?: SidebarTree;
  location?: Nav["location"];
  siteUrl?: string;
}

function Keys() {
  useCommandKeys();
  return null;
}

async function renderPalette({ results = { issues: [], lists: [] }, tree = TREE, location = null, siteUrl = "" }: Setup = {}) {
  mocked.paletteQuery.mockResolvedValue(results);
  mocked.sidebarTree.mockResolvedValue(tree);
  mocked.navCurrent.mockResolvedValue({ location, expanded: [] });
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl }));
  mocked.listCreate.mockResolvedValue(NEW_LIST_ID);
  mocked.viewSetGroup.mockResolvedValue(undefined as never);
  const client = createQueryClient();
  render(
    <QueryClientProvider client={client}>
      <Keys />
      <CommandPalette />
    </QueryClientProvider>,
  );
  if (useStore.getState().paletteOpen) {
    await waitFor(() => {
      expect(client.getQueryData(SIDEBAR_KEY)).toBeDefined();
      expect(client.getQueryData(NAV_KEY)).toBeDefined();
    });
  }
  return client;
}

const input = () => screen.getByRole<HTMLInputElement>("combobox");
const options = (region?: string) => within(region ? screen.getByRole("region", { name: region }) : screen.getByRole("listbox")).getAllByRole("option");
const selected = () => screen.getAllByRole("option").filter((o) => o.getAttribute("aria-selected") === "true");

async function search(text: string) {
  await userEvent.type(input(), text);
}

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
    it("asks for no results and lists actions only", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [listHit()] } });

      expect(mocked.paletteQuery).not.toHaveBeenCalled();
      expect(screen.queryByRole("region", { name: "Issues" })).not.toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "Filter lists" })).not.toBeInTheDocument();
      expect(screen.getByRole("option", { name: /Refresh now/ })).toBeInTheDocument();
      expect(screen.getByRole("option", { name: "Full sync" })).toBeInTheDocument();
    });

    it("lists the titled commands but not the palette's own", async () => {
      await renderPalette();

      expect(screen.getByRole("option", { name: "Toggle sidebar" })).toBeInTheDocument();
      expect(screen.queryByRole("option", { name: "Command palette" })).not.toBeInTheDocument();
    });

    it("shows at most six actions", async () => {
      await renderPalette({ location: { listId: NW_ID, viewId: 5 } });

      expect(options("Actions")).toHaveLength(6);
    });
  });

  describe("searching", () => {
    it("asks Rust for the typed text", async () => {
      await renderPalette();

      await search("contact");

      await waitFor(() => expect(mocked.paletteQuery).toHaveBeenLastCalledWith("contact"));
    });

    it("asks once for text typed in a burst, not for each prefix", async () => {
      await renderPalette();

      await userEvent.type(input(), "abc", { delay: null });

      await waitFor(() => expect(mocked.paletteQuery).toHaveBeenCalledWith("abc"));
      expect(mocked.paletteQuery.mock.calls.map(([text]) => text)).toEqual(["abc"]);
    });

    it("keeps the previous results listed while the next ones load", async () => {
      await renderPalette({ results: { issues: [issueHit()], lists: [] } });
      await search("contact");
      await screen.findByRole("option", { name: /NW-43/ });
      mocked.paletteQuery.mockReturnValue(new Promise(() => {}));

      await userEvent.type(input(), "s");
      await waitFor(() => expect(mocked.paletteQuery).toHaveBeenLastCalledWith("contacts"));

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

    it("counts the issues and the actions", async () => {
      await renderPalette({ results: { issues: [issueHit(), issueHit({ id: "10044", key: "NW-44" })], lists: [] } });

      await search("form");

      await screen.findAllByRole("option", { name: /NW-4/ });
      const actions = options("Actions").length;
      expect(screen.getByText(`2 issues · ${actions} ${actions === 1 ? "action" : "actions"}`)).toBeInTheDocument();
    });

    it("offers to save the text as a filter list", async () => {
      await renderPalette();

      await search("contact");

      expect(screen.getByRole("option", { name: /Save “contact” as a filter list/ })).toBeInTheDocument();
    });

    it("filters the actions by the text but keeps the save action", async () => {
      await renderPalette();

      await search("refresh");

      expect(screen.getByRole("option", { name: /Refresh now/ })).toBeInTheDocument();
      expect(screen.getByRole("option", { name: /Save “refresh”/ })).toBeInTheDocument();
      expect(screen.queryByRole("option", { name: "Full sync" })).not.toBeInTheDocument();
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

    it("runs the chosen action and closes", async () => {
      await renderPalette();

      await userEvent.click(screen.getByRole("option", { name: /Refresh now/ }));

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:refresh");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("runs the selected action on Enter", async () => {
      await renderPalette();

      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:full");
    });

    it("regroups the current view from a Group by action", async () => {
      await renderPalette({ location: { listId: NW_ID, viewId: 5 } });
      await search("group by status");

      await userEvent.click(screen.getByRole("option", { name: "Group by Status" }));

      expect(mocked.viewSetGroup).toHaveBeenCalledWith(5, "status");
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
