import { useEffect } from "react";
import { api } from "../api";
import { runBusCommand } from "../state/requests";
import { useStore } from "../state/store";
import { isRecordingTarget, registerCommands, resolveKeydown, setOverrides, type Command } from "./registry";
import { activeScopes, registerScope } from "./scopes";
import { ISSUE_KEY_COMMANDS } from "./issueKeys";
import { VIEW_KEY_COMMANDS } from "./viewKeys";
import { PAGE_ACTION_COMMANDS } from "./pageActions";

/** The window this page runs in: the Taskboard window (`main`) or Settings. */
export type WindowName = "main" | "settings";

let currentWindow: WindowName = "main";

/** Registers the core scopes; safe to call again (tests and re-installs do). */
export function registerCoreScopes() {
  registerScope({ id: "global", parent: null, label: "Every window", isActive: () => true });
  // Settings lists the Taskboard window's keys without running them.
  registerScope({ id: "main", parent: "global", label: "Taskboard window", isActive: () => currentWindow === "main" });
}

/** The windows' own keys. Settings (⌘,), Quit and Close Window are Rust commands with app menu keys only. The
 *  sidebar key is the menu item's too; the page claims it so it can be rebound, and runs the same bus command. */
const APP_COMMANDS: Command[] = [
  { id: "sidebar.toggle", title: "Toggle sidebar", scope: "main", defaultBinding: "Mod+\\", run: () => runBusCommand("view:toggleSidebar", "Could not toggle the sidebar") },
  { id: "space.previous", title: "Previous Space", scope: "main", defaultBinding: "Mod+Alt+ArrowUp", run: () => runBusCommand("nav:previousSpace", "Could not open the Space") },
  { id: "palette.open", title: "Command palette", scope: "main", defaultBinding: "Mod+K", run: () => useStore.getState().setPaletteOpen(true) },
  { id: "space.next", title: "Next Space", scope: "main", defaultBinding: "Mod+Alt+ArrowDown", run: () => runBusCommand("nav:nextSpace", "Could not open the Space") },
  ...VIEW_KEY_COMMANDS,
  ...PAGE_ACTION_COMMANDS,
  ...ISSUE_KEY_COMMANDS,
];

/** Every window registers every command, so Settings can list them; a scope keeps a key to its window. */
export function installCommands(windowName: WindowName) {
  currentWindow = windowName;
  registerCoreScopes();
  registerCommands(APP_COMMANDS);
}

/** A context menu takes its own keys. */
const inMenu = (target: EventTarget | null) => target instanceof Element && !!target.closest('[role="menu"]');

/** ↵ and Space activate the focused button or link rather than a command. */
function activatesControl(e: KeyboardEvent): boolean {
  if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey || (e.key !== "Enter" && e.key !== " ")) return false;
  return e.target instanceof Element && !!e.target.closest('button, a[href], [role="button"], [role="option"]');
}

/** Capture-phase keydown handler for the window. */
function handleKeydown(e: KeyboardEvent) {
  if (isRecordingTarget(e.target) || activatesControl(e) || inMenu(e.target)) return;
  const c = resolveKeydown(e, activeScopes());
  if (!c) return;
  e.preventDefault();
  e.stopPropagation();
  // A held key is swallowed rather than passed on, so the native menu key equivalent does not repeat it either.
  if (e.repeat) return;
  void c.run();
}

/** Routes the window's keys to the commands while mounted, with the user's overrides from Rust (`get_keybindings`,
 *  then `keybindings_changed`). */
export function useCommandKeys() {
  useEffect(() => {
    void api.getKeybindings().then(setOverrides, () => {});
    const unlisten = api.listen("keybindings_changed", setOverrides);
    window.addEventListener("keydown", handleKeydown, true);
    return () => {
      void unlisten.then((fn) => fn()).catch(() => {});
      window.removeEventListener("keydown", handleKeydown, true);
    };
  }, []);
}
