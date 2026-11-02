import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import type { Condition } from "../../api";
import { resetStore, useStore } from "../store";
import { NO_VIEW_NARROWING } from "./narrowing";

const KEY = "11:101";
const OTHER = "11:102";
const HIGH: Condition = { field: "priority", op: "is", value: "High" };
const narrowings = () => useStore.getState().narrowings;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("setNarrowing", () => {
  it("merges the patch into the view's narrowing at once", () => {
    useStore.getState().setNarrowing(KEY, { text: "login" });
    useStore.getState().setNarrowing(KEY, { me: true });

    expect(narrowings()[KEY]).toEqual({ ...NO_VIEW_NARROWING, me: true, text: "login" });
  });

  it("starts a view from the saved state: no draft of any kind", () => {
    useStore.getState().setNarrowing(KEY, { text: "login" });

    expect(narrowings()[KEY]).toMatchObject({ me: null, list: null, view: null, epic: null });
    expect(narrowings()[KEY]?.person).toBeUndefined();
  });

  it("tells a drafted Anyone (null) from the view's saved person (undefined)", () => {
    useStore.getState().setNarrowing(KEY, { person: null });

    expect(narrowings()[KEY]?.person).toBeNull();
  });

  it("keeps the narrowings of different views apart", () => {
    useStore.getState().setNarrowing(KEY, { me: true });

    expect(narrowings()[OTHER]).toBeUndefined();
  });
});

describe("clearDrafts", () => {
  it("drops the Me chip and both condition drafts of the view", () => {
    useStore.getState().setNarrowing(KEY, { me: true, list: [HIGH], view: [] });

    useStore.getState().clearDrafts(KEY);

    expect(narrowings()[KEY]).toMatchObject({ me: null, list: null, view: null });
  });

  it("drops a drafted person back to the view's saved one, even a drafted Anyone", () => {
    useStore.getState().setNarrowing(KEY, { person: null });
    useStore.getState().setNarrowing(OTHER, { person: "demo-account-2" });

    useStore.getState().clearDrafts(KEY);
    useStore.getState().clearDrafts(OTHER);

    expect(narrowings()[KEY]?.person).toBeUndefined();
    expect(narrowings()[OTHER]?.person).toBeUndefined();
  });

  it("keeps the text filter and the epic", () => {
    useStore.getState().setNarrowing(KEY, { me: true, text: "login", epic: "10001" });

    useStore.getState().clearDrafts(KEY);

    expect(narrowings()[KEY]).toEqual({ ...NO_VIEW_NARROWING, text: "login", epic: "10001" });
  });

  it("leaves other views' drafts alone", () => {
    useStore.getState().setNarrowing(KEY, { me: true });
    useStore.getState().setNarrowing(OTHER, { me: true });

    useStore.getState().clearDrafts(KEY);

    expect(narrowings()[OTHER]?.me).toBe(true);
  });

  it("does nothing for a view without a narrowing", () => {
    useStore.getState().clearDrafts(KEY);

    expect(narrowings()).toEqual({});
  });
});

describe("keepNarrowings", () => {
  it("keeps the narrowings of every view it is given and drops the rest", () => {
    useStore.getState().setNarrowing(KEY, { me: true, text: "login" });
    useStore.getState().setNarrowing(OTHER, { view: [HIGH] });
    useStore.getState().setNarrowing("12:201", { text: "pricing" });

    useStore.getState().keepNarrowings([KEY, OTHER]);

    expect(Object.keys(narrowings()).sort()).toEqual([KEY, OTHER]);
    expect(narrowings()[KEY]).toEqual({ ...NO_VIEW_NARROWING, me: true, text: "login" });
    expect(narrowings()[OTHER]).toEqual({ ...NO_VIEW_NARROWING, view: [HIGH] });
  });

  it("drops everything for no views", () => {
    useStore.getState().setNarrowing(KEY, { me: true });

    useStore.getState().keepNarrowings([]);

    expect(narrowings()).toEqual({});
  });

  it("ignores views that have no narrowing", () => {
    useStore.getState().setNarrowing(KEY, { me: true });

    useStore.getState().keepNarrowings([KEY, "99:1"]);

    expect(narrowings()).toEqual({ [KEY]: { ...NO_VIEW_NARROWING, me: true } });
  });
});

describe("keepOnlyNarrowing", () => {
  it("drops the text, epic and drafts of every other view", () => {
    useStore.getState().setNarrowing(OTHER, { me: true, text: "login", epic: "10001", list: [HIGH] });
    useStore.getState().setNarrowing("12:201", { view: [HIGH] });

    useStore.getState().keepOnlyNarrowing(KEY);

    expect(narrowings()).toEqual({});
  });

  it("keeps the narrowing of the view it is given", () => {
    useStore.getState().setNarrowing(KEY, { me: true, text: "login", list: [HIGH] });
    useStore.getState().setNarrowing(OTHER, { me: true });

    useStore.getState().keepOnlyNarrowing(KEY);

    expect(narrowings()).toEqual({ [KEY]: { ...NO_VIEW_NARROWING, me: true, text: "login", list: [HIGH] } });
  });
});
