import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { installCommands } from ".";
import { onIssueKeys, type IssueKeyHandlers } from "./issueKeys";
import { resetStore, useStore } from "../state/store";
import { getCommand, resolveKeydown, setOverrides } from "./registry";

const SCOPES = ["global", "main"];

function press(key: string, mods: { meta?: boolean; shift?: boolean } = {}) {
  return resolveKeydown({ key, metaKey: !!mods.meta, shiftKey: !!mods.shift, ctrlKey: false, altKey: false }, SCOPES);
}

function handlers(patch: Partial<IssueKeyHandlers> = {}) {
  return {
    move: vi.fn(),
    setGroupOpen: vi.fn(),
    closePeek: vi.fn(),
    copyKey: vi.fn(),
    copyUrl: vi.fn(),
    openInJira: vi.fn(),
    open: vi.fn(),
    openEpic: vi.fn(),
    hasSelection: () => true,
    peekOpen: () => true,
    ...patch,
  } satisfies IssueKeyHandlers;
}

let release: (() => void) | null = null;
function register(h: IssueKeyHandlers) {
  release = onIssueKeys(h);
  return release;
}

beforeEach(() => {
  installCommands("main");
  setOverrides({});
  resetStore();
});

afterEach(() => {
  release?.();
  release = null;
  document.body.innerHTML = "";
  window.getSelection()?.removeAllRanges();
});

describe("issue keys", () => {
  describe("without a view that handles them", () => {
    it.each([
      ["J", "j", {}],
      ["K", "k", {}],
      ["↓", "ArrowDown", {}],
      ["↑", "ArrowUp", {}],
      ["←", "ArrowLeft", {}],
      ["→", "ArrowRight", {}],
      ["⎋", "Escape", {}],
      ["⌘C", "c", { meta: true }],
      ["⌘⇧C", "c", { meta: true, shift: true }],
      ["⌘↵", "Enter", { meta: true }],
    ])("leaves %s to nobody", (_label, key, mods) => {
      expect(press(key, mods)).toBeNull();
    });
  });

  describe("moving", () => {
    it.each([
      ["J", "j", 1],
      ["↓", "ArrowDown", 1],
      ["K", "k", -1],
      ["↑", "ArrowUp", -1],
    ])("%s moves the selection by %i", (_label, key, step) => {
      const h = handlers({ hasSelection: () => false });
      register(h);

      void press(key)?.run();

      expect(h.move).toHaveBeenCalledWith(step);
    });
  });

  describe("group keys", () => {
    it.each([
      ["←", "ArrowLeft", false],
      ["→", "ArrowRight", true],
    ])("%s sets the selected row's group open to %s", (_label, key, open) => {
      const h = handlers();
      register(h);

      void press(key)?.run();

      expect(h.setGroupOpen).toHaveBeenCalledWith(open);
    });

    it.each(["ArrowLeft", "ArrowRight"])("ignores %s without a selected row", (key) => {
      register(handlers({ hasSelection: () => false }));

      expect(press(key)).toBeNull();
    });
  });

  describe("Escape", () => {
    it("closes the peek while it is open", () => {
      const h = handlers();
      register(h);

      void press("Escape")?.run();

      expect(h.closePeek).toHaveBeenCalled();
    });

    it("does nothing while the peek is closed", () => {
      register(handlers({ peekOpen: () => false }));

      expect(press("Escape")).toBeNull();
    });

    it("leaves Escape to an open dialog", () => {
      register(handlers());
      document.body.innerHTML = '<div role="dialog">Confirm</div>';

      expect(press("Escape")).toBeNull();
    });
  });

  describe("opening", () => {
    it("↵ opens the selected issue", () => {
      const h = handlers();
      register(h);

      void press("Enter")?.run();

      expect(h.open).toHaveBeenCalled();
    });

    it("E opens the selected issue's epic", () => {
      const h = handlers();
      register(h);

      void press("e")?.run();

      expect(h.openEpic).toHaveBeenCalled();
    });

    it.each([
      ["↵", "Enter"],
      ["E", "e"],
    ])("ignore %s without a selected row", (_label, key) => {
      register(handlers({ hasSelection: () => false }));

      expect(press(key)).toBeNull();
    });
  });

  describe("Escape in the Issue view", () => {
    it("goes back while the Issue view is open", () => {
      register(handlers({ peekOpen: () => false }));
      useStore.getState().openIssue("10042", { listId: 1, viewId: 1 });

      expect(press("Escape")?.id).toBe("issue.back");
    });

    it("does nothing while the Issue view is closed", () => {
      expect(press("Escape")).toBeNull();
    });

    it("leaves Escape to an open dialog", () => {
      useStore.getState().openIssue("10042", { listId: 1, viewId: 1 });
      document.body.innerHTML = '<div role="dialog">Confirm</div>';

      expect(press("Escape")).toBeNull();
    });

    it("closes the peek, not the Issue view, while only the peek is open", () => {
      register(handlers());

      expect(press("Escape")?.id).toBe("peek.close");
    });
  });

  describe("copy key (⌘C)", () => {
    it("copies the key of the selected row", () => {
      const h = handlers();
      register(h);

      void press("c", { meta: true })?.run();

      expect(h.copyKey).toHaveBeenCalled();
    });

    it("does nothing without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("c", { meta: true })).toBeNull();
    });

    it("leaves ⌘C to a focused text field", () => {
      register(handlers());
      const input = document.createElement("input");
      document.body.append(input);
      input.focus();

      expect(press("c", { meta: true })).toBeNull();
    });

    it("leaves ⌘C to selected text", () => {
      register(handlers());
      const text = document.createElement("p");
      text.textContent = "Validate German postcodes.";
      document.body.append(text);
      window.getSelection()?.selectAllChildren(text);

      expect(press("c", { meta: true })).toBeNull();
    });
  });

  describe("copy URL and open in Jira", () => {
    it("⌘⇧C copies the selected issue's link", () => {
      const h = handlers();
      register(h);

      void press("c", { meta: true, shift: true })?.run();

      expect(h.copyUrl).toHaveBeenCalled();
    });

    it("⌘↵ opens the selected issue in Jira", () => {
      const h = handlers();
      register(h);

      void press("Enter", { meta: true })?.run();

      expect(h.openInJira).toHaveBeenCalled();
    });

    it.each([
      ["⌘⇧C", "c", { meta: true, shift: true }],
      ["⌘↵", "Enter", { meta: true }],
    ])("ignore %s without a selected row", (_label, key, mods) => {
      register(handlers({ hasSelection: () => false }));

      expect(press(key, mods)).toBeNull();
    });
  });

  describe("releasing", () => {
    it("disables the commands again", () => {
      const off = register(handlers());
      expect(press("j")).not.toBeNull();

      off();

      expect(press("j")).toBeNull();
      expect(getCommand("issue.openInJira")?.when?.()).toBe(false);
    });

    it("keeps a newer registration when an older one is released late", () => {
      const older = handlers();
      const newer = handlers();
      const releaseOlder = onIssueKeys(older);
      register(newer);

      releaseOlder();
      void press("j")?.run();

      expect(newer.move).toHaveBeenCalledWith(1);
      expect(older.move).not.toHaveBeenCalled();
    });
  });
});
