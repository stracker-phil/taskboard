import { render } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { pinItem, pins } from "../test/pins";
import { createQueryClient } from "./queryClient";
import { usePinExpiry } from "./pins";

const mocked = vi.mocked(api, true);

function Mounted() {
  usePinExpiry();
  return null;
}

const inMs = (ms: number) => new Date(Date.now() + ms).toISOString();

async function mount() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Mounted />
    </QueryClientProvider>,
  );
  await vi.waitFor(() => expect(mocked.pinsList).toHaveBeenCalledTimes(1));
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.pinsList.mockResolvedValue(pins());
  vi.useFakeTimers({ shouldAdvanceTime: true });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("usePinExpiry", () => {
  it("refetches the pins once the soonest pin has run out", async () => {
    mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 1, expiresAt: inMs(3_600_000) }), pinItem({ id: 2, expiresAt: inMs(60_000) })]));
    await mount();

    await vi.advanceTimersByTimeAsync(30_000);
    expect(mocked.pinsList).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(31_000);
    await vi.waitFor(() => expect(mocked.pinsList).toHaveBeenCalledTimes(2));
  });

  it("does not refetch for pins that stay until unpinned", async () => {
    mocked.pinsList.mockResolvedValue(pins([pinItem({ expiresAt: null })]));
    await mount();

    await vi.advanceTimersByTimeAsync(7 * 24 * 3_600_000);

    expect(mocked.pinsList).toHaveBeenCalledTimes(1);
  });
});
