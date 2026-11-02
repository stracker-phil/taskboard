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
    copyAppLink: vi.fn(),
    openInJira: vi.fn(),
    open: vi.fn(),
    openEpic: vi.fn(),
    togglePin: vi.fn(),
    toggleRead: vi.fn(),
    toggleMine: vi.fn(),
    changeStatus: vi.fn(),
    assignTo: vi.fn(),
    changePriority: vi.fn(),
    rename: vi.fn(),
    togglePeek: vi.fn(),
    extend: vi.fn(),
    pickAll: vi.fn(),
    clearPicks: vi.fn(),
    hasPicks: () => false,
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
      ["P", "p", {}],
      ["I", "i", {}],
      ["D", "d", {}],
      ["A", "a", {}],
      ["⇧P", "P", { shift: true }],
      ["⇧J", "J", { shift: true }],
      ["⇧K", "K", { shift: true }],
      ["⌘A", "a", { meta: true }],
      ["F2", "F2", {}],
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

  describe("picking", () => {
    it.each([
      ["⇧J", "J", 1],
      ["⇧↓", "ArrowDown", 1],
      ["⇧K", "K", -1],
      ["⇧↑", "ArrowUp", -1],
    ])("%s extends the picks by %i", (_label, key, step) => {
      const h = handlers();
      register(h);

      void press(key, { shift: true })?.run();

      expect(h.extend).toHaveBeenCalledWith(step);
      expect(h.move).not.toHaveBeenCalled();
    });

    it("leaves ⇧J to nobody where the view cannot pick", () => {
      register(handlers({ extend: undefined }));

      expect(press("J", { shift: true })).toBeNull();
    });

    it("⌘A picks every row", () => {
      const h = handlers();
      register(h);

      void press("a", { meta: true })?.run();

      expect(h.pickAll).toHaveBeenCalled();
    });

    it("leaves ⌘A to the native Select All where the view cannot pick", () => {
      register(handlers({ pickAll: undefined }));

      expect(press("a", { meta: true })).toBeNull();
    });

    it("leaves ⌘A to a focused text field", () => {
      register(handlers());
      const input = document.createElement("input");
      document.body.append(input);
      input.focus();

      expect(press("a", { meta: true })).toBeNull();
    });

    it("leaves ⌘A to an open dialog", () => {
      register(handlers());
      document.body.innerHTML = '<div role="dialog">Confirm</div>';

      expect(press("a", { meta: true })).toBeNull();
    });

    describe("Escape", () => {
      it("drops the picks while rows are picked, even with the peek open", () => {
        const h = handlers({ hasPicks: () => true, peekOpen: () => true });
        register(h);

        const command = press("Escape");
        void command?.run();

        expect(command?.id).toBe("peek.close");
        expect(h.clearPicks).toHaveBeenCalled();
        expect(h.closePeek).not.toHaveBeenCalled();
      });

      it("drops the picks where there is no preview", () => {
        const h = handlers({ hasPicks: () => true, peekOpen: () => false });
        register(h);

        const command = press("Escape");
        void command?.run();

        expect(command?.id).toBe("peek.close");
        expect(h.clearPicks).toHaveBeenCalled();
      });

      it("closes the peek when nothing is picked", () => {
        const h = handlers({ hasPicks: () => false, peekOpen: () => true });
        register(h);

        const command = press("Escape");
        void command?.run();

        expect(command?.id).toBe("peek.close");
        expect(h.closePeek).toHaveBeenCalled();
        expect(h.clearPicks).not.toHaveBeenCalled();
      });

      it("does nothing when nothing is picked and the peek is closed", () => {
        register(handlers({ hasPicks: () => false, peekOpen: () => false }));

        expect(press("Escape")).toBeNull();
      });

      it("is left to a focused text field", () => {
        register(handlers({ hasPicks: () => true }));
        const input = document.createElement("input");
        document.body.append(input);
        input.focus();

        expect(press("Escape")).toBeNull();
      });

      it("is left to an open dialog", () => {
        register(handlers({ hasPicks: () => true }));
        document.body.innerHTML = '<div role="dialog">Confirm</div>';

        expect(press("Escape")).toBeNull();
      });
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

    it("leaves Escape to a focused text field, such as the title being renamed", () => {
      register(handlers());
      const input = document.createElement("input");
      document.body.append(input);
      input.focus();

      expect(press("Escape")).toBeNull();
    });

    it("closes the peek again once the text field has lost focus", () => {
      register(handlers());
      const input = document.createElement("input");
      document.body.append(input);
      input.focus();
      input.blur();

      expect(press("Escape")?.id).toBe("peek.close");
    });
  });

  describe("renaming", () => {
    it("F2 renames the selected issue", () => {
      const h = handlers();
      register(h);

      void press("F2")?.run();

      expect(h.rename).toHaveBeenCalled();
    });

    it("ignores F2 without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("F2")).toBeNull();
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

  describe("bookmarking", () => {
    it("P bookmarks the selected issue or removes its bookmark", () => {
      const h = handlers();
      register(h);

      void press("p")?.run();

      expect(h.togglePin).toHaveBeenCalled();
    });

    it("ignores P without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("p")).toBeNull();
    });
  });

  describe("assigning to me", () => {
    it("I assigns the selected issue to me or unassigns it", () => {
      const h = handlers();
      register(h);

      void press("i")?.run();

      expect(h.toggleMine).toHaveBeenCalled();
    });

    it("ignores I without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("i")).toBeNull();
    });
  });

  describe("changing the status", () => {
    it("D changes the selected issue's status", () => {
      const h = handlers();
      register(h);

      void press("d")?.run();

      expect(h.changeStatus).toHaveBeenCalled();
    });

    it("ignores D without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("d")).toBeNull();
    });

    it("no longer answers to S", () => {
      register(handlers());

      expect(press("s")).toBeNull();
    });
  });

  describe("assigning to someone", () => {
    it("A opens the selected issue's assignee menu", () => {
      const h = handlers();
      register(h);

      void press("a")?.run();

      expect(h.assignTo).toHaveBeenCalled();
    });

    it("ignores A without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("a")).toBeNull();
    });
  });

  describe("changing the priority", () => {
    it("⇧P opens the selected issue's priority menu", () => {
      const h = handlers();
      register(h);

      void press("P", { shift: true })?.run();

      expect(h.changePriority).toHaveBeenCalled();
    });

    it("ignores ⇧P without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("P", { shift: true })).toBeNull();
    });

    it("leaves a plain P to bookmarking", () => {
      const h = handlers();
      register(h);

      void press("p")?.run();

      expect(h.togglePin).toHaveBeenCalled();
      expect(h.changePriority).not.toHaveBeenCalled();
    });
  });

  describe("marking read or unread", () => {
    it("M toggles the selected issue between read and unread", () => {
      const h = handlers();
      register(h);

      void press("m")?.run();

      expect(h.toggleRead).toHaveBeenCalled();
    });

    it("titles M's command as marking the issue read or unread", () => {
      expect(getCommand("issue.markSeen")?.title).toBe("Mark the issue as read or unread");
    });

    it("ignores M without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press("m")).toBeNull();
    });

    it("leaves M alone without a view that handles the issue keys", () => {
      expect(press("m")).toBeNull();
    });
  });

  describe("peek (Space)", () => {
    it("toggles the selected issue's peek", () => {
      const h = handlers();
      register(h);

      void press(" ")?.run();

      expect(h.togglePeek).toHaveBeenCalled();
    });

    it("ignores Space without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(press(" ")).toBeNull();
    });

    it("leaves Space alone without a view that handles the issue keys", () => {
      expect(press(" ")).toBeNull();
    });
  });

  describe("a view without open or peek handlers", () => {
    it.each([
      ["↩", "Enter"],
      ["Space", " "],
    ])("leaves %s unclaimed even with a selection", (_label, key) => {
      register(handlers({ open: undefined, togglePeek: undefined }));

      expect(press(key)).toBeNull();
    });

    it("still claims the other issue keys", () => {
      register(handlers({ open: undefined, togglePeek: undefined }));

      expect(press("m")?.id).toBe("issue.markSeen");
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

    it("leaves Escape to a focused text field, such as the find", () => {
      register(handlers({ peekOpen: () => false }));
      useStore.getState().openIssue("10042", { listId: 1, viewId: 1 });
      const input = document.createElement("input");
      document.body.append(input);
      input.focus();

      expect(press("Escape")).toBeNull();
    });

    it("goes back again once the text field has lost focus", () => {
      register(handlers({ peekOpen: () => false }));
      useStore.getState().openIssue("10042", { listId: 1, viewId: 1 });
      const input = document.createElement("input");
      document.body.append(input);
      input.focus();
      input.blur();

      expect(press("Escape")?.id).toBe("issue.back");
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

    it.each(["textarea", "input"])("⌘↵ leaves the key to a focused %s", (tag) => {
      register(handlers());
      const field = document.createElement(tag);
      document.body.append(field);
      field.focus();

      expect(press("Enter", { meta: true })).toBeNull();
    });

    it("⌘↵ opens Jira again once the text field has lost focus", () => {
      const h = handlers();
      register(h);
      const field = document.createElement("textarea");
      document.body.append(field);
      field.focus();
      field.blur();

      void press("Enter", { meta: true })?.run();

      expect(h.openInJira).toHaveBeenCalled();
    });

    it.each([
      ["⌘⇧C", "c", { meta: true, shift: true }],
      ["⌘↵", "Enter", { meta: true }],
    ])("ignore %s without a selected row",(_label, key, mods) => {
      register(handlers({ hasSelection: () => false }));

      expect(press(key, mods)).toBeNull();
    });
  });

  describe("copy Taskboard link (⌘⌥⇧C)", () => {
    it("copies the Taskboard link of the selected issue", () => {
      const h = handlers();
      register(h);

      void resolveKeydown({ key: "c", metaKey: true, shiftKey: true, ctrlKey: false, altKey: true }, SCOPES)?.run();

      expect(h.copyAppLink).toHaveBeenCalled();
    });

    it("is disabled without a selected row", () => {
      register(handlers({ hasSelection: () => false }));

      expect(getCommand("issue.copyAppLink")?.when?.()).toBe(false);
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
