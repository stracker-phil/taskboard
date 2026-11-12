/**
 * The frontend's install list: the core first, then one line per module. Removing a module is deleting
 * its folder and its line here; the core never imports a module.
 */
import { installCommands, type WindowName } from "./commands";
import { registerCoreSettingsPages } from "./components/settings/pages";

/** Call once per window, before the first render. */
export function installModules(window: WindowName) {
  // Commands first: the Shortcuts page lists every command's row for the Settings search.
  installCommands(window);
  registerCoreSettingsPages();
}
