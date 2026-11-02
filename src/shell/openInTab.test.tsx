import { QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { tab, tabbedNav } from "../test/nav";
import { useOpenIssueInTab } from "./openInTab";

const mocked = vi.mocked(api, true);

const ORIGIN = { listId: 11, viewId: 101 };
const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={createQueryClient()}>{children}</QueryClientProvider>;

async function openIssue(show: boolean) {
  const { result } = renderHook(() => useOpenIssueInTab(), { wrapper });
  await act(async () => result.current("10042", ORIGIN, show));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.navTabNew.mockResolvedValue(tabbedNav([tab(1, ORIGIN), tab(5, ORIGIN), tab(2, ORIGIN)], 1));
});

describe("useOpenIssueInTab", () => {
  it("opens a tab at the origin behind the shown one and parks the issue with the tab right after the shown one", async () => {
    await openIssue(false);

    expect(mocked.navTabNew).toHaveBeenCalledWith(11, 101, false);
    expect(useStore.getState().parkedIssueViews).toEqual({ 5: { origin: ORIGIN, stack: ["10042"] } });
    expect(mocked.navTabActivate).not.toHaveBeenCalled();
  });

  it("also shows the new tab when asked to", async () => {
    await openIssue(true);

    expect(mocked.navTabActivate).toHaveBeenCalledWith(5);
  });

  it("parks nothing and shows an error toast when the tab cannot be opened", async () => {
    mocked.navTabNew.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

    await openIssue(true);

    expect(useStore.getState().parkedIssueViews).toEqual({});
    expect(mocked.navTabActivate).not.toHaveBeenCalled();
    expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not open a tab: Disk full");
  });
});
