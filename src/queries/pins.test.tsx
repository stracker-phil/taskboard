import { render } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { commentTarget, issueTarget, listTarget, pinItem, pins, viewTarget } from "../test/pins";
import { createQueryClient } from "./queryClient";
import { pinOf, usePinExpiry } from "./pins";

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

describe("pinOf", () => {
  const ISSUE = pinItem({ id: 1, target: issueTarget({ id: "10042" }) });
  const COMMENT = pinItem({ id: 2, target: commentTarget({ id: "20001" }) });
  const LIST = pinItem({ id: 3, target: listTarget({ id: 21 }) });
  const VIEW = pinItem({ id: 4, target: viewTarget({ id: 21 }) });
  const ALL = [ISSUE, COMMENT, LIST, VIEW];

  it.each([
    ["an issue", "issue", "10042", 1],
    ["a comment", "comment", "20001", 2],
    ["a filter list", "filterList", "21", 3],
    ["a view", "view", "21", 4],
  ] as const)("finds the pin of %s by kind and id", (_name, kind, id, pinId) => {
    expect(pinOf(ALL, kind, id)?.id).toBe(pinId);
  });

  it("keeps a list and a view with the same id apart", () => {
    expect(pinOf([VIEW], "filterList", "21")).toBeUndefined();
    expect(pinOf([LIST], "view", "21")).toBeUndefined();
  });

  it("finds nothing for an id nobody bookmarked or before the pins have loaded", () => {
    expect(pinOf(ALL, "view", "99")).toBeUndefined();
    expect(pinOf(undefined, "view", "21")).toBeUndefined();
  });
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
