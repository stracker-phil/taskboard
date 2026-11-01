import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Behaviour, type IssueDetail } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { issueDetail } from "../test/issues";
import { behaviour, settings } from "../test/settings";
import { EditableTitle, requestRename } from "./EditableTitle";

const mocked = vi.mocked(api, true);

const OLD = "Checkout: address validation for DACH postcodes";
const NEW = "Checkout: validate DACH postcodes";
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);
const allowChanges = (patch: Partial<Behaviour> = {}) =>
  mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, ...patch }) }));

function renderTitle(issue: IssueDetail = issueDetail(), as: "h1" | "h2" = "h2") {
  const client = createQueryClient();
  const tree = (shown: IssueDetail) => (
    <QueryClientProvider client={client}>
      <EditableTitle key={shown.id} issue={shown} as={as} className="peek-title" />
    </QueryClientProvider>
  );
  const view = render(tree(issue));
  return { ...view, show: (next: IssueDetail) => view.rerender(tree(next)) };
}

const heading = (name: string) => screen.getByRole("heading", { name });
const textbox = () => screen.getByRole("textbox", { name: "Title of NW-1042" });

/** Double-clicks until the settings have arrived and the heading turns editable. */
async function startEditing(name = OLD) {
  await waitFor(() => {
    fireEvent.doubleClick(heading(name));
    expect(textbox()).toBeInTheDocument();
  });
  return textbox();
}

/** Lets the settings arrive, so that a double-click that does nothing is a verdict, not a race. */
async function settle() {
  await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
  await act(() => new Promise((resolve) => setTimeout(resolve, 20)));
}

function type(box: HTMLElement, text: string) {
  box.textContent = text;
  fireEvent.input(box);
}

let hasFocus: MockInstance<() => boolean>;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
  mocked.issueWrite.mockResolvedValue({ done: "Renamed NW-1042", reverse: null });
  hasFocus = vi.spyOn(document, "hasFocus").mockReturnValue(true);
});

afterEach(() => {
  hasFocus.mockRestore();
  // A request nobody consumed must not start an edit in the next test.
  requestRename("none");
});

