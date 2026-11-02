import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type WriteOutcome } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { behaviour, settings } from "../test/settings";
import { CommentBox } from "./CommentBox";

const mocked = vi.mocked(api, true);

const allowChanges = (on = true) => mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: on }) }));
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

function renderBox() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <CommentBox issueId="10042" />
    </QueryClientProvider>,
  );
}

const box = () => screen.findByRole("textbox", { name: "Add a comment" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
});

describe("CommentBox", () => {
  it("is absent while changes to Jira are off", async () => {
    allowChanges(false);
    renderBox();
    await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

    expect(screen.queryByRole("textbox", { name: "Add a comment" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Comment" })).not.toBeInTheDocument();
  });

  it("is absent while commenting is switched off, though changes to Jira are on", async () => {
    mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["comment"] }) }));
    renderBox();
    await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

    expect(screen.queryByRole("textbox", { name: "Add a comment" })).not.toBeInTheDocument();
  });

  it("stays when only another kind of change is switched off", async () => {
    mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["assign", "addWebLink"] }) }));
    renderBox();

    expect(await box()).toBeInTheDocument();
  });

  it("keeps Comment disabled until there is text beyond whitespace", async () => {
    renderBox();
    const input = await box();
    expect(screen.getByRole("button", { name: "Comment" })).toBeDisabled();

    await userEvent.type(input, "   ");
    expect(screen.getByRole("button", { name: "Comment" })).toBeDisabled();

    await userEvent.type(input, "Looks good");
    expect(screen.getByRole("button", { name: "Comment" })).toBeEnabled();
  });

  it("sends the text when Comment is clicked, then clears the box and confirms", async () => {
    mocked.issueComment.mockResolvedValue({ done: "Commented on NW-1042", reverse: null });
    renderBox();
    const input = await box();

    await userEvent.type(input, "Ready for review");
    await userEvent.click(screen.getByRole("button", { name: "Comment" }));

    expect(mocked.issueComment).toHaveBeenCalledWith("10042", "Ready for review");
    await vi.waitFor(() => expect(input).toHaveValue(""));
    expect(toasts()).toEqual([["success", "Commented on NW-1042"]]);
  });

  it("sends the text on ⌘↩", async () => {
    renderBox();
    const input = await box();

    await userEvent.type(input, "Ready for review");
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

    expect(mocked.issueComment).toHaveBeenCalledWith("10042", "Ready for review");
    await vi.waitFor(() => expect(input).toHaveValue(""));
  });

  it("empties the box and gives up focus once a comment sent with ⌘↩ went through", async () => {
    mocked.issueComment.mockResolvedValue({ done: "Commented on NW-1042", reverse: null });
    renderBox();
    const input = await box();

    await userEvent.type(input, "Ready for review");
    expect(input).toHaveFocus();
    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

    await vi.waitFor(() => expect(input).toHaveValue(""));
    expect(input).not.toHaveFocus();
    expect(document.activeElement).not.toBe(input);
  });

  it("does not send an empty comment on ⌘↩", async () => {
    renderBox();
    await userEvent.click(await box());

    await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

    expect(mocked.issueComment).not.toHaveBeenCalled();
  });

  it("reads Sending… and locks the box while the comment is on its way", async () => {
    let finish: (outcome: WriteOutcome) => void = () => {};
    mocked.issueComment.mockReturnValue(new Promise((resolve) => (finish = resolve)));
    renderBox();
    const input = await box();
    await userEvent.type(input, "Ready for review");

    await userEvent.click(screen.getByRole("button", { name: "Comment" }));

    expect(screen.getByRole("button", { name: "Sending…" })).toBeDisabled();
    expect(input).toBeDisabled();
    finish({ done: "Commented on NW-1042", reverse: null });
    expect(await screen.findByRole("button", { name: "Comment" })).toBeDisabled();
    expect(input).toBeEnabled();
  });

  it("keeps the text and shows an error toast when Jira refuses the comment", async () => {
    mocked.issueComment.mockRejectedValue({ kind: "auth", message: "No permission to comment", code: null });
    renderBox();
    const input = await box();

    await userEvent.type(input, "Ready for review");
    await userEvent.click(screen.getByRole("button", { name: "Comment" }));

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not add the comment: No permission to comment"]]));
    expect(input).toHaveValue("Ready for review");
    expect(input).toBeEnabled();
  });

  describe("keys", () => {
    it("keeps typed keys and ⌘↩ from reaching the page's shortcuts", async () => {
      const page = vi.fn();
      document.addEventListener("keydown", page);
      renderBox();

      await userEvent.type(await box(), "j");
      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      document.removeEventListener("keydown", page);
      expect(page).not.toHaveBeenCalled();
    });

    it("leaves the box on Escape", async () => {
      renderBox();
      const input = await box();
      await userEvent.click(input);
      expect(input).toHaveFocus();

      await userEvent.keyboard("{Escape}");

      expect(input).not.toHaveFocus();
    });
  });
});
