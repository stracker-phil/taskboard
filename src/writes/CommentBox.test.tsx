import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Person, type WriteOutcome } from "../api";
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

const box = () => screen.findByRole("combobox", { name: "Add a comment" });

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

    expect(screen.queryByRole("combobox", { name: "Add a comment" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Comment" })).not.toBeInTheDocument();
  });

  it("is absent while commenting is switched off, though changes to Jira are on", async () => {
    mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["comment"] }) }));
    renderBox();
    await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

    expect(screen.queryByRole("combobox", { name: "Add a comment" })).not.toBeInTheDocument();
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

  describe("height", () => {
    const stubContentHeight = (el: HTMLElement, height: number) =>
      Object.defineProperty(el, "scrollHeight", { configurable: true, get: () => height });

    it("starts as two rows when empty", async () => {
      renderBox();

      expect(await box()).toHaveAttribute("rows", "2");
    });

    it("grows to the height of the typed lines", async () => {
      renderBox();
      const input = await box();
      stubContentHeight(input, 120);

      await userEvent.type(input, "First line{Shift>}{Enter}{/Shift}Second line{Shift>}{Enter}{/Shift}Third line");

      expect(input.style.height).toBe("120px");
    });

    it("shrinks back to the empty content height after a comment is sent with ⌘↩", async () => {
      mocked.issueComment.mockResolvedValue({ done: "Commented on NW-1042", reverse: null });
      renderBox();
      const input = await box();
      stubContentHeight(input, 120);
      await userEvent.type(input, "First line{Shift>}{Enter}{/Shift}Second line");
      expect(input.style.height).toBe("120px");

      stubContentHeight(input, 64);
      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      await vi.waitFor(() => expect(input).toHaveValue(""));
      expect(input.style.height).toBe("64px");
    });
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

  it("hints at Markdown, mentions and the send shortcut", async () => {
    renderBox();
    await box();

    expect(screen.getByText("Markdown · @ mentions · ⌘↩ to send")).toBeInTheDocument();
  });

  describe("mentions", () => {
    const marta: Person = { accountId: "acc-marta", name: "Marta Klein", initials: "MK", color: 1 };
    const jonas: Person = { accountId: "acc-jonas", name: "Jonas Reiter", initials: "JR", color: 2 };
    const list = () => screen.queryByRole("listbox", { name: "People to mention" });
    const options = () => screen.getAllByRole("option");
    const expectSelected = (name: string) => {
      const chosen = screen.getAllByRole("option", { selected: true });
      expect(chosen).toHaveLength(1);
      expect(chosen[0]).toHaveAccessibleName(name);
    };
    const expectOptions = (...names: string[]) => {
      expect(options()).toHaveLength(names.length);
      names.forEach((name, i) => expect(options()[i]).toHaveAccessibleName(name));
    };

    async function typeInto(text: string) {
      renderBox();
      const input = await box();
      await vi.waitFor(() => expect(mocked.issueMentionable).toHaveBeenCalledWith("10042"));
      await userEvent.type(input, text);
      return input;
    }

    beforeEach(() => {
      mocked.issueMentionable.mockResolvedValue([marta, jonas]);
    });

    it("lists only the people matching the typed @name", async () => {
      await typeInto("Thanks @ma");

      expect(await screen.findByRole("listbox", { name: "People to mention" })).toBeInTheDocument();
      expectOptions("Marta Klein");
    });

    it("lists everyone once only the @ is typed, with the first person selected", async () => {
      await typeInto("@");

      await screen.findByRole("listbox", { name: "People to mention" });
      expectOptions("Marta Klein", "Jonas Reiter");
      expectSelected("Marta Klein");
    });

    it("shows no list when nobody matches", async () => {
      await typeInto("Thanks @zz");

      expect(list()).not.toBeInTheDocument();
    });

    it("shows no list for an @ inside a word", async () => {
      await typeInto("write to a@ma");

      expect(list()).not.toBeInTheDocument();
    });

    it("moves the selection with the arrow keys and wraps at both ends", async () => {
      await typeInto("@");
      await screen.findByRole("listbox", { name: "People to mention" });

      await userEvent.keyboard("{ArrowDown}");
      expectSelected("Jonas Reiter");
      await userEvent.keyboard("{ArrowDown}");
      expectSelected("Marta Klein");
      await userEvent.keyboard("{ArrowUp}");
      expectSelected("Jonas Reiter");
    });

    it("inserts the selected person on Enter, closes the list and does not send the comment", async () => {
      const input = await typeInto("Thanks @ma");
      await screen.findByRole("listbox", { name: "People to mention" });

      await userEvent.keyboard("{Enter}");

      expect(input).toHaveValue("Thanks @Marta Klein ");
      expect(list()).not.toBeInTheDocument();
      expect(mocked.issueComment).not.toHaveBeenCalled();
    });

    it("inserts the person chosen with the arrow keys on Enter", async () => {
      const input = await typeInto("@");
      await screen.findByRole("listbox", { name: "People to mention" });

      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(input).toHaveValue("@Jonas Reiter ");
    });

    it("inserts the selected person on Tab and keeps the focus in the field", async () => {
      const input = await typeInto("Thanks @ma");
      await screen.findByRole("listbox", { name: "People to mention" });

      await userEvent.keyboard("{Tab}");

      expect(input).toHaveValue("Thanks @Marta Klein ");
      expect(input).toHaveFocus();
    });

    it("inserts a person when their option is clicked", async () => {
      const input = await typeInto("Thanks @jo");

      await userEvent.click(await screen.findByRole("option", { name: /Jonas Reiter/ }));

      expect(input).toHaveValue("Thanks @Jonas Reiter ");
      expect(list()).not.toBeInTheDocument();
      expect(input).toHaveFocus();
    });

    it("closes the list on Escape and keeps the focus, a second Escape leaves the field", async () => {
      const input = await typeInto("Thanks @ma");
      await screen.findByRole("listbox", { name: "People to mention" });

      await userEvent.keyboard("{Escape}");
      expect(list()).not.toBeInTheDocument();
      expect(input).toHaveFocus();
      expect(input).toHaveValue("Thanks @ma");

      await userEvent.keyboard("{Escape}");
      expect(input).not.toHaveFocus();
    });

    it("sends the comment on ⌘↩ while the list is open", async () => {
      await typeInto("Thanks @ma");
      await screen.findByRole("listbox", { name: "People to mention" });

      await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

      expect(mocked.issueComment).toHaveBeenCalledWith("10042", "Thanks @ma");
    });

    it("is a combobox whose aria-expanded follows the list", async () => {
      renderBox();
      const input = await screen.findByRole("combobox", { name: "Add a comment" });
      await vi.waitFor(() => expect(mocked.issueMentionable).toHaveBeenCalled());
      expect(input).toHaveAttribute("aria-expanded", "false");

      await userEvent.type(input, "@ma");
      await screen.findByRole("listbox", { name: "People to mention" });
      expect(input).toHaveAttribute("aria-expanded", "true");

      await userEvent.keyboard("{Enter}");
      expect(input).toHaveAttribute("aria-expanded", "false");
    });
  });
});
