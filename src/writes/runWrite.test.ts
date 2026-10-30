import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type WriteOutcome } from "../api";
import { resetStore, useStore } from "../state/store";
import { explainWritesOff, runWrite } from "./runWrite";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts;
const messages = () => toasts().map((t) => [t.kind, t.message]);

const ASSIGNED: WriteOutcome = {
  done: "Assigned NW-1042 to you",
  reverse: { label: "Change back to Marta Klein", change: { kind: "assign", accountId: "demo-account-3" } },
};

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("runWrite", () => {
  it("confirms the change with a success toast offering the way back", async () => {
    runWrite("10042", Promise.resolve(ASSIGNED), "Could not change the assignee");

    await vi.waitFor(() => expect(messages()).toEqual([["success", "Assigned NW-1042 to you"]]));
    expect(toasts()[0].action?.label).toBe("Change back to Marta Klein");
  });

  it("offers no way back when the outcome has none", async () => {
    runWrite("10042", Promise.resolve({ done: "Unassigned NW-1042", reverse: null }), "Could not change the assignee");

    await vi.waitFor(() => expect(messages()).toEqual([["success", "Unassigned NW-1042"]]));
    expect(toasts()[0].action).toBeUndefined();
  });

  it("sends the reverse change for the same issue when the toast action runs, and confirms that outcome too", async () => {
    mocked.issueWrite.mockResolvedValue({ done: "Assigned NW-1042 to Marta Klein", reverse: null });
    runWrite("10042", Promise.resolve(ASSIGNED), "Could not change the assignee");
    await vi.waitFor(() => expect(toasts()).toHaveLength(1));

    toasts()[0].action?.run();

    expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "assign", accountId: "demo-account-3" });
    await vi.waitFor(() => expect(messages()).toContainEqual(["success", "Assigned NW-1042 to Marta Klein"]));
  });

  it("shows the reason Jira refused the change as an error toast", async () => {
    runWrite("10042", Promise.reject(Object.assign(new Error("Jira refused the change: no permission"), { kind: "auth", code: null })), "Could not change the assignee");

    await vi.waitFor(() => expect(messages()).toEqual([["error", "Could not change the assignee: Jira refused the change: no permission"]]));
  });

  it("shows an error toast when the change back is refused", async () => {
    mocked.issueWrite.mockRejectedValue({ kind: "io", message: "Network is down", code: null });
    runWrite("10042", Promise.resolve(ASSIGNED), "Could not change the assignee");
    await vi.waitFor(() => expect(toasts()).toHaveLength(1));

    toasts()[0].action?.run();

    await vi.waitFor(() => expect(messages()).toContainEqual(["error", "Could not change it back: Network is down"]));
  });
});

describe("explainWritesOff", () => {
  it("tells the owner where to turn changes to Jira on", () => {
    explainWritesOff();

    expect(messages()).toEqual([["info", "Changes to Jira are off. Turn them on in Settings → Connection."]]);
  });
});
