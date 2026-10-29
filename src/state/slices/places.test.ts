import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type ViewPlace } from "../../api";
import { resetStore, useStore } from "../store";
import { EMPTY_PLACE } from "./places";

const mocked = vi.mocked(api, true);

const SAVED: ViewPlace = { scrollTop: 320, toggled: { "status:done": true }, collapsedParents: [], selectedId: "10042", peekOpen: true, onlyMine: false, text: "" };
const KEY = "11:101";
const place = () => useStore.getState().places[KEY];
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers({ shouldAdvanceTime: true });
  resetStore();
});

afterEach(async () => {
  await vi.runOnlyPendingTimersAsync();
  vi.useRealTimers();
});

describe("loadPlace", () => {
  it("stores the place Rust saved for the view", async () => {
    mocked.placeGet.mockResolvedValue(SAVED);

    await useStore.getState().loadPlace(11, 101);

    expect(mocked.placeGet).toHaveBeenCalledWith(11, 101);
    expect(place()).toEqual(SAVED);
  });

  it("starts from the empty place when Rust has none", async () => {
    mocked.placeGet.mockResolvedValue(null);

    await useStore.getState().loadPlace(11, 101);

    expect(place()).toEqual(EMPTY_PLACE);
  });

  it("keeps the places of different views apart", async () => {
    mocked.placeGet.mockImplementation(async (_list, viewId) => (viewId === 101 ? SAVED : null));

    await useStore.getState().loadPlace(11, 101);
    await useStore.getState().loadPlace(11, 102);

    expect(useStore.getState().places).toEqual({ "11:101": SAVED, "11:102": EMPTY_PLACE });
  });

  it("asks Rust once however often the view is loaded", async () => {
    mocked.placeGet.mockResolvedValue(SAVED);

    await useStore.getState().loadPlace(11, 101);
    await useStore.getState().loadPlace(11, 101);

    expect(mocked.placeGet).toHaveBeenCalledTimes(1);
  });

  it("keeps a change made while the place was loading", async () => {
    let finish: (saved: ViewPlace) => void = () => {};
    mocked.placeGet.mockReturnValue(new Promise<ViewPlace>((resolve) => (finish = resolve)));

    const loading = useStore.getState().loadPlace(11, 101);
    useStore.getState().updatePlace(11, 101, { scrollTop: 50 });
    finish(SAVED);
    await loading;

    expect(place()).toEqual({ ...EMPTY_PLACE, scrollTop: 50 });
  });

  it("seeds the view's narrowing from the saved Me chip and text", async () => {
    mocked.placeGet.mockResolvedValue({ ...SAVED, onlyMine: true, text: "login" });

    await useStore.getState().loadPlace(11, 101);

    expect(useStore.getState().narrowings[KEY]).toEqual({ me: true, text: "login", epic: null });
  });

  it("seeds an unnarrowed view when Rust has no place", async () => {
    mocked.placeGet.mockResolvedValue(null);

    await useStore.getState().loadPlace(11, 101);

    expect(useStore.getState().narrowings[KEY]).toEqual({ me: false, text: "", epic: null });
  });

  it("keeps a narrowing that was set before the place loaded", async () => {
    mocked.placeGet.mockResolvedValue({ ...SAVED, onlyMine: true, text: "login" });
    useStore.setState({ narrowings: { [KEY]: { me: false, text: "", epic: "10001" } } });

    await useStore.getState().loadPlace(11, 101);

    expect(useStore.getState().narrowings[KEY]).toEqual({ me: false, text: "", epic: "10001" });
  });

  it("shows a toast and starts from the empty place when the load fails", async () => {
    mocked.placeGet.mockRejectedValue({ kind: "io", message: "places.json is unreadable", code: null });

    await useStore.getState().loadPlace(11, 101);

    expect(toasts()).toEqual([["error", "Could not restore where you were in this view: places.json is unreadable"]]);
    expect(place()).toEqual(EMPTY_PLACE);
  });
});

describe("updatePlace", () => {
  it("merges the patch into the place at once", async () => {
    mocked.placeGet.mockResolvedValue(SAVED);
    await useStore.getState().loadPlace(11, 101);

    useStore.getState().updatePlace(11, 101, { selectedId: "10043" });

    expect(place()).toEqual({ ...SAVED, selectedId: "10043" });
  });

  it("starts from the empty place for a view that was never loaded", () => {
    useStore.getState().updatePlace(11, 101, { peekOpen: true });

    expect(place()).toEqual({ ...EMPTY_PLACE, peekOpen: true });
  });

  it("saves nothing until 300ms have passed without a change", async () => {
    useStore.getState().updatePlace(11, 101, { scrollTop: 10 });

    await vi.advanceTimersByTimeAsync(299);

    expect(mocked.placeSet).not.toHaveBeenCalled();
  });

  it("saves the merged place to Rust once after a burst of changes", async () => {
    const update = useStore.getState().updatePlace;
    update(11, 101, { scrollTop: 10 });
    await vi.advanceTimersByTimeAsync(200);
    update(11, 101, { scrollTop: 20 });
    await vi.advanceTimersByTimeAsync(200);
    update(11, 101, { selectedId: "10042" });
    await vi.advanceTimersByTimeAsync(300);

    expect(mocked.placeSet).toHaveBeenCalledTimes(1);
    expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, { ...EMPTY_PLACE, scrollTop: 20, selectedId: "10042" });
  });

  it("saves each view's place on its own", async () => {
    useStore.getState().updatePlace(11, 101, { scrollTop: 10 });
    useStore.getState().updatePlace(11, 102, { scrollTop: 20 });

    await vi.advanceTimersByTimeAsync(300);

    expect(mocked.placeSet).toHaveBeenCalledWith(11, 101, { ...EMPTY_PLACE, scrollTop: 10 });
    expect(mocked.placeSet).toHaveBeenCalledWith(11, 102, { ...EMPTY_PLACE, scrollTop: 20 });
  });

  it("shows a toast when the save fails", async () => {
    mocked.placeSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

    useStore.getState().updatePlace(11, 101, { scrollTop: 10 });
    await vi.advanceTimersByTimeAsync(300);

    expect(toasts()).toEqual([["error", "Could not remember where you were: Disk full"]]);
  });
});
