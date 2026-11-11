import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { emitEvent } from "../../test/apiMock";
import { activityDetail, activityText, JiraActivityStatus } from "./JiraActivityStatus";

const mocked = vi.mocked(api, true);

const advance = (ms: number) =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
const doing = (...entries: string[]) =>
  act(async () => {
    emitEvent(api, "jira_activity", { doing: entries });
  });

async function renderStatus(initial: string[] = []) {
  mocked.jiraActivity.mockResolvedValue({ doing: initial });
  render(<JiraActivityStatus />);
  await advance(0);
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers({ shouldAdvanceTime: true });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("activityText", () => {
  it.each<[string, string[], string]>([
    ["a single request", ["Saving the status"], "Saving the status…"],
    ["two requests, naming the newest", ["Fetching issues", "Saving the status"], "Saving the status… +1"],
    ["four requests, counting the others", ["Fetching issues", "Loading a project", "Fetching issues", "Saving the status"], "Saving the status… +3"],
  ])("describes %s", (_name, entries, text) => {
    expect(activityText(entries)).toBe(text);
  });
});

describe("activityDetail", () => {
  it("lists one line per request", () => {
    expect(activityDetail(["Saving the status", "Loading a project"])).toBe("Saving the status\nLoading a project");
  });

  it("counts alike requests on one line, in order of first appearance", () => {
    expect(activityDetail(["Fetching issues", "Saving the status", "Fetching issues", "Fetching issues"])).toBe("Fetching issues ×3\nSaving the status");
  });
});

describe("JiraActivityStatus", () => {
  it("shows nothing while Jira is not being asked", async () => {
    await renderStatus();

    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows a request only once it has run for 300 ms", async () => {
    await renderStatus();
    await doing("Saving the status");

    await advance(200);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();

    await advance(150);
    expect(screen.getByRole("status")).toHaveTextContent("Saving the status…");
  });

  it("never shows a request that ends before 300 ms", async () => {
    await renderStatus();
    await doing("Saving the status");
    await advance(100);
    await doing();

    await advance(1000);

    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows the activity that was already running when it appeared", async () => {
    await renderStatus(["Fetching issues"]);

    await advance(350);

    expect(screen.getByRole("status")).toHaveTextContent("Fetching issues…");
  });

  it("lists every request in the title, counting alike ones", async () => {
    await renderStatus();
    await doing("Fetching issues", "Fetching issues", "Saving the status");

    await advance(350);

    expect(screen.getByRole("status")).toHaveAttribute("title", "Fetching issues ×2\nSaving the status");
  });

  it("follows new events immediately while shown", async () => {
    await renderStatus(["Fetching issues"]);
    await advance(350);

    await doing("Fetching issues", "Saving the status");

    expect(screen.getByRole("status")).toHaveTextContent("Saving the status… +1");
  });

  it("stays through a gap shorter than 250 ms between two requests", async () => {
    await renderStatus(["Fetching issues"]);
    await advance(350);

    await doing();
    await advance(100);
    expect(screen.getByRole("status")).toBeInTheDocument();

    await doing("Loading a project");
    await advance(1000);

    expect(screen.getByRole("status")).toHaveTextContent("Loading a project…");
  });

  it("hides 250 ms after the last request ends", async () => {
    await renderStatus(["Fetching issues"]);
    await advance(350);

    await doing();
    await advance(200);
    expect(screen.getByRole("status")).toBeInTheDocument();

    await advance(100);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
