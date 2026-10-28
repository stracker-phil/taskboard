import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { resetStore, useStore } from "../store";
import { EMPTY_PLACE } from "./places";

const mocked = vi.mocked(api, true);

const KEY = "11:101";

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers({ shouldAdvanceTime: true });
  resetStore();
  mocked.placeSet.mockResolvedValue(undefined);
});

afterEach(async () => {
  await vi.runOnlyPendingTimersAsync();
  vi.useRealTimers();
});

describe("setNarrowing", () => {
  it("merges the patch into the view's narrowing at once", () => {
    useStore.getState().setNarrowing(KEY, { text: "login" });
    useStore.getState().setNarrowing(KEY, { me: true });

    expect(useStore.getState().narrowings[KEY]).toEqual({ me: true, text: "login", epic: null });
  });

  it("keeps the narrowings of different views apart", () => {
    useStore.getState().setNarrowing("11:101", { me: true });

    expect(useStore.getState().narrowings["11:102"]).toBeUndefined();
  });

  it("saves the Me chip with the view's place after the debounce", async () => {
    useStore.getState().setNarrowing(KEY, { me: true });

    await vi.advanceTimersByTimeAsync(299);
    expect(mocked.placeSet).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);

    expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, { ...EMPTY_PLACE, onlyMine: true, text: "" });
  });

  it("saves the text together with the current Me chip", async () => {
    useStore.getState().setNarrowing(KEY, { me: true });
    useStore.getState().setNarrowing(KEY, { text: "login" });

    await vi.advanceTimersByTimeAsync(300);

    expect(mocked.placeSet).toHaveBeenCalledTimes(1);
    expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, { ...EMPTY_PLACE, onlyMine: true, text: "login" });
  });

  it("saves nothing for an epic-only change", async () => {
    useStore.getState().setNarrowing(KEY, { epic: "10001" });

    await vi.advanceTimersByTimeAsync(1000);

    expect(mocked.placeSet).not.toHaveBeenCalled();
    expect(useStore.getState().narrowings[KEY]?.epic).toBe("10001");
  });

  it("keeps the rest of the saved place when the Me chip changes", async () => {
    mocked.placeGet.mockResolvedValue({ ...EMPTY_PLACE, scrollTop: 320, selectedId: "10042" });
    await useStore.getState().loadPlace(11, 101);

    useStore.getState().setNarrowing(KEY, { me: true });
    await vi.advanceTimersByTimeAsync(300);

    expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, { ...EMPTY_PLACE, scrollTop: 320, selectedId: "10042", onlyMine: true });
  });
});
