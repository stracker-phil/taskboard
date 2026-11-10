import { QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type BulkOutcome, type BulkStatus, type Behaviour, type IssueChange } from "../api";
import { MenuHost } from "../components/menu";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { ALEX, SAM } from "../test/issues";
import { issueTarget, pinItem, pins } from "../test/pins";
import { behaviour, settings } from "../test/settings";
import { BulkBar, openBulkMenu } from "./BulkBar";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);
const allowChanges = (patch: Partial<Behaviour> = {}) =>
  mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, ...patch }) }));
const outcome = (patch: Partial<BulkOutcome> = {}): BulkOutcome => ({ done: "Moved to Done: 3 issues", changed: 3, refused: 0, reverse: null, ...patch });
const moves = (ids: string[], transitionId: string): IssueChange[] => ids.map((issueId) => ({ issueId, change: { kind: "transition", transitionId } }));
const DONE: BulkStatus = { statusName: "Done", style: "done", changes: moves(["1", "3"], "31") };
const IN_REVIEW: BulkStatus = { statusName: "In Review", style: "review", changes: moves(["1", "2", "3"], "21") };

function tree(picked: string[], onClear = () => {}) {
  return (
    <QueryClientProvider client={createQueryClient()}>
      <BulkBar picked={picked} onClear={onClear} />
      <MenuHost />
    </QueryClientProvider>
  );
}

function renderBar(picked = ["1", "2", "3"], onClear = () => {}) {
  return render(tree(picked, onClear));
}

/** The settings and the bookmarks have reached the bar. */
async function loaded() {
  await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
  await waitFor(() => expect(mocked.pinsList).toHaveBeenCalled());
  await act(async () => {
    await mocked.getSettings.mock.results[0]?.value;
    await mocked.pinsList.mock.results[0]?.value;
  });
}

/** A bar button, once the settings have switched it on. */
async function enabled(name: string) {
  const button = await screen.findByRole("button", { name });
  await waitFor(() => expect(button).toBeEnabled());
  return button;
}

async function openMenuOf(button: string, menu: string, picked?: string[]) {
  renderBar(picked);
  await userEvent.click(await enabled(button));
  return screen.findByRole("menu", { name: menu });
}

const item = (menu: HTMLElement, name: string | RegExp) => within(menu).getByRole("menuitem", { name });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
  mocked.pinsList.mockResolvedValue(pins());
  mocked.issuesBulkStatuses.mockResolvedValue([DONE, IN_REVIEW]);
  mocked.issuesBulkPriorities.mockResolvedValue([
    { name: "Highest", level: 1 },
    { name: "High", level: 2 },
    { name: "Low", level: 4 },
  ]);
  mocked.issuesBulkAssignees.mockResolvedValue([ALEX, SAM]);
  mocked.issuesWriteMany.mockResolvedValue(outcome());
});

