import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type ActivityItem } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { ALEX, SAM, changeItem, commentItem } from "../test/issues";
import { Activity } from "./Activity";

const mocked = vi.mocked(api, true);

function renderActivity(items: ActivityItem[]) {
  mocked.issueActivity.mockResolvedValue(items);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Activity id="10042" />
    </QueryClientProvider>,
  );
}

const created = () => changeItem({ field: "created", from: null, to: null });

/** The whole line of the change whose text contains this one. */
const lineOf = async (text: string) => (await screen.findByText(text, { exact: false })).closest("div")!;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("Activity", () => {
  it("lists comments with their author and body", async () => {
    renderActivity([commentItem({ author: SAM, bodyHtml: "<p>Ship it after the postcode fix.</p>" })]);

    expect(await screen.findByText("Sam Lee")).toBeInTheDocument();
    expect(screen.getByText("Ship it after the postcode fix.")).toBeInTheDocument();
  });

  it("marks an edited comment and names an unknown author", async () => {
    renderActivity([commentItem({ author: null, edited: true })]);

    expect(await screen.findByText("Someone")).toBeInTheDocument();
    expect(screen.getByText(/· edited/)).toBeInTheDocument();
  });

  it("describes a status change with the two statuses as pills", async () => {
    renderActivity([changeItem()]);

    expect(await lineOf("Changed status")).toHaveTextContent("Changed status In Review → Done");
    expect(screen.getByText("In Review")).toHaveClass("status-pill--activity");
    expect(screen.getByText("Done")).toHaveClass("status-pill--activity");
  });

  it("names the people in an assignee change and shows none for an empty side", async () => {
    renderActivity([changeItem({ field: "assignee", from: null, to: { kind: "person", value: ALEX } })]);

    expect(await lineOf("Changed assignee")).toHaveTextContent("Changed assignee none → Alex Morgan");
  });

  it("describes the creation of the issue", async () => {
    renderActivity([created()]);

    expect(await screen.findByText("Created the issue")).toBeInTheDocument();
  });

  describe("change lines", () => {
    const text = (value: string) => ({ kind: "text" as const, value });

    it("starts with the author's name in bold and continues in lowercase", async () => {
      renderActivity([changeItem({ author: SAM })]);

      const author = await screen.findByText("Sam Lee");
      expect(author.tagName).toBe("STRONG");
      expect(author.closest("div")).toHaveTextContent("Sam Lee changed status In Review → Done");
    });

    it("reads an in-progress move by a named author as one sentence", async () => {
      const review = { kind: "status" as const, value: { name: "In Review", style: "review" as const } };
      const progress = { kind: "status" as const, value: { name: "In Progress", style: "progress" as const } };
      renderActivity([changeItem({ author: { ...SAM, name: "Marta Klein", initials: "MK" }, from: review, to: progress })]);

      expect(await lineOf("changed status")).toHaveTextContent("Marta Klein changed status In Review → In Progress");
    });

    it("starts with a capital Changed when the author is unknown", async () => {
      renderActivity([changeItem({ author: null })]);

      const line = await lineOf("Changed status");
      expect(line).toHaveTextContent("Changed status In Review → Done");
      expect(line).not.toHaveTextContent("changed status");
    });

    it("names the author who created the issue", async () => {
      renderActivity([changeItem({ field: "created", from: null, to: null, author: ALEX })]);

      const author = await screen.findByText("Alex Morgan");
      expect(author.parentElement).toHaveTextContent("Alex Morgan created the issue");
    });

    it("names the author of a change without values and shows no none", async () => {
      renderActivity([changeItem({ field: "description", from: null, to: null, author: ALEX })]);

      const author = await screen.findByText("Alex Morgan");
      expect(author.parentElement).toHaveTextContent(/^Alex Morgan changed description$/);
    });

    it("says Changed description without an author and without values", async () => {
      renderActivity([changeItem({ field: "description", from: null, to: null, author: null })]);

      const line = await screen.findByText("Changed description");
      expect(line).not.toHaveTextContent("none");
    });

    it.each([
      ["due_date", "due date"],
      ["duedate", "due date"],
      ["Fix Version", "fix version"],
      ["status", "status"],
      ["assignee", "assignee"],
    ])("shows the field %s as %s", async (field, shown) => {
      renderActivity([changeItem({ field, from: text("a"), to: text("b"), author: null })]);

      expect(await lineOf(`Changed ${shown}`)).toHaveTextContent(`Changed ${shown} a → b`);
    });
  });

  it("lists the items in the order received, newest first", async () => {
    renderActivity([changeItem(), commentItem({ id: "1", author: SAM }), created()]);

    await screen.findByText("Sam Lee");
    const text = screen.getByRole("complementary", { name: "Activity" }).textContent ?? "";
    expect(text.indexOf("Changed status")).toBeLessThan(text.indexOf("Sam Lee"));
    expect(text.indexOf("Sam Lee")).toBeLessThan(text.indexOf("Created the issue"));
  });

  describe("width", () => {
    const handle = () => screen.findByRole("separator", { name: "Activity width" });

    it("is 400 by default", async () => {
      renderActivity([]);

      expect(await handle()).toHaveAttribute("aria-valuenow", "400");
    });

    it.each([
      [500, "500"],
      [1000, "720"],
      [100, "300"],
    ])("shows the saved width %s clamped to %s", async (saved, shown) => {
      resetStore({ activityWidth: saved });
      renderActivity([]);

      expect(await handle()).toHaveAttribute("aria-valuenow", shown);
    });

    it("widens by 16 on ArrowLeft and saves it", async () => {
      renderActivity([]);
      (await handle()).focus();

      await userEvent.keyboard("{ArrowLeft}");

      expect(await handle()).toHaveAttribute("aria-valuenow", "416");
      expect(mocked.setUiState).toHaveBeenLastCalledWith({ sidebarHidden: false, activityWidth: 416 });
    });

    it("narrows by 16 on ArrowRight and saves it", async () => {
      renderActivity([]);
      (await handle()).focus();

      await userEvent.keyboard("{ArrowRight}");

      expect(await handle()).toHaveAttribute("aria-valuenow", "384");
      expect(mocked.setUiState).toHaveBeenLastCalledWith({ sidebarHidden: false, activityWidth: 384 });
    });

    it("stops widening at the maximum", async () => {
      resetStore({ activityWidth: 720 });
      renderActivity([]);
      (await handle()).focus();

      await userEvent.keyboard("{ArrowLeft}");

      expect(await handle()).toHaveAttribute("aria-valuenow", "720");
    });
  });

  describe("new since yesterday", () => {
    const old = () => commentItem({ id: "1", author: SAM, bodyHtml: "<p>Old note</p>" });
    const first = () => commentItem({ id: "2", author: SAM, bodyHtml: "<p>First new note</p>", fresh: true });
    const second = () => commentItem({ id: "3", author: SAM, bodyHtml: "<p>Second new note</p>", fresh: true });

    it("puts one separator above the fresh items at the top, ahead of the older ones", async () => {
      renderActivity([second(), first(), old()]);

      const separator = await screen.findByRole("separator", { name: "" });
      expect(separator).toHaveTextContent("New since yesterday");
      expect(screen.getAllByRole("separator", { name: "" })).toHaveLength(1);
      const text = screen.getByRole("complementary", { name: "Activity" }).textContent ?? "";
      expect(text.indexOf("New since yesterday")).toBeLessThan(text.indexOf("Second new note"));
      expect(text.indexOf("Second new note")).toBeLessThan(text.indexOf("First new note"));
      expect(text.indexOf("First new note")).toBeLessThan(text.indexOf("Old note"));
    });

    it("shows no separator when no item is fresh", async () => {
      renderActivity([changeItem(), old()]);

      await screen.findByText("Old note");
      expect(screen.queryByRole("separator", { name: "" })).not.toBeInTheDocument();
    });
  });

  describe("mark unread from here", () => {
    it("marks the issue unread from a comment's time", async () => {
      renderActivity([commentItem({ at: "2026-10-05T10:00:00Z" })]);

      await userEvent.click(await screen.findByRole("button", { name: "Mark unread from here" }));

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("10042", "2026-10-05T10:00:00Z");
    });

    it("marks the issue unread from a change line's time", async () => {
      renderActivity([changeItem({ at: "2026-10-06T11:00:00Z" })]);

      await userEvent.click(await screen.findByRole("button", { name: "Mark unread from here" }));

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("10042", "2026-10-06T11:00:00Z");
    });

    it("offers the button on every comment and change", async () => {
      renderActivity([changeItem(), commentItem({ id: "1" }), created()]);

      await screen.findByText("Sam Lee");
      expect(screen.getAllByRole("button", { name: "Mark unread from here" })).toHaveLength(3);
    });

    it("sends the time of the item whose button was clicked", async () => {
      renderActivity([commentItem({ id: "2", at: "2026-10-07T08:00:00Z" }), commentItem({ id: "1", at: "2026-10-05T10:00:00Z" })]);

      const buttons = await screen.findAllByRole("button", { name: "Mark unread from here" });
      await userEvent.click(buttons[1]);

      expect(mocked.issueMarkUnread).toHaveBeenCalledWith("10042", "2026-10-05T10:00:00Z");
    });

    it("shows an error toast when marking unread fails", async () => {
      mocked.issueMarkUnread.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderActivity([commentItem()]);

      await userEvent.click(await screen.findByRole("button", { name: "Mark unread from here" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark it unread: Disk full"));
    });
  });

  describe("filter", () => {
    const items = () => [changeItem(), commentItem({ author: SAM }), created()];

    it("shows everything by default", async () => {
      renderActivity(items());

      expect(await screen.findByText("Sam Lee")).toBeInTheDocument();
      expect(screen.getByText("Created the issue")).toBeInTheDocument();
      expect(screen.getByText(/Changed status/)).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "All" })).toHaveAttribute("aria-pressed", "true");
    });

    it("shows only comments when Comments is chosen", async () => {
      renderActivity(items());
      await screen.findByText("Sam Lee");

      await userEvent.click(screen.getByRole("button", { name: "Comments" }));

      expect(screen.getByRole("button", { name: "Comments" })).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByRole("button", { name: "All" })).toHaveAttribute("aria-pressed", "false");
      expect(screen.getByText("Sam Lee")).toBeInTheDocument();
      expect(screen.queryByText("Created the issue")).not.toBeInTheDocument();
      expect(screen.queryByText(/Changed status/)).not.toBeInTheDocument();
    });

    it("shows only changes when Changes is chosen", async () => {
      renderActivity(items());
      await screen.findByText("Sam Lee");

      await userEvent.click(screen.getByRole("button", { name: "Changes" }));

      expect(screen.queryByText("Sam Lee")).not.toBeInTheDocument();
      expect(screen.getByText("Created the issue")).toBeInTheDocument();
      expect(screen.getByText(/Changed status/)).toBeInTheDocument();
    });

    it("says there is nothing when the chosen filter matches no entry", async () => {
      renderActivity([commentItem()]);
      await screen.findByText("Sam Lee");

      await userEvent.click(screen.getByRole("button", { name: "Changes" }));

      expect(screen.getByText("Nothing here yet.")).toBeInTheDocument();
    });
  });

  it("says there is nothing yet when the issue has no activity", async () => {
    renderActivity([]);

    expect(await screen.findByText("Nothing here yet.")).toBeInTheDocument();
  });

  it("offers no comment box or placeholder about commenting", async () => {
    renderActivity([]);

    await screen.findByText("Nothing here yet.");
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(screen.queryByText(/commenting|reply in jira/i)).not.toBeInTheDocument();
  });
});
