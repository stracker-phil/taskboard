import { useCallback, useSyncExternalStore } from "react";
import { bindingFor, bindingLabel, getBindingsVersion, subscribeBindings } from "./registry";

/** The current key of a command (`⌘⇧M`, or "" when it has none) and a tooltip helper that appends it:
 *  `title("Only my issues", "toolbar.me")` → "Only my issues (⌘⇧M)". Both follow Settings → Shortcuts. */
export function useShortcuts() {
  const version = useSyncExternalStore(subscribeBindings, getBindingsVersion);
  // `version` is the dependency: the labels change with it.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const key = useCallback((id: string) => bindingLabel(bindingFor(id)), [version]);
  const title = useCallback(
    (label: string, id: string) => {
      const shortcut = key(id);
      return shortcut ? `${label} (${shortcut})` : label;
    },
    [key],
  );
  return { key, title };
}
