import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { commentTarget, issueTarget, listTarget, pinItem, pins } from "../test/pins";
import { PinnedView } from "./PinnedView";

const mocked = vi.mocked(api, true);

const ISSUE = pinItem({ id: 1, reason: "Needs a decision", timeLeft: "2d left" });
const COMMENT = pinItem({ id: 2, target: commentTarget(), reason: "Rules to follow", timeLeft: "today" });
const LIST = pinItem({ id: 3, target: listTarget(), reason: null, timeLeft: "" });

function renderPinned() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PinnedView listId={3} viewId={0} />
    </QueryClientProvider>,
  );
}

const pinnedTable = () => screen.findByRole("table", { name: "Bookmarked" });
const rowOf = async (text: RegExp) => within(await pinnedTable()).getByRole("row", { name: text });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.pinsList.mockResolvedValue(pins());
});

describe("PinnedView", () => {
  it("renders nothing until the pins have loaded", () => {
    mocked.pinsList.mockReturnValue(new Promise(() => {}));
    renderPinned();

    expect(screen.queryByRole("table")).not.toBeInTheDocument();
  });

  it("shows the empty state when nothing is pinned", async () => {
    renderPinned();

    expect(await screen.findByText(/^Nothing pinned/)).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
  });

  it("shows the Bookmarked, Reason and Time left columns", async () => {
    mocked.pinsList.mockResolvedValue(pins([ISSUE]));
    renderPinned();

    const table = within(await pinnedTable());
    expect(table.getAllByRole("columnheader").map((h) => h.textContent)).toEqual(["Bookmarked", "Reason", "Time left"]);
  });

  describe("rows", () => {
    beforeEach(() => {
      mocked.pinsList.mockResolvedValue(pins([ISSUE, COMMENT, LIST]));
    });

    it("shows an issue with its key and summary, reason and time left", async () => {
      renderPinned();

      const row = within(await rowOf(/Redesign the pricing page/));
      expect(row.getByText("NW-1042")).toBeInTheDocument();
      expect(row.getByText("Needs a decision")).toBeInTheDocument();
      expect(row.getByText("2d left")).toBeInTheDocument();
    });

    it("marks an issue that is gone", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ target: issueTarget({ gone: true }) })]));
      renderPinned();

      expect(within(await rowOf(/Redesign the pricing page/)).getByText("gone")).toBeInTheDocument();
    });

    it("shows a comment with its author, excerpt and issue key", async () => {
      renderPinned();

      const row = within(await rowOf(/Sam Lee/));
      expect(row.getByText("Please check the postcode rules first.")).toBeInTheDocument();
      expect(row.getByText("NW-1042")).toBeInTheDocument();
      expect(row.getByText("Rules to follow")).toBeInTheDocument();
    });

    it("names the author Someone when the comment has none", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ target: commentTarget({ author: null }) })]));
      renderPinned();

      expect(await rowOf(/Someone/)).toBeInTheDocument();
    });

    it("shows a filter list with its Space and name", async () => {
      renderPinned();

      const row = within(await rowOf(/My open issues/));
      expect(row.getByText("Website relaunch /")).toBeInTheDocument();
    });

    it("shows a global filter list without a Space", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ target: listTarget({ space: null }) })]));
      renderPinned();

      const row = await rowOf(/My open issues/);
      expect(row).not.toHaveTextContent("/");
    });
  });

  describe("opening a row", () => {
    beforeEach(() => {
      mocked.pinsList.mockResolvedValue(pins([ISSUE, COMMENT, LIST]));
    });

    it("opens the Issue view for an issue", async () => {
      renderPinned();

      await userEvent.click(await rowOf(/Redesign the pricing page/));

      expect(useStore.getState().issueView).toEqual({ origin: { listId: 3, viewId: 0 }, stack: ["10042"] });
    });

    it("opens the commented issue for a comment", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 2, target: commentTarget({ issueId: "10077" }) })]));
      renderPinned();

      await userEvent.click(await rowOf(/Sam Lee/));

      expect(useStore.getState().issueView?.stack).toEqual(["10077"]);
    });

    it("navigates to the list for a filter list", async () => {
      renderPinned();

      await userEvent.click(await rowOf(/My open issues/));

      expect(mocked.navGo).toHaveBeenCalledWith(21, null);
      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("removing a bookmark", () => {
    it("ends the bookmark from the row's Remove bookmark button without opening the row", async () => {
      mocked.pinsList.mockResolvedValue(pins([ISSUE, COMMENT]));
      renderPinned();

      await userEvent.click(within(await rowOf(/Sam Lee/)).getByRole("button", { name: "Remove bookmark" }));

      expect(mocked.pinRemove).toHaveBeenCalledWith(2);
      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("recently removed", () => {
    it("is left out when no bookmark ended", async () => {
      mocked.pinsList.mockResolvedValue(pins([ISSUE]));
      renderPinned();

      await pinnedTable();
      expect(screen.queryByRole("region", { name: "Recently removed" })).not.toBeInTheDocument();
    });

    it("lists the ended bookmarks below, without Remove bookmark buttons or time left", async () => {
      const ended = pinItem({ id: 5, target: issueTarget({ id: "10099", key: "NW-1099", summary: "Archive the old banner" }), reason: "Done with it", timeLeft: "" });
      mocked.pinsList.mockResolvedValue(pins([ISSUE], [ended]));
      renderPinned();

      const section = within(await screen.findByRole("region", { name: "Recently removed" }));
      const row = within(section.getByRole("row", { name: /NW-1099/ }));
      expect(row.getByText("Archive the old banner")).toBeInTheDocument();
      expect(row.getByText("Done with it")).toBeInTheDocument();
      expect(row.queryByRole("button", { name: "Remove bookmark" })).not.toBeInTheDocument();
    });

    it("shows the empty state above it when only ended pins exist", async () => {
      mocked.pinsList.mockResolvedValue(pins([], [pinItem({ id: 5 })]));
      renderPinned();

      expect(await screen.findByText(/^Nothing pinned/)).toBeInTheDocument();
      expect(screen.getByRole("region", { name: "Recently removed" })).toBeInTheDocument();
    });

    it("opens the issue of an ended bookmark when its row is clicked", async () => {
      mocked.pinsList.mockResolvedValue(pins([], [pinItem({ id: 5 })]));
      renderPinned();

      await userEvent.click(within(await screen.findByRole("region", { name: "Recently removed" })).getByRole("row", { name: /NW-1042/ }));

      expect(useStore.getState().issueView?.stack).toEqual(["10042"]);
    });
  });
});
