import { beforeEach, describe, expect, it } from "vitest";
import { acceleratorLabel, clearCommands, conflicts, registerCommands, setOverrides, type Command } from "./registry";
import { registerCoreScopes } from "./index";
import { registerScope } from "./scopes";

const command = (id: string, scope: string, defaultBinding?: string): Command => ({ id, title: id, scope, defaultBinding, run: () => {} });

beforeEach(() => {
  clearCommands();
  registerCoreScopes();
  registerScope({ id: "test.left", parent: "main", label: "Left", isActive: () => true });
  registerScope({ id: "test.right", parent: "main", label: "Right", isActive: () => true });
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

describe("acceleratorLabel", () => {
  it.each([
    ["CmdOrCtrl+Shift+R", "⌘⇧R"],
    ["CmdOrCtrl+K", "⌘K"],
    ["CmdOrCtrl+Alt+Up", "⌘⌥↑"],
    ["CmdOrCtrl+Down", "⌘↓"],
    ["CmdOrCtrl+Backslash", "⌘\\"],
    ["CmdOrCtrl+Enter", "⌘↵"],
  ])("writes %s as %s", (accelerator, label) => {
    expect(acceleratorLabel(accelerator)).toBe(label);
  });

  it.each([null, ""])("is empty for %j", (accelerator) => {
    expect(acceleratorLabel(accelerator)).toBe("");
  });
});
