import { useSettings } from "../queries/settings";

/** Whether the owner allows changes to Jira (Settings → Connection). */
export function useJiraChanges(): boolean {
  const { data: settings } = useSettings();
  return settings?.behaviour.allowJiraChanges ?? false;
}
