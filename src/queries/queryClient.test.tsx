import { QueryClientProvider, useQuery } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { resetStore, useStore } from "../state/store";
import { createQueryClient } from "./queryClient";
import { useViewBoard, useViewRows } from "./views";

const mocked = vi.mocked(api, true);
const DISK_FULL = { kind: "db", message: "Disk full", code: null };

function wrapper({ children }: { children: ReactNode }) {
  const client = createQueryClient();
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

const toasts = () => useStore.getState().toasts.map((t) => t.message);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("createQueryClient", () => {
  it("shows an error toast with the reason when a view's rows fail to load", async () => {
    mocked.viewRows.mockRejectedValue(DISK_FULL);

    renderHook(() => useViewRows(1, 2), { wrapper });

    await waitFor(() => expect(toasts()).toContain("Could not load this view: Disk full"));
  });

  it("shows an error toast with the reason when a Board view fails to load", async () => {
    mocked.viewBoard.mockRejectedValue(DISK_FULL);

    renderHook(() => useViewBoard(1, 2), { wrapper });

    await waitFor(() => expect(toasts()).toContain("Could not load this view: Disk full"));
  });

  it("shows no toast when a query without a failure message fails", async () => {
    mocked.filterOptions.mockRejectedValue(DISK_FULL);

    const { result } = renderHook(() => useQuery({ queryKey: ["plain"], queryFn: () => api.filterOptions() }), { wrapper });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(toasts()).toEqual([]);
  });
});
