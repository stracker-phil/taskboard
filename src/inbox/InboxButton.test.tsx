import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type InboxRow } from "../api";
import { installCommands, useCommandKeys } from "../commands";
import { runPageAction } from "../commands/pageActions";
import { createQueryClient } from "../queries/queryClient";
import { useEvents } from "../queries/useEvents";
import { resetStore } from "../state/store";
import { emitEvent } from "../test/apiMock";
import { inboxOf, inboxRow, THREE_ROWS } from "../test/inbox";
import { nav } from "../test/nav";
import { InboxButton } from "./InboxButton";
import { useInboxState } from "./inboxState";

const mocked = vi.mocked(api, true);

function Harness() {
  useCommandKeys();
  useEvents();
  return (
    <>
      <InboxButton />
      <button type="button">Elsewhere</button>
    </>
  );
}

async function renderInbox(rows: InboxRow[] = THREE_ROWS) {
  const snapshot = inboxOf(rows);
  mocked.inboxOpen.mockResolvedValue(snapshot);
  mocked.inboxRows.mockResolvedValue(snapshot);
  mocked.inboxCount.mockResolvedValue(snapshot.unread);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness />
    </QueryClientProvider>,
  );
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));
}

const bell = () => screen.getByRole("button", { name: /^Notifications/ });
const dialog = () => screen.queryByRole("dialog", { name: "Notifications" });

async function openInbox() {
  await userEvent.click(bell());
  const found = await screen.findByRole("dialog", { name: "Notifications" });
  await act(async () => {});
  return found;
}

const rowsOf = (dlg: HTMLElement) => within(within(dlg).getByRole("list", { name: "Unread issues" })).getAllByRole("listitem");

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  useInboxState.setState({ open: false, selected: 0 });
  installCommands("main");
  mocked.navCurrent.mockResolvedValue(nav({ listId: 11, viewId: 101 }));
});