describe("EditableTitle", () => {
  it.each(["h1", "h2"] as const)("shows the title as a level %s heading", (as) => {
    renderTitle(issueDetail(), as);

    expect(screen.getByRole("heading", { level: Number(as[1]), name: OLD })).toBeInTheDocument();
  });

  describe("starting an edit", () => {
    it("turns the heading into an editable textbox showing the title when it is double-clicked", async () => {
      renderTitle();

      const box = await startEditing();

      expect(box).toHaveAttribute("contenteditable", "plaintext-only");
      expect(box).toHaveTextContent(OLD);
      expect(screen.queryByRole("heading", { name: OLD })).not.toBeInTheDocument();
    });

    it("selects the whole title", async () => {
      renderTitle();

      await startEditing();

      expect(window.getSelection()?.toString()).toBe(OLD);
    });

    it("starts when a rename of this issue was requested", async () => {
      requestRename("10042");
      renderTitle();

      expect(await screen.findByRole("textbox", { name: "Title of NW-1042" })).toHaveTextContent(OLD);
    });

    it("ignores a rename requested for another issue", async () => {
      requestRename("10099");
      renderTitle();
      await settle();

      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(heading(OLD)).toBeInTheDocument();
    });

    it.each([
      ["changes to Jira are off", { allowJiraChanges: false }],
      ["renaming is switched off", { disabledChanges: ["changeTitle" as const] }],
    ])("does nothing on a double-click while %s", async (_label, patch) => {
      allowChanges(patch);
      renderTitle();
      await settle();

      fireEvent.doubleClick(heading(OLD));

      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("does nothing on a double-click for an issue Jira deleted", async () => {
      renderTitle(issueDetail({ gone: true }));
      await settle();

      fireEvent.doubleClick(heading(OLD));

      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("does not start a requested rename while renaming is off", async () => {
      allowChanges({ disabledChanges: ["changeTitle"] });
      requestRename("10042");
      renderTitle();
      await settle();

      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });
  });

  describe("saving", () => {
    it("sends the trimmed text on Enter and confirms with a toast", async () => {
      renderTitle();
      const box = await startEditing();

      type(box, `  ${NEW}  `);
      fireEvent.keyDown(box, { key: "Enter" });

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "title", text: NEW });
      await waitFor(() => expect(toasts()).toEqual([["success", "Renamed NW-1042"]]));
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("collapses runs of whitespace inside the text", async () => {
      renderTitle();
      const box = await startEditing();

      type(box, "Checkout:\n  validate   DACH\tpostcodes");
      fireEvent.keyDown(box, { key: "Enter" });

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "title", text: "Checkout: validate DACH postcodes" });
    });

    it("saves when the textbox loses focus inside the window", async () => {
      renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.blur(box);

      expect(mocked.issueWrite).toHaveBeenCalledWith("10042", { kind: "title", text: NEW });
    });

    it("keeps editing when the window itself loses focus", async () => {
      hasFocus.mockReturnValue(false);
      renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.blur(box);

      expect(mocked.issueWrite).not.toHaveBeenCalled();
      expect(textbox()).toHaveTextContent(NEW);
    });

    it("sends one write when a blur follows Enter", async () => {
      renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.keyDown(box, { key: "Enter" });
      fireEvent.blur(box);

      await waitFor(() => expect(toasts()).toHaveLength(1));
      expect(mocked.issueWrite).toHaveBeenCalledTimes(1);
    });

    it.each([
      ["unchanged", OLD],
      ["unchanged but for whitespace", `  ${OLD}   `],
      ["empty", ""],
      ["only whitespace", "   \n "],
    ])("sends nothing when the title is %s", async (_label, text) => {
      renderTitle();
      const box = await startEditing();

      type(box, text);
      fireEvent.keyDown(box, { key: "Enter" });

      expect(mocked.issueWrite).not.toHaveBeenCalled();
      expect(heading(OLD)).toBeInTheDocument();
      expect(toasts()).toEqual([]);
    });
  });

  describe("cancelling", () => {
    it("drops the edit on Escape, sends nothing and shows the old title", async () => {
      renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.keyDown(box, { key: "Escape" });

      expect(mocked.issueWrite).not.toHaveBeenCalled();
      expect(heading(OLD)).toBeInTheDocument();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("sends nothing when a blur follows Escape", async () => {
      renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.keyDown(box, { key: "Escape" });
      fireEvent.blur(box);

      expect(mocked.issueWrite).not.toHaveBeenCalled();
    });

    it("starts from the saved title again on the next edit", async () => {
      renderTitle();
      const first = await startEditing();
      type(first, NEW);
      fireEvent.keyDown(first, { key: "Escape" });

      const second = await startEditing();

      expect(second).toHaveTextContent(OLD);
    });
  });

  describe("the title shown after saving", () => {
    it("shows the new title while the issue still has the old summary, and once it has the new one", async () => {
      const { show } = renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.keyDown(box, { key: "Enter" });

      await waitFor(() => expect(toasts()).toHaveLength(1));
      expect(heading(NEW)).toBeInTheDocument();
      show(issueDetail({ summary: NEW }));
      expect(heading(NEW)).toBeInTheDocument();
    });

    it("shows the new title at once, before Jira has answered", async () => {
      mocked.issueWrite.mockReturnValue(new Promise(() => {}));
      renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.keyDown(box, { key: "Enter" });

      expect(heading(NEW)).toBeInTheDocument();
    });

    it("shows the old title again when a later change back restores the old summary", async () => {
      const { show } = renderTitle();
      const box = await startEditing();
      type(box, NEW);
      fireEvent.keyDown(box, { key: "Enter" });
      await waitFor(() => expect(toasts()).toHaveLength(1));
      show(issueDetail({ summary: NEW }));

      show(issueDetail({ summary: OLD }));

      expect(heading(OLD)).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: NEW })).not.toBeInTheDocument();
    });

    it("shows the old title again and an error toast when Jira refuses the write", async () => {
      mocked.issueWrite.mockRejectedValue({ kind: "invalid_input", message: "The title is too long", code: null });
      renderTitle();
      const box = await startEditing();

      type(box, NEW);
      fireEvent.keyDown(box, { key: "Enter" });

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not change the title: The title is too long"]]));
      expect(heading(OLD)).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: NEW })).not.toBeInTheDocument();
    });
  });
});
