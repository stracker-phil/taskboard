import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type SyncState, type TokenExpiry } from "../api";
import { clockTime } from "../lib/relativeTime";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { syncOverview } from "../test/jira";
import { TokenBanner } from "./TokenBanner";

const mocked = vi.mocked(api, true);

const LAST_FULL = "2026-03-04T09:30:00Z";
const PAUSED: SyncState = { state: "paused", message: "Jira rejected the token." };

async function setup(expiry: TokenExpiry, state: SyncState = { state: "idle", lastOk: null }, lastFullAt: string | null = LAST_FULL) {
  const view = await mocked.connectionGet();
  mocked.connectionGet.mockResolvedValue({ ...view, expiry });
  mocked.syncOverview.mockResolvedValue(syncOverview({ state, lastFullAt }));
}

function renderBanner() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TokenBanner />
    </QueryClientProvider>,
  );
}

async function settled() {
  await waitFor(() => {
    expect(mocked.connectionGet).toHaveBeenCalled();
    expect(mocked.syncOverview).toHaveBeenCalled();
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  localStorage.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("TokenBanner", () => {
  describe("while Jira rejects the token", () => {
    it("shows a red alert with the time of the cached data", async () => {
      await setup({ state: "fine" }, PAUSED);
      renderBanner();

      const alert = await screen.findByRole("alert");

      expect(alert).toHaveTextContent(`Jira rejected the token. Showing cached data from ${clockTime(LAST_FULL)}.`);
    });

    it("omits the cache time when nothing was ever fully synced", async () => {
      await setup({ state: "fine" }, PAUSED, null);
      renderBanner();

      expect(await screen.findByRole("alert")).toHaveTextContent(/^Jira rejected the token\.\s*Update token$/);
    });

    it("opens the connection page from Update token", async () => {
      await setup({ state: "fine" }, PAUSED);
      renderBanner();

      await userEvent.click(await screen.findByRole("button", { name: "Update token" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:connection");
    });

    it("wins over the expiry warning", async () => {
      await setup({ state: "soon", days: 3 }, PAUSED);
      renderBanner();

      await screen.findByRole("alert");

      expect(screen.queryByText(/expires/)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Replace" })).not.toBeInTheDocument();
    });
  });

  describe("when the token expires soon", () => {
    it.each<[number, string]>([
      [12, "Your Jira token expires in 12 days."],
      [1, "Your Jira token expires in 1 day."],
      [0, "Your Jira token expires today."],
    ])("warns with %i days left as %s", async (days, text) => {
      await setup({ state: "soon", days });
      renderBanner();

      expect(await screen.findByRole("status")).toHaveTextContent(text);
    });

    it("opens the connection page from Replace", async () => {
      await setup({ state: "soon", days: 12 });
      renderBanner();

      await userEvent.click(await screen.findByRole("button", { name: "Replace" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:connection");
    });

    it("hides when dismissed and stays hidden on a re-render the same day", async () => {
      await setup({ state: "soon", days: 12 });
      const first = renderBanner();
      await userEvent.click(await screen.findByRole("button", { name: "Dismiss for today" }));
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
      first.unmount();

      renderBanner();
      await settled();

      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    it("shows again on another day", async () => {
      await setup({ state: "soon", days: 12 });
      const first = renderBanner();
      await userEvent.click(await screen.findByRole("button", { name: "Dismiss for today" }));
      first.unmount();

      vi.useFakeTimers({ shouldAdvanceTime: true, toFake: ["Date"], now: new Date(Date.now() + 2 * 24 * 60 * 60 * 1000) });
      renderBanner();

      expect(await screen.findByRole("status")).toHaveTextContent("Your Jira token expires in 12 days.");
    });
  });

  describe("when the token has expired", () => {
    it("shows a red alert that sync will stop", async () => {
      await setup({ state: "expired" });
      renderBanner();

      expect(await screen.findByRole("alert")).toHaveTextContent("Your Jira token has expired; sync will stop until you replace it.");
    });

    it("opens the connection page from Update token", async () => {
      await setup({ state: "expired" });
      renderBanner();

      await userEvent.click(await screen.findByRole("button", { name: "Update token" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:connection");
    });

    it("cannot be dismissed, even after an earlier dismissal today", async () => {
      localStorage.setItem("taskboard.tokenBannerDismissed", new Date().toDateString());
      await setup({ state: "expired" });
      renderBanner();

      await screen.findByRole("alert");

      expect(screen.queryByRole("button", { name: "Dismiss for today" })).not.toBeInTheDocument();
    });
  });

  describe("when the token is fine", () => {
    it("shows nothing", async () => {
      await setup({ state: "fine" });
      renderBanner();
      await settled();

      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });
  });
});
