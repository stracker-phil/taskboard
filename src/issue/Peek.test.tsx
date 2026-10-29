import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type IssueDetail } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { connectionView } from "../test/jira";
import { ALEX, changeItem, commentItem } from "../test/issues";
import { commentTarget, pinItem, pins } from "../test/pins";
import { settings } from "../test/settings";
import { pullRequestLink } from "../test/webLinks";
import { Peek } from "./Peek";

const mocked = vi.mocked(api, true);

function detail(patch: Partial<IssueDetail> = {}): IssueDetail {
  return {
    id: "10042",
    key: "NW-1042",
    issueType: "Story",
    summary: "Redesign the pricing page",
    style: "progress",
    statusName: "In Progress",
    epic: { id: "9", name: "Launch Q4", color: 2 },
    releases: ["Release 2.4"],
    priority: { name: "High", level: 2 },
    assignees: [ALEX],
    reporter: null,
    sprint: null,
    labels: [],
    due: null,
    createdAt: "2026-10-01T09:00:00Z",
    updatedAt: "2026-10-02T09:00:00Z",
    syncedAt: "2026-10-27T09:00:00.000Z",
    descriptionHtml: "<p>Update the <strong>pricing</strong> table.</p>",
    commentCount: 0,
    isEpic: false,
    gone: false,
    custom: [],
    since: null,
    unread: false,
    ...patch,
  };
}

function renderPeek(onClose = vi.fn(), onExpand = vi.fn()) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Peek id="10042" onClose={onClose} onExpand={onExpand} />
    </QueryClientProvider>,
  );
  return onClose;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.issueForget.mockResolvedValue(undefined);
  mocked.pinsList.mockResolvedValue(pins());
  mocked.getSettings.mockResolvedValue(settings());
  mocked.issueGet.mockResolvedValue(detail());
  mocked.issueActivity.mockResolvedValue([]);
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));
});

