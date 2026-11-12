import { beforeEach, describe, expect, it } from "vitest";
import { acceleratorLabel, bindingFromEvent, clearCommands, conflicts, holdersOf, registerCommands, setOverrides, withBinding, type Command } from "./registry";
import { registerCoreScopes } from "./index";
import { registerScope } from "./scopes";

const command = (id: string, scope: string, defaultBinding?: string): Command => ({ id, title: id, scope, defaultBinding, run: () => {} });

beforeEach(() => {
  clearCommands();
  registerCoreScopes();
  registerScope({ id: "test.left", parent: "main", label: "Left", isActive: () => true });
  registerScope({ id: "test.right", parent: "main", label: "Right", isActive: () => true });
  registerScope({ id: "test.overlay", parent: "main", label: "Overlay", isActive: () => true, shadows: true });
  registerScope({ id: "test.overlay.inner", parent: "test.overlay", label: "Overlay inner", isActive: () => true });
});

describe("conflicts", () => {
  it("reports two commands with the same binding in the same scope", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main", "mod+k")]);

    const found = conflicts();

    expect(found.map((c) => [c.binding, c.a.id, c.b.id])).toEqual([["Mod+K", "a", "b"]]);
  });

  it("reports a command in an ancestor scope against one in a descendant scope", () => {
    registerCommands([command("a", "global", "Mod+K"), command("b", "test.left", "Mod+K")]);

    expect(conflicts()).toHaveLength(1);
  });

  it("does not report the same binding in sibling scopes", () => {
    registerCommands([command("a", "test.left", "Mod+K"), command("b", "test.right", "Mod+K")]);

    expect(conflicts()).toEqual([]);
  });

  describe("with a child scope that shadows its parent", () => {
    it("does not report a key shared with the parent scope", () => {
      registerCommands([command("a", "main", "Escape"), command("b", "test.overlay", "Escape")]);

      expect(conflicts()).toEqual([]);
    });

    it("does not report a key shared with an ancestor above the parent", () => {
      registerCommands([command("a", "global", "Escape"), command("b", "test.overlay", "Escape")]);

      expect(conflicts()).toEqual([]);
    });

    it("does not report a key shared with the parent scope by a scope below the shadowing one", () => {
      registerCommands([command("a", "main", "Escape"), command("b", "test.overlay.inner", "Escape")]);

      expect(conflicts()).toEqual([]);
    });

    it("still reports a key shared inside the shadowing scope", () => {
      registerCommands([command("a", "test.overlay", "Escape"), command("b", "test.overlay", "Escape")]);

      expect(conflicts().map((c) => [c.binding, c.a.id, c.b.id])).toEqual([["Escape", "a", "b"]]);
    });

    it("still reports the same key in a child scope that does not shadow", () => {
      registerCommands([command("a", "main", "Escape"), command("b", "test.left", "Escape")]);

      expect(conflicts().map((c) => [c.binding, c.a.id, c.b.id])).toEqual([["Escape", "a", "b"]]);
    });
  });

  it("does not report different bindings", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main", "Mod+J")]);

    expect(conflicts()).toEqual([]);
  });

  it("counts the overrides passed in", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main", "Mod+J")]);

    const found = conflicts({ b: "Mod+K" });

    expect(found.map((c) => [c.binding, c.a.id, c.b.id])).toEqual([["Mod+K", "a", "b"]]);
  });

  it("drops a conflict when an override unbinds one command", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main", "Mod+K")]);

    expect(conflicts({ b: null })).toEqual([]);
  });

  it("uses the stored overrides when none are passed", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main", "Mod+J")]);
    setOverrides({ b: "Mod+K" });

    expect(conflicts()).toHaveLength(1);
  });
});

describe("holdersOf", () => {
  it("lists the other commands bound to the key in the same scope", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main", "Mod+K"), command("c", "main", "Mod+J")]);

    expect(holdersOf(command("a", "main"), "Mod+K").map((c) => c.id)).toEqual(["b"]);
  });

  it("never lists the command itself", () => {
    registerCommands([command("a", "main", "Mod+K")]);

    expect(holdersOf(command("a", "main"), "Mod+K")).toEqual([]);
  });

  it("matches a differently spelled binding", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main")]);

    expect(holdersOf(command("b", "main"), "k+mod").map((c) => c.id)).toEqual(["a"]);
  });

  it("lists a holder in an ancestor or descendant scope", () => {
    registerCommands([command("a", "global", "Mod+K"), command("b", "test.left", "Mod+K"), command("c", "main")]);

    expect(holdersOf(command("c", "main"), "Mod+K").map((c) => c.id)).toEqual(["a", "b"]);
  });

  it("skips a holder in a sibling scope", () => {
    registerCommands([command("a", "test.left", "Mod+K"), command("b", "test.right")]);

    expect(holdersOf(command("b", "test.right"), "Mod+K")).toEqual([]);
  });

  it("skips a holder in the scope a shadowing scope covers", () => {
    registerCommands([command("a", "main", "Escape"), command("b", "test.overlay")]);

    expect(holdersOf(command("b", "test.overlay"), "Escape")).toEqual([]);
  });

  it("reads the overrides passed in, so a moved or removed key holds nothing", () => {
    registerCommands([command("a", "main", "Mod+K"), command("b", "main", "Mod+J"), command("c", "main")]);
    const c = command("c", "main");

    expect(holdersOf(c, "Mod+K", { a: null })).toEqual([]);
    expect(holdersOf(c, "Mod+K", { a: "Mod+L" })).toEqual([]);
    expect(holdersOf(c, "Mod+K", { b: "Mod+K" }).map((x) => x.id)).toEqual(["a", "b"]);
  });
});

