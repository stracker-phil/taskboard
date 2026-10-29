import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type IssueDetail, type Row } from "../api";
import { installCommands } from "../commands";
import { getCommand, resolveKeydown } from "../commands/registry";
import { issueKey } from "../queries/issues";
import { createQueryClient } from "../queries/queryClient";
import type { IssueOrigin } from "../state/slices/issueView";
import { narrowingKey } from "../state/slices/narrowing";
import { resetStore, useStore } from "../state/store";
import { commentItem, groupOf, issueDetail, related, relatedIssue, row, SAM, viewData } from "../test/issues";
import { connectionView } from "../test/jira";
import { pinItem, pins } from "../test/pins";
import { settings } from "../test/settings";
import { sidebarTree, space } from "../test/spaces";
import { listNode, view } from "../test/views";
import { pullRequestLink } from "../test/webLinks";
import { IssueView } from "./IssueView";

const mocked = vi.mocked(api, true);

const ORIGIN: IssueOrigin = { listId: 11, viewId: 101 };
const EPIC = issueDetail({ id: "10001", key: "NW-1001", issueType: "Epic", summary: "Checkout", isEpic: true, epic: null, commentCount: 1 });

function serve(...issues: IssueDetail[]) {
  mocked.issueGet.mockImplementation(async (id) => {
    const found = issues.find((issue) => issue.id === id);
    if (!found) throw Object.assign(new Error("No such issue"), { kind: "notFound", code: null });
    return found;
  });
}

function renderIssue(id = "10042", origin = ORIGIN) {
  resetStore({ issueView: { origin, stack: [id] } });
  const client = createQueryClient();
  const view = render(
    <QueryClientProvider client={client}>
      <IssueView id={id} origin={origin} />
    </QueryClientProvider>,
  );
  return { ...view, client };
}

const stack = () => useStore.getState().issueView?.stack;
const title = (name: string) => screen.findByRole("heading", { level: 1, name });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.pinsList.mockResolvedValue(pins());
  mocked.getSettings.mockResolvedValue(settings());
  mocked.issueForget.mockResolvedValue(undefined);
  mocked.issuesMarkSeen.mockResolvedValue(undefined);
  mocked.pinRemove.mockResolvedValue(undefined);
  serve(issueDetail(), EPIC);
  mocked.issueRelated.mockResolvedValue(related());
  mocked.issueActivity.mockResolvedValue([]);
  mocked.issueSyncComments.mockResolvedValue(false);
  mocked.viewRows.mockResolvedValue(viewData([], []));
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space({ lists: [listNode({ id: 21, name: "My open issues" })] })]));
  mocked.viewsOf.mockResolvedValue([view()]);
});

