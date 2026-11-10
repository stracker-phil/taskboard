import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type InboxRow } from "../api";
import { installCommands, useCommandKeys } from "../commands";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { inboxOf, inboxRow, THREE_ROWS } from "../test/inbox";
import { nav } from "../test/nav";
import { InboxButton } from "./InboxButton";
import { useInboxState } from "./inboxState";

const mocked = vi.mocked(api, true);

function Harness() {
  useCommandKeys();
  return <InboxButton />;
}

async function openInbox(rows: InboxRow[] = THREE_ROWS) {
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
  await userEvent.click(screen.getByRole("button", { name: /^Notifications/ }));
  const dialog = await screen.findByRole("dialog", { name: "Notifications" });
  await act(async () => {});
  return dialog;
}

const rowsOf = (dialog: HTMLElement) => within(within(dialog).getByRole("list", { name: "Unread issues" })).getAllByRole("listitem");
const selectedIndex = (dialog: HTMLElement) => rowsOf(dialog).findIndex((r) => r.getAttribute("aria-current") === "true");
const isClosed = () => expect(screen.queryByRole("dialog", { name: "Notifications" })).not.toBeInTheDocument();

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  useInboxState.setState({ open: false, selected: 0 });
  installCommands("main");
  mocked.navCurrent.mockResolvedValue(nav({ listId: 11, viewId: 101 }));
});

describe("the open inbox's keys", () => {
  describe("selection", () => {
    it("starts on the first row", async () => {
      const dialog = await openInbox();

      expect(selectedIndex(dialog)).toBe(0);
    });

    it.each([
      ["J", "j", "k"],
      ["ArrowDown", "{ArrowDown}", "{ArrowUp}"],
    ])("%s moves down one row and its counterpart moves back up", async (_name, down, up) => {
      const dialog = await openInbox();

      await userEvent.keyboard(down);
      expect(selectedIndex(dialog)).toBe(1);
      await userEvent.keyboard(down);
      expect(selectedIndex(dialog)).toBe(2);
      await userEvent.keyboard(up);

      expect(selectedIndex(dialog)).toBe(1);
    });

    it("stops at the first and the last row", async () => {
      const dialog = await openInbox();

      await userEvent.keyboard("k");
      expect(selectedIndex(dialog)).toBe(0);
      await userEvent.keyboard("jjjj");

      expect(selectedIndex(dialog)).toBe(2);
    });

    it("follows the pointer", async () => {
      const dialog = await openInbox();

      await userEvent.hover(rowsOf(dialog)[2]);

      expect(selectedIndex(dialog)).toBe(2);
    });

    it("starts on the first row again after it was closed and opened", async () => {
      const dialog = await openInbox();
      await userEvent.keyboard("jj");
      await userEvent.keyboard("{Escape}");
      await waitFor(isClosed);

      await userEvent.click(screen.getByRole("button", { name: /^Notifications/ }));

      expect(selectedIndex(await screen.findByRole("dialog", { name: "Notifications" }))).toBe(0);
      expect(dialog).not.toBeInTheDocument();
    });
  });

  describe("M", () => {
    it("marks an unread row as read", async () => {
      await openInbox();

      await userEvent.keyboard("m");

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
      expect(mocked.issueMarkUnread).not.toHaveBeenCalled();
    });

    it("marks a read row as unread", async () => {
      await openInbox([inboxRow(), inboxRow({ id: "10043", key: "NW-1043", unread: false })]);
      await userEvent.keyboard("j");

      await userEvent.keyboard("m");

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("10043");
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });

    it("acts on the row selected by the pointer", async () => {
      const dialog = await openInbox();

      await userEvent.hover(rowsOf(dialog)[2]);
      await userEvent.keyboard("m");

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10044"]);
    });

    it("shows an error toast when Rust cannot change the read state", async () => {
      await openInbox();
      mocked.issuesMarkSeen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      await userEvent.keyboard("m");

      await waitFor(() => expect(useStore.getState().toasts.some((t) => t.message.includes("Disk full"))).toBe(true));
    });
  });

  describe("opening an issue", () => {
    it("Enter opens the selected issue in this tab and closes the inbox", async () => {
      await openInbox();
      await userEvent.keyboard("j");

      await userEvent.keyboard("{Enter}");

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["10043"] });
      await waitFor(isClosed);
      expect(mocked.inboxClose).toHaveBeenCalled();
    });

    it("clicking a row opens its issue and closes the inbox", async () => {
      const dialog = await openInbox();

      await userEvent.click(rowsOf(dialog)[2]);

      expect(useStore.getState().issueView?.stack).toEqual(["10044"]);
      await waitFor(isClosed);
      expect(mocked.inboxClose).toHaveBeenCalled();
    });

    it("Mod+Enter asks for a new tab at this place, leaves this tab's Issue view alone and closes the inbox", async () => {
      await openInbox();

      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      await waitFor(() => expect(mocked.navTabNew).toHaveBeenCalledWith(11, 101, false));
      expect(useStore.getState().issueView).toBeNull();
      await waitFor(isClosed);
      expect(mocked.inboxClose).toHaveBeenCalled();
    });
  });

  describe("marking everything read", () => {
    it("Mod+Shift+A tells Rust", async () => {
      await openInbox();

      await userEvent.keyboard("{Meta>}{Shift>}a{/Shift}{/Meta}");

      expect(mocked.inboxMarkAllSeen).toHaveBeenCalled();
    });

    it("the Mark all as read button tells Rust and keeps the inbox open", async () => {
      const dialog = await openInbox();

      await userEvent.click(within(dialog).getByRole("button", { name: "Mark all as read" }));

      expect(mocked.inboxMarkAllSeen).toHaveBeenCalled();
      expect(screen.getByRole("dialog", { name: "Notifications" })).toBeInTheDocument();
    });
  });

  describe("while closed", () => {
    it("J and M do nothing to the inbox", async () => {
      await openInbox();
      await userEvent.keyboard("{Escape}");
      await waitFor(isClosed);

      await userEvent.keyboard("jm");

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      expect(mocked.issueMarkUnread).not.toHaveBeenCalled();
    });
  });
});
