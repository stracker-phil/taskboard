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
