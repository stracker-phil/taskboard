import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type IssuePullRequests, type PullRequest } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { prStatus, pullRequest } from "../test/pullRequests";
import { PullRequests } from "./PullRequests";

const mocked = vi.mocked(api, true);

const answer = (items: PullRequest[], refreshInMs: number | null = null): IssuePullRequests => ({ items, refreshInMs });

const second = () => pullRequest({ id: "30003", number: 476, url: "https://github.com/example/shop/pull/476", title: "Format postcodes in the summary" });

function renderPullRequests(variant: "issue" | "peek" = "issue") {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PullRequests id="10042" variant={variant} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("PullRequests", () => {
  it.each(["issue", "peek"] as const)("renders nothing in the %s variant when there are no pull requests", async (variant) => {
    renderPullRequests(variant);

    await vi.waitFor(() => expect(mocked.issuePullRequests).toHaveBeenCalledWith("10042"));
    expect(screen.queryByRole("region", { name: "Pull requests" })).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Pull requests" })).not.toBeInTheDocument();
  });

  describe("rows", () => {
    it("shows a pull request as its number and title, not its url", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()]));
      renderPullRequests();

      const row = within(await screen.findByRole("button", { name: /Validate DACH postcodes/ }));
      expect(row.getByText("#481")).toBeInTheDocument();
      expect(screen.queryByText("https://github.com/example/shop/pull/481")).not.toBeInTheDocument();
    });

    it("labels a single pull request in the issue variant as Pull request", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()]));
      renderPullRequests("issue");

      expect(await screen.findByText("Pull request")).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: "Pull requests" })).not.toBeInTheDocument();
    });

    it("labels only the first of several pull requests in the issue variant as Pull requests", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest(), second()]));
      renderPullRequests("issue");

      expect(await screen.findByText("Pull requests")).toBeInTheDocument();
      expect(screen.getAllByText("Pull requests")).toHaveLength(1);
      expect(screen.getByRole("button", { name: /Validate DACH postcodes/ })).toHaveTextContent("#481");
      expect(screen.getByRole("button", { name: /Format postcodes in the summary/ })).toHaveTextContent("#476");
    });

    it("shows a Pull requests heading in the peek variant instead of a row label", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest(), second()]));
      renderPullRequests("peek");

      expect(await screen.findByRole("heading", { name: "Pull requests" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Format postcodes in the summary/ })).toBeInTheDocument();
      expect(screen.queryByText("Pull request")).not.toBeInTheDocument();
    });
  });

  describe("status", () => {
    it.each([
      ["open", "Open"],
      ["draft", "Draft"],
      ["merged", "Merged"],
      ["closed", "Closed"],
    ] as const)("shows the %s state as the %s pill", async (state, label) => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest({ status: prStatus({ state }) })]));
      renderPullRequests();

      expect(await screen.findByText(label)).toBeInTheDocument();
    });

    it("shows how many checks passed out of how many", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest({ status: prStatus({ checks: { state: "failing", passed: 9, total: 12 } }) })]));
      renderPullRequests();

      expect(await screen.findByText("9/12")).toBeInTheDocument();
    });

    it.each([
      ["approved", "Approved"],
      ["changesRequested", "Changes requested"],
      ["waiting", "Awaiting review"],
    ] as const)("shows the %s review as %s", async (review, label) => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest({ status: prStatus({ review }) })]));
      renderPullRequests();

      expect(await screen.findByText(label)).toBeInTheDocument();
    });

    it("shows no checks mark when there are no checks and no review text when there is no review", async () => {
      mocked.issuePullRequests.mockResolvedValue(
        answer([pullRequest({ status: prStatus({ checks: { state: "none", passed: 0, total: 0 }, review: "none" }) })]),
      );
      renderPullRequests();

      expect(await screen.findByText("Open")).toBeInTheDocument();
      expect(screen.queryByText(/\d+\/\d+/)).not.toBeInTheDocument();
      for (const text of ["Approved", "Changes requested", "Awaiting review"]) expect(screen.queryByText(text)).not.toBeInTheDocument();
    });

    it("says No status when there is no status but a problem", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest({ problem: "gh is not signed in" })]));
      renderPullRequests();

      expect(await screen.findByText("No status")).toBeInTheDocument();
    });

    it("shows no status text when there is neither a status nor a problem", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()]));
      renderPullRequests();

      await screen.findByRole("button", { name: /Validate DACH postcodes/ });
      expect(screen.queryByText("No status")).not.toBeInTheDocument();
    });
  });

  describe("opening", () => {
    it("opens an openable row with its url", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()]));
      renderPullRequests();

      await userEvent.click(await screen.findByRole("button", { name: /Validate DACH postcodes/ }));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://github.com/example/shop/pull/481");
    });

    it("shows a row that is not openable as text, not a button", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest({ openable: false, url: "http://intranet.example.com/pull/481" })]));
      renderPullRequests();

      await userEvent.click(await screen.findByText("Validate DACH postcodes"));

      expect(screen.queryByRole("button")).not.toBeInTheDocument();
      expect(mocked.openUrl).not.toHaveBeenCalled();
    });
  });

  describe("syncing", () => {
    const MINUTES_5 = 300_000;

    beforeEach(() => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
    });

    it("syncs at once when a status is due now and then shows the synced status", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()], 0));
      mocked.issueSyncPullRequests.mockResolvedValue(answer([pullRequest({ status: prStatus({ state: "merged" }) })]));
      renderPullRequests();

      expect(await screen.findByText("Merged")).toBeInTheDocument();
      expect(mocked.issueSyncPullRequests).toHaveBeenCalledWith("10042");
    });

    it("syncs after the time Rust gave and not before", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()], MINUTES_5));
      renderPullRequests();
      await screen.findByText("#481");

      await vi.advanceTimersByTimeAsync(MINUTES_5 - 5_000);
      expect(mocked.issueSyncPullRequests).not.toHaveBeenCalled();

      await vi.advanceTimersByTimeAsync(10_000);
      expect(mocked.issueSyncPullRequests).toHaveBeenCalledWith("10042");
    });

    it("never syncs when Rust says nothing is due", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()], null));
      renderPullRequests();
      await screen.findByText("#481");

      await vi.advanceTimersByTimeAsync(MINUTES_5 * 3);

      expect(mocked.issueSyncPullRequests).not.toHaveBeenCalled();
    });

    it("syncs again 5 minutes after a sync answer that asks for it", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest()], 0));
      mocked.issueSyncPullRequests
        .mockResolvedValueOnce(answer([pullRequest({ status: prStatus({ state: "open" }) })], MINUTES_5))
        .mockResolvedValueOnce(answer([pullRequest({ status: prStatus({ state: "merged" }) })], null));
      renderPullRequests();
      await screen.findByText("Open");

      await vi.advanceTimersByTimeAsync(MINUTES_5 - 5_000);
      expect(screen.queryByText("Merged")).not.toBeInTheDocument();

      await vi.advanceTimersByTimeAsync(10_000);
      expect(await screen.findByText("Merged")).toBeInTheDocument();
    });

    it("keeps showing the cached pull requests when a sync fails", async () => {
      mocked.issuePullRequests.mockResolvedValue(answer([pullRequest({ status: prStatus() })], 0));
      mocked.issueSyncPullRequests.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      renderPullRequests();

      expect(await screen.findByText("Open")).toBeInTheDocument();
      await vi.advanceTimersByTimeAsync(10);
      expect(screen.getByRole("button", { name: /Validate DACH postcodes/ })).toBeInTheDocument();
    });
  });
});
