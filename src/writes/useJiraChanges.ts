import type { Capability } from "../api";
import { useSettings } from "../queries/settings";

/** Whether the owner allows changes to Jira at all (Settings → Connection). */
export function useJiraChanges(): boolean {
  const { data: settings } = useSettings();
  return settings?.behaviour.allowJiraChanges ?? false;
}

/** Whether this kind of change is allowed: changes to Jira on, and this one not switched off. */
export function useJiraChange(capability: Capability): boolean {
  const { data: settings } = useSettings();
  const behaviour = settings?.behaviour;
  return (behaviour?.allowJiraChanges ?? false) && !(behaviour?.disabledChanges ?? []).includes(capability);
}
