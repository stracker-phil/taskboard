import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { formatLocalDate } from "../lib/time";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { issueDetail } from "../test/issues";
import { pinItem, pins } from "../test/pins";
import { GoneNotice } from "./GoneNotice";

const mocked = vi.mocked(api, true);

const GONE = issueDetail({ gone: true, syncedAt: "2026-10-27T09:00:00.000Z" });

function renderNotice(onRemoved = vi.fn()) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <GoneNotice issue={GONE} onRemoved={onRemoved} />
    </QueryClientProvider>,
  );
  return onRemoved;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.pinsList.mockResolvedValue(pins());
  mocked.issueForget.mockResolvedValue(undefined);
  mocked.pinRemove.mockResolvedValue(undefined);
});

describe("GoneNotice", () => {
  it("says the issue is gone and when the cached copy was last synced", () => {
    renderNotice();

    expect(screen.getByText(`Deleted in Jira or you lost access. Last synced ${formatLocalDate(GONE.syncedAt)}. This is the cached copy.`)).toBeInTheDocument();
  });

  it("forgets the issue and reports it removed from Remove from cache", async () => {
    const onRemoved = renderNotice();

    await userEvent.click(screen.getByRole("button", { name: "Remove from cache" }));

    expect(mocked.issueForget).toHaveBeenCalledWith(GONE.id);
    await waitFor(() => expect(onRemoved).toHaveBeenCalled());
  });

  it("shows a toast and does not report it removed when forgetting fails", async () => {
    mocked.issueForget.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    const onRemoved = renderNotice();

    await userEvent.click(screen.getByRole("button", { name: "Remove from cache" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not remove it from the cache: Disk full"));
    expect(onRemoved).not.toHaveBeenCalled();
  });

  describe("Remove bookmark", () => {
    it("is not offered while the issue is not pinned", async () => {
      renderNotice();

      await waitFor(() => expect(mocked.pinsList).toHaveBeenCalled());
      expect(screen.queryByText("Remove bookmark")).not.toBeInTheDocument();
    });

    it("removes the issue's pin when it is pinned", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 7 })]));
      renderNotice();

      await userEvent.click(await screen.findByText("Remove bookmark"));

      expect(mocked.pinRemove).toHaveBeenCalledWith(7);
    });

    it("shows a toast when removing the bookmark fails", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 7 })]));
      mocked.pinRemove.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderNotice();

      await userEvent.click(await screen.findByText("Remove bookmark"));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not remove the bookmark: Disk full"));
    });
  });
});