describe("Peek", () => {
  it("shows the issue's key, type, title, status, epic, release, priority and assignee", async () => {
    renderPeek();

    expect(await screen.findByRole("heading", { name: "Redesign the pricing page" })).toBeInTheDocument();
    expect(screen.getByText("NW-1042")).toBeInTheDocument();
    expect(screen.getByText("Story")).toBeInTheDocument();
    expect(screen.getByText("In Progress")).toBeInTheDocument();
    expect(screen.getByText("Launch Q4")).toBeInTheDocument();
    expect(screen.getByText("Release 2.4")).toBeInTheDocument();
    expect(screen.getByText("High")).toBeInTheDocument();
    expect(screen.getByText("Alex Morgan")).toBeInTheDocument();
  });

  it("asks Rust for the issue by its id", async () => {
    renderPeek();

    await screen.findByRole("heading", { name: "Redesign the pricing page" });
    expect(mocked.issueGet).toHaveBeenCalledWith("10042");
  });

  it("shows the description without its tags", async () => {
    renderPeek();

    const emphasised = await screen.findByText("pricing");
    expect(emphasised.parentElement).toHaveTextContent("Update the pricing table.");
    expect(emphasised.parentElement?.textContent).not.toContain("<");
  });

  it("drops scripts from the description", async () => {
    mocked.issueGet.mockResolvedValue(detail({ descriptionHtml: "<p>Safe text</p><script>alert(1)</script>" }));
    renderPeek();

    expect(await screen.findByText("Safe text")).toBeInTheDocument();
    expect(screen.queryByText(/alert/)).not.toBeInTheDocument();
  });

  it.each([null, "", "<p> </p>"])("says there is no description when it is %j", async (descriptionHtml) => {
    mocked.issueGet.mockResolvedValue(detail({ descriptionHtml }));
    renderPeek();

    expect(await screen.findByText("No description.")).toBeInTheDocument();
  });

  it("shows a rich text custom field as a section under its name", async () => {
    mocked.issueGet.mockResolvedValue(
      detail({ custom: [{ id: "customfield_10080", name: "Steps to reproduce", value: null, html: "<ol><li>Open the form</li></ol><script>alert(1)</script>" }] }),
    );
    renderPeek();

    const section = await screen.findByRole("region", { name: "Steps to reproduce" });
    expect(within(section).getByRole("heading", { name: "Steps to reproduce" })).toBeInTheDocument();
    expect(within(section).getByText("Open the form")).toBeInTheDocument();
    expect(screen.queryByText(/alert/)).not.toBeInTheDocument();
  });

  it.each(["", "  ", "<p> </p>"])("shows no section for a rich text custom field whose html is %j", async (html) => {
    mocked.issueGet.mockResolvedValue(detail({ custom: [{ id: "customfield_10080", name: "Steps to reproduce", value: null, html }] }));
    renderPeek();

    await screen.findByRole("heading", { name: "Redesign the pricing page" });
    expect(screen.queryByRole("region", { name: "Steps to reproduce" })).not.toBeInTheDocument();
  });

  it("lists the issue's web links", async () => {
    mocked.issueWebLinks.mockResolvedValue([pullRequestLink()]);
    renderPeek();

    const section = within(await screen.findByRole("region", { name: "Web links" }));
    expect(section.getByRole("button", { name: /Validate DACH postcodes/ })).toHaveTextContent("shop#481");
  });

  it("leaves out the epic, release and priority an issue does not have", async () => {
    mocked.issueGet.mockResolvedValue(detail({ epic: null, releases: [], priority: null, assignees: [] }));
    renderPeek();

    await screen.findByRole("heading", { name: "Redesign the pricing page" });
    for (const text of ["Launch Q4", "Release 2.4", "High", "Alex Morgan"]) expect(screen.queryByText(text)).not.toBeInTheDocument();
  });

  it("calls onClose when the close button is clicked", async () => {
    const onClose = renderPeek();

    await userEvent.click(await screen.findByRole("button", { name: "Close (esc)" }));

    expect(onClose).toHaveBeenCalled();
  });

  it("opens the bookmark dialog from the Bookmark this issue button and bookmarks the issue", async () => {
    renderPeek();

    await userEvent.click(await screen.findByRole("button", { name: "Bookmark this issue" }));
    await userEvent.type(screen.getByLabelText(/Reason/), "Needs a decision{Enter}");

    expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "untilUnpinned", "Needs a decision");
  });

  it("offers Remove bookmark instead when the issue is bookmarked", async () => {
    mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 4 })]));
    renderPeek();

    await userEvent.click(await screen.findByRole("button", { name: "Remove bookmark" }));

    expect(mocked.pinRemove).toHaveBeenCalledWith(4);
  });

  it("shows a pinned comment's reason and time left on its card", async () => {
    mocked.issueActivity.mockResolvedValue([commentItem({ id: "20001" })]);
    mocked.pinsList.mockResolvedValue(pins([pinItem({ target: commentTarget(), reason: "Rules to follow", timeLeft: "1d left" })]));
    renderPeek();

    expect(await screen.findByText("Rules to follow")).toBeInTheDocument();
    expect(screen.getByText("1d left")).toBeInTheDocument();
  });

  it("shows Bookmarked on a bookmarked comment without a reason", async () => {
    mocked.issueActivity.mockResolvedValue([commentItem({ id: "20001" })]);
    mocked.pinsList.mockResolvedValue(pins([pinItem({ target: commentTarget(), reason: null, timeLeft: "" })]));
    renderPeek();

    expect(await screen.findByText("Bookmarked")).toBeInTheDocument();
  });

  it("offers Bookmark comment on a comment and bookmarks it with the comment's id", async () => {
    mocked.issueActivity.mockResolvedValue([commentItem({ id: "20001" })]);
    renderPeek();

    await userEvent.click(await screen.findByRole("button", { name: "Bookmark comment" }));
    await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

    expect(mocked.pinCreate).toHaveBeenCalledWith("comment", "20001", "untilUnpinned", null);
  });

  it("calls onExpand when the Full view button is clicked", async () => {
    const onExpand = vi.fn();
    renderPeek(vi.fn(), onExpand);

    await userEvent.click(await screen.findByRole("button", { name: "Full view" }));

    expect(onExpand).toHaveBeenCalled();
  });

  describe("unread changes", () => {
    const SINCE = { label: "Since yesterday", text: "2 new comments, moved to Review." };

    it("shows the banner of what changed, without a Mark as read button", async () => {
      mocked.issueGet.mockResolvedValue(detail({ since: SINCE, unread: true }));
      renderPeek();

      expect(await screen.findByRole("status")).toHaveTextContent("Since yesterday: 2 new comments, moved to Review.");
      expect(screen.queryByRole("button", { name: "Mark as read" })).not.toBeInTheDocument();
    });

    it("shows no banner for an issue without unread changes", async () => {
      renderPeek();

      await screen.findByRole("heading", { name: "Redesign the pricing page" });
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });
  });

  describe("marking seen after a delay", () => {
    const SINCE = { label: "Since yesterday", text: "1 new comment." };

    beforeEach(() => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    async function showUnseen(seenAfterSeconds?: number) {
      if (seenAfterSeconds !== undefined) mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds, pinDefault: "untilUnpinned", openIssuesIn: "preview" } }));
      serveByKey();
      const view = renderSwitchable("10042");
      await screen.findByRole("status");
      await vi.advanceTimersByTimeAsync(0);
      return view;
    }

    function renderSwitchable(id: string) {
      const client = createQueryClient();
      const view = (shown: string) => (
        <QueryClientProvider client={client}>
          <Peek id={shown} onClose={vi.fn()} onExpand={vi.fn()} />
        </QueryClientProvider>
      );
      const result = render(view(id));
      return { ...result, show: (next: string) => result.rerender(view(next)) };
    }

    function serveByKey() {
      mocked.issueGet.mockImplementation(async (id) => detail({ id, key: id === "10042" ? "NW-1042" : "NW-1043", summary: `Issue ${id}`, since: id === "10042" ? SINCE : null, unread: id === "10042" }));
    }

    it("marks the issue seen once it has been open for the default two seconds", async () => {
      await showUnseen();

      await vi.advanceTimersByTimeAsync(1500);
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(700);

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("waits for the delay set in the settings", async () => {
      await showUnseen(5);

      await vi.advanceTimersByTimeAsync(4500);
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(700);

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("marks the issue seen at once when the delay is 0", async () => {
      await showUnseen(0);

      await vi.advanceTimersByTimeAsync(50);

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("does not mark an issue that is closed before the delay", async () => {
      const { unmount } = await showUnseen();

      await vi.advanceTimersByTimeAsync(1000);
      unmount();
      await vi.advanceTimersByTimeAsync(5000);

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });

    it("does not mark an issue when another one is shown before the delay", async () => {
      const { show } = await showUnseen();

      await vi.advanceTimersByTimeAsync(1000);
      show("10043");
      await screen.findByRole("heading", { name: "Issue 10043" });
      await vi.advanceTimersByTimeAsync(5000);

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });

    it("does not mark an issue without unseen changes", async () => {
      serveByKey();
      renderSwitchable("10043");
      await screen.findByRole("heading", { name: "Issue 10043" });

      await vi.advanceTimersByTimeAsync(5000);

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });
  });

  describe("latest activity", () => {
    const comment = (id: string, text: string) => commentItem({ id, bodyHtml: `<p>${text}</p>` });

    it("shows the first three items as received, newest first, under Latest activity", async () => {
      mocked.issueActivity.mockResolvedValue([comment("4", "Fourth note"), comment("3", "Third note"), comment("2", "Second note"), comment("1", "First note")]);
      renderPeek();

      const section = within(await screen.findByRole("region", { name: "Latest activity" }));
      expect(section.queryByText("First note")).not.toBeInTheDocument();
      const shown = section.getAllByText(/ note$/).map((e) => e.textContent);
      expect(shown).toEqual(["Fourth note", "Third note", "Second note"]);
    });

    it("shows recorded changes too", async () => {
      mocked.issueActivity.mockResolvedValue([changeItem()]);
      renderPeek();

      expect(await screen.findByText(/Changed status/)).toBeInTheDocument();
    });

    it("opens the full view from All activity", async () => {
      mocked.issueActivity.mockResolvedValue([comment("1", "First note")]);
      const onExpand = vi.fn();
      renderPeek(vi.fn(), onExpand);

      await userEvent.click(await screen.findByRole("button", { name: "All activity" }));

      expect(onExpand).toHaveBeenCalled();
    });

    it("is left out when the issue has no activity", async () => {
      renderPeek();

      await screen.findByRole("heading", { name: "Redesign the pricing page" });
      await vi.waitFor(() => expect(mocked.issueActivity).toHaveBeenCalled());
      expect(screen.queryByRole("region", { name: "Latest activity" })).not.toBeInTheDocument();
    });
  });

  describe("Open in Jira", () => {
    it("opens the issue's page on the Jira site", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "https://example.atlassian.net" }));
      renderPeek();

      await userEvent.click(await screen.findByRole("button", { name: "Open in Jira" }));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-1042");
    });

    it("is not offered without a Jira site", async () => {
      renderPeek();

      await screen.findByRole("heading", { name: "Redesign the pricing page" });
      await vi.waitFor(() => expect(mocked.connectionGet).toHaveBeenCalled());
      expect(screen.queryByRole("button", { name: "Open in Jira" })).not.toBeInTheDocument();
    });
  });

  describe("an issue Jira deleted", () => {
    it("shows the notice about the cached copy", async () => {
      mocked.issueGet.mockResolvedValue(detail({ gone: true }));
      renderPeek();

      expect(await screen.findByText(/^Deleted in Jira or you lost access\. Last synced .+\. This is the cached copy\.$/)).toBeInTheDocument();
      expect(screen.getByRole("heading", { name: "Redesign the pricing page" })).toBeInTheDocument();
    });

    it("forgets the issue and closes the peek from Remove from cache", async () => {
      mocked.issueGet.mockResolvedValue(detail({ gone: true }));
      const onClose = renderPeek();

      await userEvent.click(await screen.findByRole("button", { name: "Remove from cache" }));

      expect(mocked.issueForget).toHaveBeenCalledWith("10042");
      await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
    });

    it("keeps the peek open and shows a toast when forgetting fails", async () => {
      mocked.issueGet.mockResolvedValue(detail({ gone: true }));
      mocked.issueForget.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const onClose = renderPeek();

      await userEvent.click(await screen.findByRole("button", { name: "Remove from cache" }));

      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not remove it from the cache: Disk full"));
      expect(onClose).not.toHaveBeenCalled();
    });

    it("shows no notice for a live issue", async () => {
      renderPeek();

      await screen.findByRole("heading", { name: "Redesign the pricing page" });
      expect(screen.queryByText(/Deleted in Jira/)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Remove from cache" })).not.toBeInTheDocument();
    });
  });

  it("says the issue is no longer in the cache when Rust cannot find it", async () => {
    mocked.issueGet.mockRejectedValue({ kind: "notFound", message: "No such issue", code: null });
    renderPeek();

    expect(await screen.findByText("This issue is no longer in the cache.")).toBeInTheDocument();
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
  });
});
