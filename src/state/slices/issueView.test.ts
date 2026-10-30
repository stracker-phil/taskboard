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

  it("closes the whole stack at once", () => {
    useStore.getState().openIssue("10042", ORIGIN);
    useStore.getState().openRelated("10051");

    useStore.getState().closeIssue();

    expect(useStore.getState().issueView).toBeNull();
  });
});