describe("IssueView", () => {
  describe("header", () => {
    it("bookmarks the issue from the Bookmark this issue button", async () => {
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Bookmark this issue" }));
      await userEvent.click(screen.getByRole("button", { name: "Until removed" }));
      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "untilUnpinned", null);
    });

    it("removes the bookmark of a bookmarked issue from the same button", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 6 })]));
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Remove bookmark" }));

      expect(mocked.pinRemove).toHaveBeenCalledWith(6);
    });

    it("shows the breadcrumb with the Space, the view the issue was opened from and the key", async () => {
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      const crumbs = within(screen.getByRole("navigation", { name: "Breadcrumb" }));
      expect(await crumbs.findByText("List")).toBeInTheDocument();
      expect(crumbs.getByText("Website relaunch")).toBeInTheDocument();
      expect(crumbs.getByText("NW-1042")).toBeInTheDocument();
    });

    it("names the list instead of a view when opened from a filter list", async () => {
      renderIssue("10042", { listId: 21, viewId: 201 });

      const crumbs = within(screen.getByRole("navigation", { name: "Breadcrumb" }));
      expect(await crumbs.findByText("My open issues")).toBeInTheDocument();
      expect(crumbs.getByText("Website relaunch")).toBeInTheDocument();
    });

    it("opens the Space from its breadcrumb button", async () => {
      renderIssue();
      const crumbs = within(screen.getByRole("navigation", { name: "Breadcrumb" }));

      await userEvent.click(await crumbs.findByRole("button", { name: "Website relaunch" }));

      expect(mocked.navGo).toHaveBeenCalledWith(11, null);
    });

    it.each([
      ["view", { listId: 11, viewId: 101 }, "List"],
      ["filter list", { listId: 21, viewId: 201 }, "My open issues"],
    ])("opens the %s the issue was opened from, at its view, from its breadcrumb button", async (_kind, origin, name) => {
      renderIssue("10042", origin);
      const crumbs = within(screen.getByRole("navigation", { name: "Breadcrumb" }));

      await userEvent.click(await crumbs.findByRole("button", { name }));

      expect(mocked.navGo).toHaveBeenCalledWith(origin.listId, origin.viewId);
    });

    it("copies the link to the issue and says so in a toast on the Jira site", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "https://example.atlassian.net" }));
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Copy the link to NW-1042" }));

      expect(mocked.copyText).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-1042");
      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Copied the link to NW-1042"));
    });

    it("offers no copy link button without a site", async () => {
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      await waitFor(() => expect(mocked.connectionGet).toHaveBeenCalled());
      expect(screen.queryByRole("button", { name: /^Copy the link to/ })).not.toBeInTheDocument();
    });

    it("goes back to the origin from the back button", async () => {
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Back to List" }));

      expect(useStore.getState().issueView).toBeNull();
    });

    it("goes back to the previous issue when the issue was opened from another one", async () => {
      renderIssue();
      act(() => useStore.getState().openRelated("10001"));

      await userEvent.click(await screen.findByRole("button", { name: "Back to List" }));

      expect(stack()).toEqual(["10042"]);
    });

    it("offers Open in Jira on the Jira site", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "https://example.atlassian.net" }));
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Open in Jira" }));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-1042");
    });

    it("offers no Open in Jira without a site", async () => {
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      await waitFor(() => expect(mocked.connectionGet).toHaveBeenCalled());
      expect(screen.queryByRole("button", { name: "Open in Jira" })).not.toBeInTheDocument();
    });
  });

  describe("previous and next", () => {
    const rows: Row[] = [row({ id: "10041", key: "NW-1041" }), row({ id: "10042", key: "NW-1042" }), row({ id: "10043", key: "NW-1043" })];

    beforeEach(() => {
      mocked.viewRows.mockResolvedValue(viewData([groupOf(rows)], rows));
    });

    it("moves to the neighbours in the order of the view the issue was opened from", async () => {
      renderIssue("10042");
      const next = screen.getByRole("button", { name: "Next issue" });
      await waitFor(() => expect(next).toBeEnabled());

      await userEvent.click(next);
      expect(stack()).toEqual(["10043"]);
    });

    it("moves to the previous issue", async () => {
      renderIssue("10042");
      const previous = screen.getByRole("button", { name: "Previous issue" });
      await waitFor(() => expect(previous).toBeEnabled());

      await userEvent.click(previous);

      expect(stack()).toEqual(["10041"]);
    });

    it("disables Previous on the first issue", async () => {
      renderIssue("10041");

      await waitFor(() => expect(screen.getByRole("button", { name: "Next issue" })).toBeEnabled());
      expect(screen.getByRole("button", { name: "Previous issue" })).toBeDisabled();
    });

    it("disables Next on the last issue", async () => {
      renderIssue("10043");

      await waitFor(() => expect(screen.getByRole("button", { name: "Previous issue" })).toBeEnabled());
      expect(screen.getByRole("button", { name: "Next issue" })).toBeDisabled();
    });

    it("disables both when the issue is not in the origin view", async () => {
      renderIssue("10001");

      await waitFor(() => expect(mocked.viewRows).toHaveBeenCalled());
      expect(screen.getByRole("button", { name: "Previous issue" })).toBeDisabled();
      expect(screen.getByRole("button", { name: "Next issue" })).toBeDisabled();
    });
  });

  describe("main column", () => {
    it("shows the type badge, key, comment count and title", async () => {
      renderIssue();

      expect(await title("Checkout: address validation for DACH postcodes")).toBeInTheDocument();
      expect(screen.getByText("Story")).toBeInTheDocument();
      expect(screen.getByText("4 comments")).toBeInTheDocument();
    });

    it("uses the singular for one comment", async () => {
      serve(issueDetail({ commentCount: 1 }));
      renderIssue();

      expect(await screen.findByText("1 comment")).toBeInTheDocument();
    });

    it("shows the sanitised description as text without its tags", async () => {
      serve(issueDetail({ descriptionHtml: "<p>Validate <strong>DACH</strong> postcodes.</p><script>alert(1)</script>" }));
      renderIssue();

      const description = within(await screen.findByRole("region", { name: "Description" }));
      expect(description.getByText("DACH")).toBeInTheDocument();
      expect(description.queryByText(/alert/)).not.toBeInTheDocument();
    });

    it("says there is no description when there is none", async () => {
      serve(issueDetail({ descriptionHtml: null }));
      renderIssue();

      expect(await screen.findByText("No description.")).toBeInTheDocument();
    });

    it("says the issue is no longer in the cache when Rust cannot find it", async () => {
      mocked.issueGet.mockRejectedValue({ kind: "notFound", message: "No such issue", code: null });
      renderIssue();

      expect(await screen.findByText("This issue is no longer in the cache.")).toBeInTheDocument();
      expect(screen.queryByRole("heading", { level: 1 })).not.toBeInTheDocument();
    });

    it("lists the issue's web links", async () => {
      mocked.issueWebLinks.mockResolvedValue([pullRequestLink()]);
      renderIssue();

      const section = within(await screen.findByRole("region", { name: "Web links" }));
      expect(section.getByRole("button", { name: /Validate DACH postcodes/ })).toHaveTextContent("shop#481");
    });

    it("asks Rust to sync the issue's comments", async () => {
      renderIssue();

      await waitFor(() => expect(mocked.issueSyncComments).toHaveBeenCalledWith("10042"));
    });

    it("asks Rust to sync the issue's history and web links", async () => {
      renderIssue();

      await waitFor(() => expect(mocked.issueSyncHistory).toHaveBeenCalledWith("10042"));
      expect(mocked.issueSyncWebLinks).toHaveBeenCalledWith("10042");
    });

    it("shows no toast when the history and web link syncs fail", async () => {
      mocked.issueSyncHistory.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      mocked.issueSyncWebLinks.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      renderIssue();

      expect(await title("Checkout: address validation for DACH postcodes")).toBeInTheDocument();
      await waitFor(() => expect(mocked.issueSyncHistory).toHaveBeenCalled());
      expect(useStore.getState().toasts).toEqual([]);
    });

    it("keeps showing the cached issue when the comment sync fails", async () => {
      mocked.issueSyncComments.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      renderIssue();

      expect(await title("Checkout: address validation for DACH postcodes")).toBeInTheDocument();
    });
  });

  describe("an issue Jira deleted", () => {
    const GONE = issueDetail({ gone: true });

    it("shows the notice about the cached copy", async () => {
      serve(GONE);
      renderIssue();

      expect(await screen.findByText(/^Deleted in Jira or you lost access\. Last synced .+\. This is the cached copy\.$/)).toBeInTheDocument();
      expect(await title("Checkout: address validation for DACH postcodes")).toBeInTheDocument();
    });

    it("forgets the issue and closes the Issue view from Remove from cache", async () => {
      serve(GONE);
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Remove from cache" }));

      expect(mocked.issueForget).toHaveBeenCalledWith("10042");
      await waitFor(() => expect(useStore.getState().issueView).toBeNull());
    });

    it("keeps the Issue view open and shows a toast when forgetting fails", async () => {
      serve(GONE);
      mocked.issueForget.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Remove from cache" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not remove it from the cache: Disk full"));
      expect(useStore.getState().issueView).not.toBeNull();
    });

    it("offers Remove bookmark in the notice for a pinned issue", async () => {
      serve(GONE);
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 6 })]));
      renderIssue();

      await userEvent.click(await screen.findByText("Remove bookmark"));

      expect(mocked.pinRemove).toHaveBeenCalledWith(6);
    });

    it("shows no notice for a live issue", async () => {
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      expect(screen.queryByText(/Deleted in Jira/)).not.toBeInTheDocument();
    });
  });

  describe("unread changes", () => {
    const SINCE = { label: "Since you last looked", text: "Moved to Review." };
    const UNREAD = issueDetail({ since: SINCE, unread: true });

    afterEach(() => {
      vi.useRealTimers();
    });

    it("shows the banner with Mark as read, which marks the issue read", async () => {
      serve(UNREAD);
      renderIssue();

      expect(await screen.findByRole("status")).toHaveTextContent("Since you last looked: Moved to Review.");
      await userEvent.click(within(screen.getByRole("status")).getByRole("button", { name: "Mark as read" }));

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("shows an error toast when marking read fails", async () => {
      serve(UNREAD);
      mocked.issuesMarkSeen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderIssue();

      await userEvent.click(within(await screen.findByRole("status")).getByRole("button", { name: "Mark as read" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark it read: Disk full"));
    });

    it("shows no banner for an issue without unread changes", async () => {
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });

    describe("the header toggle", () => {
      it("reads Mark as read on an unread issue and marks it read", async () => {
        serve(issueDetail({ unread: true }));
        renderIssue();

        await userEvent.click(await screen.findByRole("button", { name: "Mark as read" }));

        expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
      });

      it("reads Mark as unread on a read issue and marks it unread", async () => {
        renderIssue();

        await userEvent.click(await screen.findByRole("button", { name: "Mark as unread" }));

        expect(mocked.issueMarkUnread).toHaveBeenCalledWith("10042");
        expect(screen.queryByRole("button", { name: "Mark as read" })).not.toBeInTheDocument();
      });

      it("shows an error toast when marking unread fails", async () => {
        mocked.issueMarkUnread.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
        renderIssue();

        await userEvent.click(await screen.findByRole("button", { name: "Mark as unread" }));

        await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark it unread: Disk full"));
      });
    });

    describe("marking unread while the issue is open", () => {
      async function markUnreadAndRefresh(client: ReturnType<typeof renderIssue>["client"]) {
        await userEvent.click(await screen.findByRole("button", { name: "Mark as unread" }));
        serve(issueDetail({ unread: true }), EPIC);
        await act(() => client.invalidateQueries({ queryKey: issueKey("10042") }));
        await screen.findByRole("button", { name: "Mark as read" });
      }

      it("keeps the issue unread instead of marking it read after the delay", async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        const { client } = renderIssue();

        await markUnreadAndRefresh(client);
        await vi.advanceTimersByTimeAsync(5000);

        expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      });

      it("keeps the issue unread when an activity item is marked unread", async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        mocked.issueActivity.mockResolvedValue([commentItem()]);
        const { client } = renderIssue();

        await userEvent.click(await screen.findByRole("button", { name: "Mark unread from here" }));
        serve(issueDetail({ unread: true }), EPIC);
        await act(() => client.invalidateQueries({ queryKey: issueKey("10042") }));
        await screen.findByRole("button", { name: "Mark as read" });
        await vi.advanceTimersByTimeAsync(5000);

        expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      });

      it("marks the issue read as usual once another issue was shown in between", async () => {
        vi.useFakeTimers({ shouldAdvanceTime: true });
        const { client, rerender } = renderIssue();
        await markUnreadAndRefresh(client);
        const show = (id: string) =>
          rerender(
            <QueryClientProvider client={client}>
              <IssueView id={id} origin={ORIGIN} />
            </QueryClientProvider>,
          );

        show("10001");
        await title("Checkout");
        await vi.advanceTimersByTimeAsync(3000);
        show("10042");
        await vi.advanceTimersByTimeAsync(2500);

        expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
      });
    });

    async function openUnseen() {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      serve(UNREAD);
      const view = renderIssue();
      await screen.findByRole("status");
      await vi.advanceTimersByTimeAsync(0);
      return view;
    }

    it("marks the issue read once it has been open for the default two seconds", async () => {
      await openUnseen();

      await vi.advanceTimersByTimeAsync(1500);
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
      await vi.advanceTimersByTimeAsync(700);

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("marks an unread issue read after the delay even without a banner", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      serve(issueDetail({ unread: true, since: null }));
      renderIssue();
      await screen.findByRole("button", { name: "Mark as read" });

      await vi.advanceTimersByTimeAsync(2500);

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("marks the issue read at once when the delay in the settings is 0", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 0, pinDefault: "untilUnpinned", openIssuesIn: "preview" } }));

      await openUnseen();
      await vi.advanceTimersByTimeAsync(50);

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
    });

    it("does not mark the issue read when the view is left before the delay", async () => {
      const { unmount } = await openUnseen();

      await vi.advanceTimersByTimeAsync(1000);
      unmount();
      await vi.advanceTimersByTimeAsync(5000);

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });

    it("never marks a read issue", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      renderIssue();
      await title("Checkout: address validation for DACH postcodes");

      await vi.advanceTimersByTimeAsync(5000);

      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });
  });

  describe("field grid", () => {
    it("shows status, assignee, epic, release, priority and sprint", async () => {
      serve(issueDetail({ sprint: "Sprint 24" }));
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      expect(screen.getByText("In Progress")).toBeInTheDocument();
      expect(screen.getByText("Alex Morgan")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Checkout" })).toBeInTheDocument();
      expect(screen.getByText("2.4.0")).toBeInTheDocument();
      expect(screen.getByText("High")).toBeInTheDocument();
      expect(screen.getByText("Sprint 24")).toBeInTheDocument();
    });

    it("shows None for the fields an issue does not have", async () => {
      serve(issueDetail({ epic: null, assignees: [], releases: [], priority: null, sprint: null }));
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      expect(screen.getAllByText("None")).toHaveLength(5);
    });

    it("shows the custom fields, a person with an avatar", async () => {
      serve(
        issueDetail({
          custom: [
            { id: "customfield_10071", name: "Reviewer", value: { kind: "person", value: SAM }, html: null },
            { id: "customfield_10072", name: "Story points", value: { kind: "text", value: "5" }, html: null },
            { id: "customfield_10073", name: "Client", value: null, html: null },
          ],
        }),
      );
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      expect(screen.getByText("Reviewer")).toBeInTheDocument();
      expect(screen.getByText("Sam Lee")).toBeInTheDocument();
      expect(screen.getByRole("img", { name: "Sam Lee" })).toBeInTheDocument();
      expect(screen.getByText("Story points")).toBeInTheDocument();
      expect(screen.getByText("5")).toBeInTheDocument();
      expect(screen.getByText("Client")).toBeInTheDocument();
    });

    it("shows a rich text custom field as a section after the description, not in the grid", async () => {
      serve(
        issueDetail({
          custom: [{ id: "customfield_10080", name: "Acceptance criteria", value: null, html: "<p>Postcodes <strong>validate</strong> on blur.</p><script>alert(1)</script>" }],
        }),
      );
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      const section = screen.getByRole("region", { name: "Acceptance criteria" });
      expect(within(section).getByRole("heading", { name: "Acceptance criteria" })).toBeInTheDocument();
      expect(section).toHaveTextContent("Postcodes validate on blur.");
      expect(screen.queryByText(/alert/)).not.toBeInTheDocument();
      expect(screen.getByRole("region", { name: "Description" }).compareDocumentPosition(section) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(screen.getAllByText("Acceptance criteria")).toHaveLength(1);
    });

    it.each(["", "   ", "<p> </p>"])("shows no section for a rich text custom field whose html is %j", async (html) => {
      serve(issueDetail({ custom: [{ id: "customfield_10080", name: "Acceptance criteria", value: null, html }] }));
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      expect(screen.queryByRole("region", { name: "Acceptance criteria" })).not.toBeInTheDocument();
    });

    it("leaves out Epic and Sprint for an epic", async () => {
      renderIssue("10001");

      await title("Checkout");
      expect(screen.queryByText("Sprint")).not.toBeInTheDocument();
      expect(screen.getAllByText("Epic")).toHaveLength(1);
      expect(screen.getByText("Release")).toBeInTheDocument();
    });

    it("opens the epic when its chip is clicked", async () => {
      renderIssue();

      await userEvent.click(await screen.findByRole("button", { name: "Checkout" }));

      expect(stack()).toEqual(["10042", "10001"]);
    });
  });

  describe("links", () => {
    it("opens a linked issue", async () => {
      mocked.issueRelated.mockResolvedValue(related({ links: [{ relation: "blocks", issue: relatedIssue() }] }));
      renderIssue();

      const links = within(await screen.findByRole("region", { name: "Links" }));
      expect(links.getByText("blocks")).toBeInTheDocument();
      await userEvent.click(links.getByRole("button", { name: /NW-1051/ }));

      expect(stack()).toEqual(["10042", "10051"]);
    });

    it("shows a linked issue the cache does not hold as disabled", async () => {
      mocked.issueRelated.mockResolvedValue(related({ links: [{ relation: "is blocked by", issue: relatedIssue({ id: null }) }] }));
      renderIssue();

      expect(await within(await screen.findByRole("region", { name: "Links" })).findByRole("button", { name: /NW-1051/ })).toBeDisabled();
    });

    it("has no Links section without links", async () => {
      renderIssue();

      await title("Checkout: address validation for DACH postcodes");
      await waitFor(() => expect(mocked.issueRelated).toHaveBeenCalled());
      expect(screen.queryByRole("region", { name: "Links" })).not.toBeInTheDocument();
    });
  });

  describe("children", () => {
    const children = {
      open: [relatedIssue({ id: "10051", key: "NW-1051" }), relatedIssue({ id: "10052", key: "NW-1052", summary: "Saved addresses" })],
      done: [
        relatedIssue({ id: "10053", key: "NW-1053", summary: "Cart totals", style: "done" }),
        relatedIssue({ id: "10054", key: "NW-1054", summary: "Payment methods", style: "done" }),
      ],
    };

    it("lists an epic's open Child issues and keeps the done ones behind a toggle", async () => {
      mocked.issueRelated.mockResolvedValue(related({ children }));
      renderIssue("10001");

      const section = within(await screen.findByRole("region", { name: "Child issues" }));
      expect(section.getByRole("button", { name: /NW-1051/ })).toBeInTheDocument();
      expect(section.getByRole("button", { name: /NW-1052/ })).toBeInTheDocument();
      expect(section.queryByRole("button", { name: /NW-1053/ })).not.toBeInTheDocument();
    });

    it("shows and hides the done children from the toggle", async () => {
      mocked.issueRelated.mockResolvedValue(related({ children }));
      renderIssue("10001");
      const section = within(await screen.findByRole("region", { name: "Child issues" }));

      await userEvent.click(section.getByRole("button", { name: "Show 2 done" }));

      expect(section.getByRole("button", { name: /NW-1053/ })).toBeInTheDocument();
      expect(section.getByRole("button", { name: /NW-1054/ })).toBeInTheDocument();

      await userEvent.click(section.getByRole("button", { name: "Hide 2 done" }));

      expect(section.queryByRole("button", { name: /NW-1053/ })).not.toBeInTheDocument();
    });

    it("has no done toggle when every child is open", async () => {
      mocked.issueRelated.mockResolvedValue(related({ children: { open: children.open, done: [] } }));
      renderIssue("10001");

      await screen.findByRole("region", { name: "Child issues" });
      expect(screen.queryByRole("button", { name: /done$/ })).not.toBeInTheDocument();
    });

    it("opens a child issue", async () => {
      mocked.issueRelated.mockResolvedValue(related({ children }));
      renderIssue("10001");

      await userEvent.click(await screen.findByRole("button", { name: /NW-1052/ }));

      expect(stack()).toEqual(["10001", "10052"]);
    });

    it("narrows the origin view to the epic and closes the Issue view from Open as list view", async () => {
      mocked.issueRelated.mockResolvedValue(related({ children }));
      renderIssue("10001");

      await userEvent.click(await screen.findByRole("button", { name: "Open as list view" }));

      expect(useStore.getState().narrowings[narrowingKey(11, 101)]).toMatchObject({ epic: "10001" });
      expect(useStore.getState().issueView).toBeNull();
    });

    it("calls the children of a task Subtasks and offers no list view", async () => {
      mocked.issueRelated.mockResolvedValue(related({ children }));
      renderIssue("10042");

      expect(await screen.findByRole("region", { name: "Subtasks" })).toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "Child issues" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Open as list view" })).not.toBeInTheDocument();
    });

    it("has no section when there are no children", async () => {
      renderIssue("10001");

      await title("Checkout");
      await waitFor(() => expect(mocked.issueRelated).toHaveBeenCalled());
      expect(screen.queryByRole("region", { name: "Child issues" })).not.toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "Subtasks" })).not.toBeInTheDocument();
    });
  });

  describe("issue keys", () => {
    const press = (id: string) => act(async () => void (await getCommand(id)?.run()));

    beforeEach(() => {
      installCommands("main");
    });

    it("M marks an unread issue read", async () => {
      serve(issueDetail({ unread: true }));
      renderIssue("10042");
      await title("Checkout: address validation for DACH postcodes");

      await press("issue.markSeen");

      expect(mocked.issuesMarkSeen).toHaveBeenCalledWith(["10042"]);
      expect(mocked.issueMarkUnread).not.toHaveBeenCalled();
    });

    it("M marks a read issue unread", async () => {
      renderIssue("10042");
      await title("Checkout: address validation for DACH postcodes");

      await press("issue.markSeen");

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("10042");
      expect(mocked.issuesMarkSeen).not.toHaveBeenCalled();
    });

    it("shows an error toast when M cannot mark the issue read", async () => {
      serve(issueDetail({ unread: true }));
      mocked.issuesMarkSeen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderIssue("10042");
      await title("Checkout: address validation for DACH postcodes");

      await press("issue.markSeen");

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark it read: Disk full"));
    });

    it("shows an error toast when M cannot mark the issue unread", async () => {
      mocked.issueMarkUnread.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderIssue("10042");
      await title("Checkout: address validation for DACH postcodes");

      await press("issue.markSeen");

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark it unread: Disk full"));
    });

    it.each([
      ["↩", "Enter"],
      ["Space", " "],
    ])("leaves %s to the page, which has no command for it", async (_name, key) => {
      renderIssue("10042");
      await title("Checkout: address validation for DACH postcodes");

      const command = resolveKeydown({ key, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false }, ["global", "main"]);

      expect(command).toBeNull();
    });

    it("still resolves M to its command, so the page really has the issue keys", async () => {
      renderIssue("10042");
      await title("Checkout: address validation for DACH postcodes");

      const command = resolveKeydown({ key: "m", metaKey: false, ctrlKey: false, altKey: false, shiftKey: false }, ["global", "main"]);

      expect(command?.id).toBe("issue.markSeen");
    });
  });
});