describe("InboxButton", () => {
  describe("bell", () => {
    it("shows the unread count as a badge and names it", async () => {
      await renderInbox();

      const found = await screen.findByRole("button", { name: "Notifications, 3 unread" });

      expect(within(found).getByText("3")).toBeInTheDocument();
    });

    it("shows no badge when nothing is unread", async () => {
      await renderInbox([]);
      await waitFor(() => expect(mocked.inboxCount).toHaveBeenCalled());
      await act(async () => {});

      expect(bell()).toHaveAccessibleName("Notifications");
      expect(bell().textContent).toBe("");
    });

    it.each([
      [99, "99"],
      [100, "99+"],
      [250, "99+"],
    ])("shows %i unread as %s", async (count, shown) => {
      await renderInbox();
      mocked.inboxCount.mockResolvedValue(count);
      emitEvent(api, "issues_changed", { ids: [], all: true });

      const found = await screen.findByRole("button", { name: `Notifications, ${count} unread` });

      expect(within(found).getByText(shown)).toBeInTheDocument();
    });

    it("names its shortcut in the tooltip", async () => {
      await renderInbox();

      expect(bell()).toHaveAttribute("title", "Notifications (⌘⇧U)");
    });
  });

  describe("opening", () => {
    it("lists every row with key, title, note and how long ago it changed, in the order Rust gave", async () => {
      await renderInbox();

      const rows = rowsOf(await openInbox());

      expect(rows).toHaveLength(3);
      expect(rows[0]).toHaveTextContent("NW-1042");
      expect(rows[0]).toHaveTextContent("Redesign the pricing page");
      expect(rows[0]).toHaveTextContent("Moved to Review, 1 new comment.");
      expect(rows[0]).toHaveTextContent("5 min ago");
      expect(rows[1]).toHaveTextContent("NW-1043");
      expect(rows[1]).toHaveTextContent("Assigned to you.");
      expect(rows[1]).toHaveTextContent("2 h ago");
      expect(rows[2]).toHaveTextContent("NW-1044");
      expect(rows[2]).toHaveTextContent("30 min ago");
    });

    it("shows the unread count in the header and marks the bell expanded", async () => {
      await renderInbox();

      const dlg = await openInbox();

      expect(within(dlg).getByText("3 unread")).toBeInTheDocument();
      expect(bell()).toHaveAttribute("aria-expanded", "true");
      expect(within(dlg).getByRole("button", { name: "Mark all as read" })).toBeEnabled();
    });

    it("takes a new snapshot each time it opens", async () => {
      await renderInbox();
      await openInbox();
      await userEvent.keyboard("{Escape}");
      await waitFor(() => expect(dialog()).not.toBeInTheDocument());
      const next = inboxOf([THREE_ROWS[2]]);
      mocked.inboxOpen.mockResolvedValue(next);
      mocked.inboxRows.mockResolvedValue(next);

      const rows = rowsOf(await openInbox());

      expect(rows).toHaveLength(1);
      expect(rows[0]).toHaveTextContent("NW-1044");
    });

    it("says so and disables Mark all as read when nothing is unread", async () => {
      await renderInbox([]);

      const dlg = await openInbox();

      expect(within(dlg).getByText("You're all caught up.")).toBeInTheDocument();
      expect(within(dlg).getByRole("button", { name: "Mark all as read" })).toBeDisabled();
    });

    it("stays closed when Rust cannot take the snapshot", async () => {
      await renderInbox();
      mocked.inboxOpen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      await userEvent.click(bell());

      await waitFor(() => expect(mocked.inboxOpen).toHaveBeenCalled());
      await act(async () => {});
      expect(dialog()).not.toBeInTheDocument();
    });
  });

  describe("reading while open", () => {
    it("keeps a row that was read in place, dimmed, after the rows are fetched again", async () => {
      await renderInbox();
      const dlg = await openInbox();
      mocked.inboxRows.mockResolvedValue(inboxOf([{ ...THREE_ROWS[0], unread: false }, THREE_ROWS[1], THREE_ROWS[2]]));

      emitEvent(api, "issues_changed", { ids: ["10042"], all: false });

      await waitFor(() => expect(rowsOf(dlg)[0]).toHaveClass("inbox__row--read"));
      const rows = rowsOf(dlg);
      expect(rows.map((r) => r.textContent?.match(/NW-\d+/)?.[0])).toEqual(["NW-1042", "NW-1043", "NW-1044"]);
      expect(rows[1]).not.toHaveClass("inbox__row--read");
      expect(within(dlg).getByText("2 unread")).toBeInTheDocument();
    });

    it("disables Mark all as read once every row is read", async () => {
      await renderInbox();
      const dlg = await openInbox();
      mocked.inboxRows.mockResolvedValue(inboxOf(THREE_ROWS.map((r) => ({ ...r, unread: false }))));

      emitEvent(api, "issues_changed", { ids: [], all: true });

      await waitFor(() => expect(within(dlg).getByRole("button", { name: "Mark all as read" })).toBeDisabled());
      expect(rowsOf(dlg)).toHaveLength(3);
    });

    it("shows a row that was read before it opened as read", async () => {
      await renderInbox([inboxRow(), inboxRow({ id: "10043", key: "NW-1043", unread: false })]);

      const rows = rowsOf(await openInbox());

      expect(rows[0]).not.toHaveClass("inbox__row--read");
      expect(rows[1]).toHaveClass("inbox__row--read");
    });
  });

  describe("closing", () => {
    it("closes on Escape, tells Rust and gives the focus back to the bell", async () => {
      await renderInbox();
      await openInbox();

      await userEvent.keyboard("{Escape}");

      await waitFor(() => expect(dialog()).not.toBeInTheDocument());
      expect(mocked.inboxClose).toHaveBeenCalled();
      expect(bell()).toHaveFocus();
    });

    it("closes when the bell is pressed again", async () => {
      await renderInbox();
      await openInbox();

      await userEvent.click(bell());

      await waitFor(() => expect(dialog()).not.toBeInTheDocument());
      expect(mocked.inboxClose).toHaveBeenCalled();
    });

    it("closes on a press outside", async () => {
      await renderInbox();
      await openInbox();

      await userEvent.click(screen.getByRole("button", { name: "Elsewhere" }));

      await waitFor(() => expect(dialog()).not.toBeInTheDocument());
      expect(mocked.inboxClose).toHaveBeenCalled();
    });

    it("stays open on a press inside", async () => {
      await renderInbox();
      const dlg = await openInbox();

      await userEvent.click(within(dlg).getByText("Notifications"));

      expect(dialog()).toBeInTheDocument();
      expect(mocked.inboxClose).not.toHaveBeenCalled();
    });
  });

  describe("commands", () => {
    it("toggles with the inbox.toggle shortcut", async () => {
      await renderInbox();

      await userEvent.keyboard("{Meta>}{Shift>}u{/Shift}{/Meta}");
      expect(await screen.findByRole("dialog", { name: "Notifications" })).toBeInTheDocument();
      await userEvent.keyboard("{Meta>}{Shift>}u{/Shift}{/Meta}");

      await waitFor(() => expect(dialog()).not.toBeInTheDocument());
      expect(mocked.inboxClose).toHaveBeenCalled();
    });

    it("opens through the inbox.open page action, and stays open when it runs again", async () => {
      await renderInbox();

      let offered = false;
      await act(async () => {
        offered = runPageAction("inbox.open");
      });
      await screen.findByRole("dialog", { name: "Notifications" });
      await act(async () => {
        runPageAction("inbox.open");
      });

      expect(offered).toBe(true);
      expect(dialog()).toBeInTheDocument();
      expect(mocked.inboxClose).not.toHaveBeenCalled();
    });
  });
});