describe("BulkBar", () => {
  describe("showing", () => {
    it("is hidden while nothing is picked", () => {
      renderBar([]);

      expect(screen.queryByRole("toolbar", { name: "Picked issues" })).not.toBeInTheDocument();
    });

    it.each([
      [["1"], "1 picked"],
      [["1", "2", "3"], "3 picked"],
    ])("shows the count of %j as %s", (picked, count) => {
      renderBar(picked);

      expect(within(screen.getByRole("toolbar", { name: "Picked issues" })).getByText(count)).toBeInTheDocument();
    });

    it("appears when issues get picked and goes when they are dropped", async () => {
      const { rerender } = renderBar([]);

      rerender(tree(["1"]));
      expect(await screen.findByRole("toolbar", { name: "Picked issues" })).toBeInTheDocument();

      rerender(tree([]));
      await waitFor(() => expect(screen.queryByRole("toolbar", { name: "Picked issues" })).not.toBeInTheDocument());
    });

    it("offers the three menus, Mark Read, Bookmark and the drop button", () => {
      renderBar();

      const bar = screen.getByRole("toolbar", { name: "Picked issues" });
      expect(within(bar).getAllByRole("button").map((b) => b.getAttribute("aria-label") ?? b.textContent)).toEqual([
        "Status",
        "Assign",
        "Priority",
        "Mark Read",
        "Bookmark",
        "Drop the picks",
      ]);
    });
  });

  describe("switched-off changes", () => {
    it("enables the three menu buttons while changes to Jira are on", async () => {
      renderBar();

      for (const name of ["Status", "Assign", "Priority"]) expect(await enabled(name)).toBeEnabled();
    });

    it("keeps the three menu buttons, disabled and explained, while changes to Jira are off", async () => {
      allowChanges({ allowJiraChanges: false });
      renderBar();
      await loaded();

      for (const name of ["Status", "Assign", "Priority"]) {
        const button = screen.getByRole("button", { name });
        expect(button).toBeDisabled();
        expect(button).toHaveAttribute("title", `${name}: switched off in Settings → Connection`);
      }
    });

    it.each([
      ["changeStatus", "Status"],
      ["assign", "Assign"],
      ["changePriority", "Priority"],
    ] as const)("disables only %s's button when that change is switched off", async (capability, label) => {
      allowChanges({ disabledChanges: [capability] });
      renderBar();
      await loaded();

      expect(screen.getByRole("button", { name: label })).toBeDisabled();
      expect(screen.getByRole("button", { name: label })).toHaveAttribute("title", `${label}: switched off in Settings → Connection`);
      for (const other of ["Status", "Assign", "Priority"].filter((name) => name !== label)) {
        await waitFor(() => expect(screen.getByRole("button", { name: other })).toBeEnabled());
      }
    });

    it("leaves Mark Read, Bookmark and the drop button on while changes to Jira are off", async () => {
      allowChanges({ allowJiraChanges: false });
      renderBar();
      await loaded();

      for (const name of ["Mark Read", "Bookmark", "Drop the picks"]) expect(screen.getByRole("button", { name })).toBeEnabled();
    });
  });

  describe("Status", () => {
    it("loads the statuses of the picked issues and lists them under a caption", async () => {
      const menu = await openMenuOf("Status", "Status of the picked issues");

      expect(mocked.issuesBulkStatuses).toHaveBeenCalledWith(["1", "2", "3"]);
      expect(within(menu).getByRole("group", { name: "Move 3 issues to" })).toBeInTheDocument();
      expect(within(menu).getAllByRole("menuitem").map((i) => i.textContent)).toEqual(["Done", "In Review"]);
    });

    it("words the caption for a single issue", async () => {
      const menu = await openMenuOf("Status", "Status of the picked issues", ["1"]);

      expect(within(menu).getByRole("group", { name: "Move 1 issue to" })).toBeInTheDocument();
    });

    it("sends the chosen status's changes with the total and toasts what Rust reports", async () => {
      mocked.issuesWriteMany.mockResolvedValue(outcome({ done: "Moved to Done: 2 of 3 issues", changed: 2, refused: 0 }));
      const menu = await openMenuOf("Status", "Status of the picked issues");

      await userEvent.click(item(menu, /Done/));

      expect(mocked.issuesWriteMany).toHaveBeenCalledWith(DONE.changes, "Moved to Done", 3);
      await waitFor(() => expect(toasts()).toEqual([["success", "Moved to Done: 2 of 3 issues"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("shows one disabled row when the picked issues share no status to move to", async () => {
      mocked.issuesBulkStatuses.mockResolvedValue([]);
      const menu = await openMenuOf("Status", "Status of the picked issues");

      const row = item(menu, "No status all of them can move to");
      expect(row).toHaveAttribute("aria-disabled", "true");
      await userEvent.click(row);
      expect(mocked.issuesWriteMany).not.toHaveBeenCalled();
    });

    it("toasts an error and opens no menu when the statuses cannot be loaded", async () => {
      mocked.issuesBulkStatuses.mockRejectedValue({ kind: "io", message: "Network is down", code: null });
      renderBar();

      await userEvent.click(await enabled("Status"));

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not load the statuses: Network is down"]]));
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Status" })).toHaveAttribute("aria-busy", "false");
    });
  });

  describe("the outcome toast", () => {
    const choose = async () => userEvent.click(item(await openMenuOf("Status", "Status of the picked issues"), /Done/));

    it.each([
      ["success", { changed: 3, refused: 0 }],
      ["info", { changed: 2, refused: 1 }],
      ["error", { changed: 0, refused: 3 }],
    ] as const)("is %s when %j", async (kind, counts) => {
      mocked.issuesWriteMany.mockResolvedValue(outcome({ done: "Moved to Done", ...counts }));

      await choose();

      await waitFor(() => expect(toasts()).toEqual([[kind, "Moved to Done"]]));
    });

    it("offers no Change back when Rust gives no reverse", async () => {
      await choose();

      await waitFor(() => expect(useStore.getState().toasts).toHaveLength(1));
      expect(useStore.getState().toasts[0].action).toBeUndefined();
    });

    it("runs the reverse as a new change when Change back is pressed", async () => {
      const reverse = moves(["1", "3"], "11");
      mocked.issuesWriteMany.mockResolvedValueOnce(outcome({ changed: 2, reverse })).mockResolvedValueOnce(outcome({ done: "Changed back: 2 issues", changed: 2 }));
      await choose();
      await waitFor(() => expect(useStore.getState().toasts[0]?.action?.label).toBe("Change back"));

      act(() => useStore.getState().toasts[0].action?.run());

      expect(mocked.issuesWriteMany).toHaveBeenLastCalledWith(reverse, "Changed back", 2);
      await waitFor(() => expect(toasts()).toContainEqual(["success", "Changed back: 2 issues"]));
    });

    it("toasts an error when Rust fails to write", async () => {
      mocked.issuesWriteMany.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      await choose();

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not change the issues: Disk full"]]));
    });
  });

  describe("Priority", () => {
    it("lists the priorities under a caption", async () => {
      const menu = await openMenuOf("Priority", "Priority of the picked issues");

      expect(mocked.issuesBulkPriorities).toHaveBeenCalledWith(["1", "2", "3"]);
      expect(within(menu).getByRole("group", { name: "Set 3 issues to" })).toBeInTheDocument();
      expect(within(menu).getAllByRole("menuitem").map((i) => i.textContent)).toEqual(["Highest", "High", "Low"]);
    });

    it("builds one priority change per picked issue", async () => {
      const menu = await openMenuOf("Priority", "Priority of the picked issues", ["1", "3"]);

      await userEvent.click(item(menu, /High$/));

      expect(mocked.issuesWriteMany).toHaveBeenCalledWith(
        [
          { issueId: "1", change: { kind: "priority", name: "High" } },
          { issueId: "3", change: { kind: "priority", name: "High" } },
        ],
        "Set to High",
        2,
      );
    });

    it("shows one disabled row when the picked issues share no priority", async () => {
      mocked.issuesBulkPriorities.mockResolvedValue([]);
      const menu = await openMenuOf("Priority", "Priority of the picked issues");

      expect(item(menu, "No priority all of them offer")).toHaveAttribute("aria-disabled", "true");
    });
  });

  describe("Assign", () => {
    it("lists the team, then Unassigned after a separator", async () => {
      const menu = await openMenuOf("Assign", "Assignee of the picked issues");

      expect(mocked.issuesBulkAssignees).toHaveBeenCalledWith(["1", "2", "3"]);
      expect(within(menu).getByRole("group", { name: "Assign 3 issues to" })).toBeInTheDocument();
      const rows = within(menu).getAllByRole("menuitem");
      expect(rows).toHaveLength(3);
      expect(rows[0]).toHaveTextContent("Alex Morgan");
      expect(rows[1]).toHaveTextContent("Sam Lee");
      expect(rows[2]).toHaveTextContent("Unassigned");
    });

    it("builds one assign change per picked issue for the chosen person", async () => {
      const menu = await openMenuOf("Assign", "Assignee of the picked issues", ["1", "2"]);

      await userEvent.click(item(menu, /Sam Lee/));

      expect(mocked.issuesWriteMany).toHaveBeenCalledWith(
        [
          { issueId: "1", change: { kind: "assign", accountId: "demo-account-2" } },
          { issueId: "2", change: { kind: "assign", accountId: "demo-account-2" } },
        ],
        "Assigned to Sam Lee",
        2,
      );
    });

    it("sends accountId null for Unassigned", async () => {
      const menu = await openMenuOf("Assign", "Assignee of the picked issues", ["1", "2"]);

      await userEvent.click(item(menu, "Unassigned"));

      expect(mocked.issuesWriteMany).toHaveBeenCalledWith(
        [
          { issueId: "1", change: { kind: "assign", accountId: null } },
          { issueId: "2", change: { kind: "assign", accountId: null } },
        ],
        "Unassigned",
        2,
      );
    });
  });

  describe("while a menu loads", () => {
    it("marks its button busy and disables all three until the choices arrive", async () => {
      let arrive: (statuses: BulkStatus[]) => void = () => {};
      mocked.issuesBulkStatuses.mockReturnValue(new Promise((resolve) => (arrive = resolve)));
      renderBar();

      await userEvent.click(await enabled("Status"));

      expect(screen.getByRole("button", { name: /Status$/ })).toHaveAttribute("aria-busy", "true");
      expect(screen.getByRole("button", { name: "Assign" })).toHaveAttribute("aria-busy", "false");
      for (const name of [/Status$/, "Assign", "Priority"]) expect(screen.getByRole("button", { name })).toBeDisabled();

      await act(async () => arrive([DONE]));

      expect(await screen.findByRole("menu", { name: "Status of the picked issues" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Status" })).toHaveAttribute("aria-busy", "false");
      expect(screen.getByRole("button", { name: "Assign" })).toBeEnabled();
    });

    describe("spinner", () => {
      const pending = <T,>() => {
        let resolve: (value: T) => void = () => {};
        let reject: (reason: unknown) => void = () => {};
        const promise = new Promise<T>((res, rej) => {
          resolve = res;
          reject = rej;
        });
        return { promise, resolve: (value: T) => resolve(value), reject: (reason: unknown) => reject(reason) };
      };
      const statusButton = () => screen.getByRole("button", { name: /Status$/ });

      it("holds a progressbar in the Status button, which keeps its label and says it asks Jira", async () => {
        const load = pending<BulkStatus[]>();
        mocked.issuesBulkStatuses.mockReturnValue(load.promise);
        renderBar();

        await userEvent.click(await enabled("Status"));

        const button = statusButton();
        expect(within(button).getByRole("progressbar", { name: "Loading status choices" })).toBeInTheDocument();
        expect(button).toHaveTextContent("Status");
        expect(button).toHaveAttribute("title", "Status: asking Jira…");
      });

      it("goes and leaves the opened menu once the choices arrive", async () => {
        const load = pending<BulkStatus[]>();
        mocked.issuesBulkStatuses.mockReturnValue(load.promise);
        renderBar();
        await userEvent.click(await enabled("Status"));

        await act(async () => load.resolve([DONE]));

        expect(await screen.findByRole("menu", { name: "Status of the picked issues" })).toBeInTheDocument();
        expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Status" })).not.toHaveAttribute("title", "Status: asking Jira…");
      });

      it("goes and frees the button again when the choices cannot be loaded", async () => {
        const load = pending<BulkStatus[]>();
        mocked.issuesBulkStatuses.mockReturnValue(load.promise);
        renderBar();
        await userEvent.click(await enabled("Status"));

        await act(async () => load.reject({ kind: "io", message: "Network is down", code: null }));

        await waitFor(() => expect(screen.queryByRole("progressbar")).not.toBeInTheDocument());
        expect(screen.getByRole("button", { name: "Status" })).toBeEnabled();
        expect(toasts()).toEqual([["error", "Could not load the statuses: Network is down"]]);
      });

      it("shows the Priority button's own progressbar and none on the other buttons", async () => {
        const load = pending<{ name: string; level: number }[]>();
        mocked.issuesBulkPriorities.mockReturnValue(load.promise);
        renderBar();

        await userEvent.click(await enabled("Priority"));

        const bar = screen.getByRole("toolbar", { name: "Picked issues" });
        const bars = within(bar).getAllByRole("progressbar");
        expect(bars).toHaveLength(1);
        expect(bars[0]).toHaveAccessibleName("Loading priority choices");
        expect(screen.getByRole("button", { name: /Priority$/ })).toContainElement(bars[0]);
        expect(screen.getByRole("button", { name: /Priority$/ })).toHaveAttribute("title", "Priority: asking Jira…");
      });
    });
  });

  describe("Mark Read", () => {
    it("marks all picked issues read and says how many", async () => {
      renderBar();

      await userEvent.click(screen.getByRole("button", { name: "Mark Read" }));

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["1", "2", "3"]);
      await waitFor(() => expect(toasts()).toEqual([["success", "Marked 3 issues read"]]));
    });

    it("says issue in the singular for one", async () => {
      renderBar(["2"]);

      await userEvent.click(screen.getByRole("button", { name: "Mark Read" }));

      await waitFor(() => expect(toasts()).toEqual([["success", "Marked 1 issue read"]]));
    });

    it("toasts an error and no success when marking fails", async () => {
      mocked.issuesMarkSeen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderBar();

      await userEvent.click(screen.getByRole("button", { name: "Mark Read" }));

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not mark them read: Disk full"]]));
    });
  });

  describe("Bookmark", () => {
    const bookmarkedIds = () => mocked.pinCreate.mock.calls.map(([, id]) => id);

    it("bookmarks the picked issues that are not bookmarked yet, with the default duration", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ target: issueTarget({ id: "2", key: "NW-1001" }) })]));
      renderBar();
      await loaded();

      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      await waitFor(() => expect(toasts()).toEqual([["success", "Bookmarked 2 issues"]]));
      expect(bookmarkedIds()).toEqual(["1", "3"]);
      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "1", "untilUnpinned", null);
    });

    it("says issue in the singular for one", async () => {
      renderBar(["1"]);
      await loaded();

      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      await waitFor(() => expect(toasts()).toEqual([["success", "Bookmarked 1 issue"]]));
    });

    it("creates nothing and says so when all are bookmarked already", async () => {
      mocked.pinsList.mockResolvedValue(
        pins([1, 2, 3].map((n) => pinItem({ id: n, target: issueTarget({ id: String(n), key: `NW-100${n}` }) }))),
      );
      renderBar();
      await loaded();

      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      await waitFor(() => expect(toasts()).toEqual([["info", "They are all bookmarked already"]]));
      expect(mocked.pinCreate).not.toHaveBeenCalled();
    });

    it("stops with an error when a bookmark cannot be created", async () => {
      mocked.pinCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderBar();
      await loaded();

      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not bookmark it: Disk full"]]));
      expect(bookmarkedIds()).toEqual(["1"]);
    });
  });

  describe("the drop button", () => {
    it("asks the view to drop the picks", async () => {
      const onClear = vi.fn();
      renderBar(["1", "2"], onClear);

      await userEvent.click(screen.getByRole("button", { name: "Drop the picks" }));

      expect(onClear).toHaveBeenCalled();
    });
  });
});

describe("openBulkMenu", () => {
  it("opens the menu of the bar's button, as a click would", async () => {
    renderBar();
    await enabled("Priority");

    await act(async () => openBulkMenu("priority"));

    expect(await screen.findByRole("menu", { name: "Priority of the picked issues" })).toBeInTheDocument();
  });

  it("explains that changes are off instead of opening a menu when the button is disabled", async () => {
    allowChanges({ allowJiraChanges: false });
    renderBar();
    await loaded();

    await act(async () => openBulkMenu("status"));

    expect(toasts()).toEqual([["info", "Changes to Jira are off. Turn them on in Settings → Connection."]]);
    expect(mocked.issuesBulkStatuses).not.toHaveBeenCalled();
  });

  it("does nothing while another menu is loading", async () => {
    mocked.issuesBulkPriorities.mockReturnValue(new Promise(() => {}));
    renderBar();
    await userEvent.click(await enabled("Priority"));

    await act(async () => openBulkMenu("status"));

    expect(mocked.issuesBulkStatuses).not.toHaveBeenCalled();
    expect(toasts()).toEqual([]);
  });

  it("does nothing without a bar", () => {
    openBulkMenu("assign");

    expect(mocked.issuesBulkAssignees).not.toHaveBeenCalled();
    expect(toasts()).toEqual([]);
  });
});
