import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { BERLIN_DAYS, localDays } from "../test/days";
import { useLocalDays } from "./localDays";
import { createQueryClient } from "./queryClient";

const mocked = vi.mocked(api, true);

function mount() {
  const wrapper = ({ children }: { children: ReactNode }) => createElement(QueryClientProvider, { client: createQueryClient() }, children);
  return renderHook(() => useLocalDays(), { wrapper });
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("useLocalDays", () => {
  it("is undefined until Rust has answered, then the day boundaries", async () => {
    mocked.localDays.mockResolvedValue(BERLIN_DAYS);

    const { result } = mount();

    expect(result.current).toBeUndefined();
    await waitFor(() => expect(result.current).toEqual(BERLIN_DAYS));
  });

  it("asks again once tomorrow has begun and then gives the new boundaries", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const now = Date.now();
      const first = localDays({ yesterday: new Date(now - 86_400_000).toISOString(), today: new Date(now - 1000).toISOString(), tomorrow: new Date(now + 5000).toISOString() });
      const next = localDays({ today: first.tomorrow, tomorrow: new Date(now + 86_400_000).toISOString() });
      mocked.localDays.mockResolvedValueOnce(first).mockResolvedValue(next);

      const { result } = mount();
      await waitFor(() => expect(result.current).toEqual(first));

      await vi.advanceTimersByTimeAsync(6000);

      await waitFor(() => expect(result.current).toEqual(next));
    } finally {
      vi.useRealTimers();
    }
  });
});
