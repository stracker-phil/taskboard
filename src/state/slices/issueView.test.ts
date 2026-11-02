import { beforeEach, describe, expect, it } from "vitest";
import { narrowingKey } from "./narrowing";
import { resetStore, useStore } from "../store";

const ORIGIN = { listId: 3, viewId: 7 };
const stack = () => useStore.getState().issueView?.stack;

beforeEach(() => {
  resetStore();
});

describe("issue view slice", () => {
  it("starts closed", () => {
    expect(useStore.getState().issueView).toBeNull();
  });

  it("opens an issue with its origin as a one-issue stack", () => {
    useStore.getState().openIssue("10042", ORIGIN);

    expect(useStore.getState().issueView).toEqual({ origin: ORIGIN, stack: ["10042"] });
  });

  describe("openRelated", () => {
    it("pushes the related issue on top", () => {
      useStore.getState().openIssue("10042", ORIGIN);

      useStore.getState().openRelated("10051");

      expect(stack()).toEqual(["10042", "10051"]);
    });

    it("does not push the issue that is already on top", () => {
      useStore.getState().openIssue("10042", ORIGIN);

      useStore.getState().openRelated("10042");

      expect(stack()).toEqual(["10042"]);
    });

    it("does nothing while the Issue view is closed", () => {
      useStore.getState().openRelated("10051");

      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("showIssue", () => {
    it("swaps the issue on top and keeps the ones below", () => {
      useStore.getState().openIssue("10042", ORIGIN);
      useStore.getState().openRelated("10051");

      useStore.getState().showIssue("10060");

      expect(stack()).toEqual(["10042", "10060"]);
    });

    it("selects the issue in the origin view's place", () => {
      useStore.getState().openIssue("10042", ORIGIN);

      useStore.getState().showIssue("10060");

      expect(useStore.getState().places[narrowingKey(ORIGIN.listId, ORIGIN.viewId)].selectedId).toBe("10060");
    });

    it("does nothing while the Issue view is closed", () => {
      useStore.getState().showIssue("10060");

      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("issueBack", () => {
    it("pops to the issue before", () => {
      useStore.getState().openIssue("10042", ORIGIN);
      useStore.getState().openRelated("10051");

      useStore.getState().issueBack();

      expect(useStore.getState().issueView).toEqual({ origin: ORIGIN, stack: ["10042"] });
    });

    it("closes the Issue view at the bottom of the stack", () => {
      useStore.getState().openIssue("10042", ORIGIN);

      useStore.getState().issueBack();

      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("switchTab", () => {
    const OTHER_ORIGIN = { listId: 4, viewId: 9 };
    const FIRST = { origin: ORIGIN, stack: ["10042", "10051"] };
    const SECOND = { origin: OTHER_ORIGIN, stack: ["10060"] };
    const parked = () => useStore.getState().parkedIssueViews;
    const shown = () => useStore.getState().issueView;

    it("parks the shown Issue view with its tab and shows none on a tab without one", () => {
      resetStore({ issueView: FIRST });

      useStore.getState().switchTab(1, 2, [1, 2]);

      expect(shown()).toBeNull();
      expect(parked()).toEqual({ 1: FIRST });
    });

    it("brings back the Issue view parked with the tab it switches to", () => {
      resetStore({ issueView: FIRST, parkedIssueViews: { 2: SECOND } });

      useStore.getState().switchTab(1, 2, [1, 2]);

      expect(shown()).toEqual(SECOND);
      expect(parked()).toEqual({ 1: FIRST });
    });

    it("gives each tab its own Issue view back when switching there and back again", () => {
      resetStore({ issueView: FIRST });

      useStore.getState().switchTab(1, 2, [1, 2]);
      useStore.getState().openIssue("10060", OTHER_ORIGIN);
      useStore.getState().switchTab(2, 1, [1, 2]);

      expect(shown()).toEqual(FIRST);
      expect(parked()).toEqual({ 2: SECOND });
    });

    it("leaves the Issue views of tabs it does not touch parked", () => {
      resetStore({ issueView: FIRST, parkedIssueViews: { 3: SECOND } });

      useStore.getState().switchTab(1, 2, [1, 2, 3]);

      expect(parked()).toEqual({ 1: FIRST, 3: SECOND });
    });

    it("drops the Issue views parked with tabs that are no longer open", () => {
      resetStore({ issueView: null, parkedIssueViews: { 2: SECOND, 3: FIRST } });

      useStore.getState().switchTab(1, 2, [1, 2]);

      expect(shown()).toEqual(SECOND);
      expect(parked()).toEqual({});
    });

    it("drops the shown Issue view when its tab was closed", () => {
      resetStore({ issueView: FIRST, parkedIssueViews: { 2: SECOND } });

      useStore.getState().switchTab(1, 2, [2]);

      expect(shown()).toEqual(SECOND);
      expect(parked()).toEqual({});
    });
  });

  it("closes the whole stack at once", () => {
    useStore.getState().openIssue("10042", ORIGIN);
    useStore.getState().openRelated("10051");

    useStore.getState().closeIssue();

    expect(useStore.getState().issueView).toBeNull();
  });
});
