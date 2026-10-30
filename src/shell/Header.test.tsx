import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { runPageAction } from "../commands/pageActions";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { connectionView } from "../test/jira";
import { listTarget, pinItem, pins } from "../test/pins";
import { space } from "../test/spaces";
import { listNode, view } from "../test/views";
import { Header } from "./Header";
import type { Level } from "./levels";

const mocked = vi.mocked(api, true);

const SPACE_LEVEL: Level = { kind: "space", space: space() };
const LIST_LEVEL: Level = { kind: "list", space: space(), node: listNode() };
const EVERYTHING: Level = { kind: "home", node: listNode({ id: 1, kind: "everything", name: "Everything" }) };
const PINNED: Level = { kind: "home", node: listNode({ id: 3, kind: "pinned", name: "Pinned" }) };

const recent = (count: number | null): Level => ({ kind: "home", node: listNode({ id: 2, kind: "recent", name: "Unread", count }) });

async function renderHeader(level: Level) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Header level={level} />
    </QueryClientProvider>,
  );
  await waitFor(() => expect(mocked.connectionGet).toHaveBeenCalled());
  await act(async () => {
    await mocked.connectionGet.mock.results[0].value;
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.connectionGet.mockResolvedValue(connectionView());
  mocked.pinsList.mockResolvedValue(pins());
  mocked.listMarkAllSeen.mockResolvedValue(undefined);
  mocked.openUrl.mockResolvedValue(undefined);
});