describe("withBinding", () => {
  const a = command("a", "main", "Mod+K");
  const b = command("b", "main", "Mod+J");

  it("sets the command's override and keeps the others", () => {
    expect(withBinding({ z: "Mod+Z" }, a, "Mod+L")).toEqual({ z: "Mod+Z", a: "Mod+L" });
  });

  it("normalizes the binding", () => {
    expect(withBinding({}, a, "l+shift+mod")).toEqual({ a: "Mod+Shift+L" });
  });

  it("stores null for no key", () => {
    expect(withBinding({}, a, null)).toEqual({ a: null });
  });

  it("stores null for no key on a command with no default", () => {
    expect(withBinding({ a: "Mod+L" }, command("a", "main"), null)).toEqual({});
  });

  it("leaves each from command without a key in the same object", () => {
    expect(withBinding({}, a, "Mod+J", [b])).toEqual({ a: "Mod+J", b: null });
  });

  it("drops the override when the binding equals the default", () => {
    expect(withBinding({ a: "Mod+L", z: "Mod+Z" }, a, "mod+k")).toEqual({ z: "Mod+Z" });
  });

  it("still unbinds the from commands when the binding is the default", () => {
    expect(withBinding({ a: "Mod+L" }, a, "Mod+K", [b])).toEqual({ b: null });
  });

  it("does not change the object passed in", () => {
    const custom = { a: "Mod+L" };

    withBinding(custom, a, null, [b]);

    expect(custom).toEqual({ a: "Mod+L" });
  });
});

describe("bindingFromEvent", () => {
  const event = (patch: Partial<Parameters<typeof bindingFromEvent>[0]>) => ({ key: "", metaKey: false, shiftKey: false, ctrlKey: false, altKey: false, ...patch });

  it.each([
    ["⌘⇧1 on a US layout", { key: "!", code: "Digit1", metaKey: true, shiftKey: true }, "Mod+Shift+1"],
    ["⌘⇧2 on a US layout", { key: "@", code: "Digit2", metaKey: true, shiftKey: true }, "Mod+Shift+2"],
    ["⌘⇧9 on a German layout", { key: ")", code: "Digit9", metaKey: true, shiftKey: true }, "Mod+Shift+9"],
  ])("names %s after the digit key", (_name, patch, binding) => {
    expect(bindingFromEvent(event(patch))).toBe(binding);
  });

  it("names ⌃⇧1 after the digit key off the Mac", () => {
    expect(bindingFromEvent(event({ key: "!", code: "Digit1", ctrlKey: true, shiftKey: true }), false)).toBe("Mod+Shift+1");
  });

  it("keeps the typed symbol for ⇧ and a digit without ⌘ or ⌃", () => {
    expect(bindingFromEvent(event({ key: "!", code: "Digit1", shiftKey: true }))).toBe("Shift+!");
  });
});

describe("acceleratorLabel", () => {
  it.each([
    ["CmdOrCtrl+Shift+R", "⌘⇧R"],
    ["CmdOrCtrl+K", "⌘K"],
    ["CmdOrCtrl+Alt+Up", "⌘⌥↑"],
    ["CmdOrCtrl+Down", "⌘↓"],
    ["CmdOrCtrl+Backslash", "⌘\\"],
    ["CmdOrCtrl+Enter", "⌘↵"],
    ["CmdOrCtrl+Alt+BracketLeft", "⌘⌥["],
    ["CmdOrCtrl+Alt+BracketRight", "⌘⌥]"],
  ])("writes %s as %s", (accelerator, label) => {
    expect(acceleratorLabel(accelerator)).toBe(label);
  });

  it.each([null, ""])("is empty for %j", (accelerator) => {
    expect(acceleratorLabel(accelerator)).toBe("");
  });
});
