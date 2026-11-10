import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type InboxRow } from "../api";
import { installCommands, useCommandKeys } from "../commands";
import { createQueryClient } from "../queries/queryClient";
import { useEvents } from "../queries/useEvents";
import { resetStore, useStore } from "../state/store";
import { emitEvent } from "../test/apiMock";
import { inboxOf, THREE_ROWS } from "../test/inbox";
import { issueDetail } from "../test/issues";
import { nav } from "../test/nav";
import { behaviour, settings } from "../test/settings";
import { InboxButton } from "./InboxButton";
import { useInboxState } from "./inboxState";

const mocked = vi.mocked(api, true);

function Harness() {
  useCommandKeys();
  useEvents();
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

const card = (key: string) => screen.findByRole("region", { name: `Peek at ${key}` });
const noCard = () => expect(screen.queryByRole("region", { name: /^Peek at/ })).not.toBeInTheDocument();
const isClosed = () => expect(screen.queryByRole("dialog", { name: "Notifications" })).not.toBeInTheDocument();

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  useInboxState.setState({ open: false, selected: 0, previewing: false });
  installCommands("main");
  vi.useFakeTimers({ shouldAdvanceTime: true });
  mocked.navCurrent.mockResolvedValue(nav({ listId: 11, viewId: 101 }));
  mocked.issueGet.mockImplementation(async (id: string) => {
    const row = THREE_ROWS.find((r) => r.id === id) ?? THREE_ROWS[0];
    return issueDetail({ id: row.id, key: row.key, summary: row.summary });
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("the inbox preview", () => {
  describe("opening and closing", () => {
    it("Space shows the selected issue's card beside the inbox, and Space again hides it", async () => {
      await openInbox();

      await userEvent.keyboard(" ");
      const found = await card("NW-1042");
      expect(within(found).getByText("Redesign the pricing page")).toBeInTheDocument();
      await userEvent.keyboard(" ");

      await waitFor(noCard);
      expect(screen.getByRole("dialog", { name: "Notifications" })).toBeInTheDocument();
    });

    it("shows nothing until Space is pressed", async () => {
      await openInbox();
      await act(async () => {});

      noCard();
    });

    it("J moves the card to the next issue", async () => {
      await openInbox();
      await userEvent.keyboard(" ");
      await card("NW-1042");

      await userEvent.keyboard("j");

      const next = await card("NW-1043");
      expect(within(next).getByText("Publish the launch notes")).toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "Peek at NW-1042" })).not.toBeInTheDocument();
    });

    it("Escape closes the card first and leaves the inbox open, a second Escape closes the inbox", async () => {
      await openInbox();
      await userEvent.keyboard(" ");
      await card("NW-1042");

      await userEvent.keyboard("{Escape}");

      await waitFor(noCard);
      expect(screen.getByRole("dialog", { name: "Notifications" })).toBeInTheDocument();
      expect(mocked.inboxClose).not.toHaveBeenCalled();
      await userEvent.keyboard("{Escape}");
      await waitFor(isClosed);
      expect(mocked.inboxClose).toHaveBeenCalled();
    });

    it("Enter opens the previewed issue and closes the card and the inbox", async () => {
      await openInbox();
      await userEvent.keyboard("j");
      await userEvent.keyboard(" ");
      await card("NW-1043");

      await userEvent.keyboard("{Enter}");

      expect(useStore.getState().issueView?.stack).toEqual(["10043"]);
      await waitFor(isClosed);
      noCard();
      expect(mocked.inboxClose).toHaveBeenCalled();
    });

    it("a press inside the card does not close the inbox", async () => {
      await openInbox();
      await userEvent.keyboard(" ");
      const found = await card("NW-1042");

      await userEvent.click(within(found).getByText("Redesign the pricing page"));

      expect(screen.getByRole("dialog", { name: "Notifications" })).toBeInTheDocument();
      expect(mocked.inboxClose).not.toHaveBeenCalled();
      expect(screen.getByRole("region", { name: "Peek at NW-1042" })).toBeInTheDocument();
    });
  });

  describe("counting as used", () => {
    it("tells Rust the previewed issue was previewed once the delay has passed", async () => {
      await openInbox();
      await userEvent.keyboard(" ");
      await card("NW-1042");
      expect(mocked.issuePreviewed).not.toHaveBeenCalled();

      await act(() => vi.advanceTimersByTimeAsync(2000));

      expect(mocked.issuePreviewed).toHaveBeenCalledWith("10042");
    });

    it("does not count an issue the preview left sooner with J", async () => {
      await openInbox();
      await userEvent.keyboard(" ");
      await card("NW-1042");
      await act(() => vi.advanceTimersByTimeAsync(1000));

      await userEvent.keyboard("j");
      await card("NW-1043");
      await act(() => vi.advanceTimersByTimeAsync(3000));

      expect(mocked.issuePreviewed).not.toHaveBeenCalledWith("10042");
    });
  });

  describe("counting as read", () => {
    it("marks the previewed unread issue read after the default delay and keeps its row listed", async () => {
      const dialog = await openInbox();
      await userEvent.keyboard(" ");
      await card("NW-1042");
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();

      await act(() => vi.advanceTimersByTimeAsync(2000));

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
      expect(within(dialog).getAllByRole("listitem")).toHaveLength(3);
    });

    it("waits for the delay from the settings", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ seenAfterSeconds: 5 }) }));
      await openInbox();
      await userEvent.keyboard(" ");
      await card("NW-1042");

      await act(() => vi.advanceTimersByTimeAsync(4000));
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      await act(() => vi.advanceTimersByTimeAsync(1500));

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("marks nothing read for an issue the preview left sooner with J", async () => {
      await openInbox();
      await userEvent.keyboard(" ");
      await card("NW-1042");
      await act(() => vi.advanceTimersByTimeAsync(1000));

      await userEvent.keyboard("j");
      await card("NW-1043");
      await act(() => vi.advanceTimersByTimeAsync(3000));

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalledWith(["10042"]);
    });

    it("keeps a row unread that M marked unread while it is previewed", async () => {
      await openInbox([{ ...THREE_ROWS[0], unread: false }, THREE_ROWS[1], THREE_ROWS[2]]);
      await userEvent.keyboard(" ");
      await card("NW-1042");

      await userEvent.keyboard("m");
      mocked.inboxRows.mockResolvedValue(inboxOf(THREE_ROWS));
      emitEvent(api, "issues_changed", { ids: ["10042"], all: false });
      await waitFor(() => expect(screen.getByText("3 unread")).toBeInTheDocument());
      await act(() => vi.advanceTimersByTimeAsync(5000));

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("10042");
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });

    it("marks nothing read while the card is closed", async () => {
      await openInbox();

      await act(() => vi.advanceTimersByTimeAsync(5000));

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });
  });
});