describe("Header", () => {
  describe("title", () => {
    it("shows a Space's name as the heading", async () => {
      await renderHeader(SPACE_LEVEL);

      expect(screen.getByRole("heading", { level: 1, name: "Website relaunch" })).toBeInTheDocument();
    });

    it("shows a filter list as a breadcrumb button with its Space, then its own name as the heading", async () => {
      await renderHeader(LIST_LEVEL);

      expect(screen.getByRole("button", { name: "Website relaunch" })).toBeInTheDocument();
      expect(screen.getByRole("heading", { level: 1, name: "My open issues" })).toBeInTheDocument();
    });

    it("opens the Space's own level when the breadcrumb is clicked", async () => {
      await renderHeader(LIST_LEVEL);

      await userEvent.click(screen.getByRole("button", { name: "Website relaunch" }));

      expect(mocked.navGo).toHaveBeenCalledWith(11, null);
    });

    it.each([
      ["Everything", EVERYTHING],
      ["Pinned", PINNED],
    ])("shows the name of the Home level %s as the heading", async (name, level) => {
      await renderHeader(level);

      expect(screen.getByRole("heading", { level: 1, name })).toBeInTheDocument();
    });
  });

  describe("renaming from the title", () => {
    beforeEach(() => {
      mocked.spaceUpdate.mockResolvedValue(undefined);
      mocked.listRename.mockResolvedValue(undefined);
    });

    it("renames a Space when its name is clicked", async () => {
      await renderHeader(SPACE_LEVEL);

      await userEvent.click(screen.getByRole("button", { name: "Website relaunch" }));
      const field = screen.getByRole("textbox", { name: "Space name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Public site{Enter}");

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", expect.objectContaining({ name: "Public site" }));
      expect(screen.queryByRole("textbox", { name: "Space name" })).not.toBeInTheDocument();
    });

    it("renames a filter list when its name is clicked", async () => {
      await renderHeader(LIST_LEVEL);

      await userEvent.click(screen.getByRole("button", { name: "My open issues" }));
      const field = screen.getByRole("textbox", { name: "Filter list name" });
      await userEvent.clear(field);
      await userEvent.type(field, "Backlog{Enter}");

      expect(mocked.listRename).toHaveBeenCalledWith(expect.any(Number), "Backlog");
    });

    it("keeps the name when the rename is canceled", async () => {
      await renderHeader(SPACE_LEVEL);

      await userEvent.click(screen.getByRole("button", { name: "Website relaunch" }));
      await userEvent.type(screen.getByRole("textbox", { name: "Space name" }), "Other{Escape}");

      expect(mocked.spaceUpdate).not.toHaveBeenCalled();
      expect(screen.getByRole("heading", { level: 1, name: "Website relaunch" })).toBeInTheDocument();
    });

    it.each([
      ["Everything", EVERYTHING],
      ["Pinned", PINNED],
    ])("leaves the Home level %s's heading plain", async (name, level) => {
      await renderHeader(level);

      expect(screen.getByRole("heading", { level: 1, name })).toBeInTheDocument();
      expect(within(screen.getByRole("heading", { level: 1, name })).queryByRole("button")).not.toBeInTheDocument();
    });
  });

  describe("icon button", () => {
    it.each([
      ["a Space", SPACE_LEVEL, "Website relaunch"],
      ["a filter list", LIST_LEVEL, "My open issues"],
    ])("opens the Icon and color popover for %s", async (_name, level, name) => {
      await renderHeader(level);

      await userEvent.click(screen.getByRole("button", { name: `Icon and color of ${name}` }));

      expect(screen.getByRole("dialog", { name: "Icon and color" })).toBeInTheDocument();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });
  });

  describe("bookmark button", () => {
    const BOOKMARK = { name: /^(Bookmark this|Remove bookmark)/ };

    it.each([
      ["a Space", SPACE_LEVEL, "Bookmark this Space", "11"],
      ["a filter list", LIST_LEVEL, "Bookmark this list", "21"],
      ["a Home list", { kind: "home", node: listNode({ id: 31 }) } satisfies Level, "Bookmark this list", "31"],
    ])("bookmarks %s as a filter list at once on an alt-click", async (_name, level, label, target) => {
      await renderHeader(level);

      fireEvent.click(await screen.findByRole("button", { name: label }), { altKey: true });

      expect(mocked.pinCreate).toHaveBeenCalledWith("filterList", target, "untilUnpinned", null);
    });

    it.each([
      ["Everything", EVERYTHING],
      ["Unread", recent(3)],
      ["Bookmarks", PINNED],
    ])("is absent on the Home level %s", async (_name, level) => {
      await renderHeader(level);

      expect(screen.queryByRole("button", BOOKMARK)).not.toBeInTheDocument();
    });

    it("sits after the title and before Open in Jira", async () => {
      await renderHeader(SPACE_LEVEL);

      const button = await screen.findByRole("button", { name: "Bookmark this Space" });

      expect(screen.getByRole("heading", { level: 1 }).compareDocumentPosition(button) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(button.compareDocumentPosition(screen.getByRole("button", { name: /Open in Jira/ })) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it("is pressed and removes the bookmark when the level is bookmarked", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 9, target: listTarget({ id: 21 }) })]));
      await renderHeader(LIST_LEVEL);

      const button = await screen.findByRole("button", { name: "Remove bookmark" });
      expect(button).toHaveAttribute("aria-pressed", "true");
      await userEvent.click(button);

      expect(mocked.pinRemove).toHaveBeenCalledWith(9);
    });

    it("is not pressed while another list is the one bookmarked", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 9, target: listTarget({ id: 99 }) })]));
      await renderHeader(LIST_LEVEL);

      expect(await screen.findByRole("button", { name: "Bookmark this list" })).toHaveAttribute("aria-pressed", "false");
    });
  });

  describe("Mark all as read", () => {
    it("is offered by the header of the Unread level", async () => {
      await renderHeader(recent(3));

      expect(screen.getByRole("heading", { level: 1, name: "Unread" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Mark all as read" })).toBeEnabled();
    });

    it("marks every issue of Unread read when no view of it is open", async () => {
      await renderHeader(recent(3));

      await userEvent.click(screen.getByRole("button", { name: "Mark all as read" }));

      expect(mocked.listMarkAllSeen).toHaveBeenCalledWith(2, null, null);
    });

    it("passes the open view and its narrowing when Unread is on screen", async () => {
      mocked.navCurrent.mockResolvedValue({ location: { listId: 2, viewId: 7 }, expanded: [] });
      mocked.viewsOf.mockResolvedValue([view({ id: 7, listId: 2, onlyMine: true })]);
      await renderHeader(recent(3));
      await waitFor(() => expect(mocked.viewsOf).toHaveBeenCalledWith(2));
      await act(async () => {});

      await userEvent.click(screen.getByRole("button", { name: "Mark all as read" }));

      expect(mocked.listMarkAllSeen).toHaveBeenCalledWith(2, 7, expect.objectContaining({ me: true, text: "" }));
    });

    it("passes no view when another level is open", async () => {
      mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
      await renderHeader(recent(3));
      await act(async () => {});

      await userEvent.click(screen.getByRole("button", { name: "Mark all as read" }));

      expect(mocked.listMarkAllSeen).toHaveBeenCalledWith(2, null, null);
    });

    it.each([0, null])("is disabled when the unseen count is %s", async (count) => {
      await renderHeader(recent(count));

      expect(screen.getByRole("button", { name: "Mark all as read" })).toBeDisabled();
    });

    it.each([
      ["Everything", EVERYTHING],
      ["a Space", SPACE_LEVEL],
      ["a filter list", LIST_LEVEL],
    ])("is absent on %s", async (_name, level) => {
      await renderHeader(level);

      expect(screen.queryByRole("button", { name: "Mark all as read" })).not.toBeInTheDocument();
    });

    it("shows an error toast when marking fails", async () => {
      mocked.listMarkAllSeen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderHeader(recent(3));

      await userEvent.click(screen.getByRole("button", { name: "Mark all as read" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark them read: Disk full"));
    });

    describe("as a page action (recent.markAllSeen)", () => {
      const run = () => {
        let offered = false;
        act(() => {
          offered = runPageAction("recent.markAllSeen");
        });
        return offered;
      };

      it("marks every issue of Unread read", async () => {
        await renderHeader(recent(3));

        expect(run()).toBe(true);

        expect(mocked.listMarkAllSeen).toHaveBeenCalledWith(2, null, null);
      });

      it.each([0, null])("is not offered when the unseen count is %s", async (count) => {
        await renderHeader(recent(count));

        expect(run()).toBe(false);
        expect(mocked.listMarkAllSeen).not.toHaveBeenCalled();
      });

      it.each([
        ["Everything", EVERYTHING],
        ["a Space", SPACE_LEVEL],
      ])("is not offered on %s", async (_name, level) => {
        await renderHeader(level);

        expect(run()).toBe(false);
      });

      it("is withdrawn once every issue is read", async () => {
        const view = render(
          <QueryClientProvider client={createQueryClient()}>
            <Header level={recent(3)} />
          </QueryClientProvider>,
        );
        expect(run()).toBe(true);

        view.rerender(
          <QueryClientProvider client={createQueryClient()}>
            <Header level={recent(0)} />
          </QueryClientProvider>,
        );

        expect(run()).toBe(false);
      });

      it("shows an error toast when marking fails", async () => {
        mocked.listMarkAllSeen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
        await renderHeader(recent(3));

        run();

        await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark them read: Disk full"));
      });
    });
  });

  describe("Open in Jira", () => {
    it.each([
      ["a Space", SPACE_LEVEL],
      ["a filter list", LIST_LEVEL],
    ])("opens the Space's project page without a double slash for %s", async (_name, level) => {
      await renderHeader(level);

      await userEvent.click(await screen.findByRole("button", { name: /Open in Jira/ }));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW");
    });

    it.each([
      ["Everything", EVERYTHING],
      ["Pinned", PINNED],
    ])("is hidden on the Home level %s", async (_name, level) => {
      await renderHeader(level);

      expect(screen.queryByRole("button", { name: /Open in Jira/ })).not.toBeInTheDocument();
    });

    it("is hidden while no Jira site is set up", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));

      await renderHeader(SPACE_LEVEL);

      expect(screen.queryByRole("button", { name: /Open in Jira/ })).not.toBeInTheDocument();
    });

    it("shows an error toast when Jira cannot be opened", async () => {
      mocked.openUrl.mockRejectedValue({ kind: "io", message: "No browser", code: null });
      await renderHeader(SPACE_LEVEL);

      await userEvent.click(await screen.findByRole("button", { name: /Open in Jira/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not open Jira: No browser"));
    });
  });
});
