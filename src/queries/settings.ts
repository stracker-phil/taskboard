import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Query keys of the Settings pages; `settings_changed` refreshes all but the projects. */
export const SETTINGS_KEYS = {
  settings: ["settings"],
  connection: ["connection"],
  projects: ["projects"],
  scope: ["scope"],
  issueTypes: ["issueTypes"],
  notificationIssueTypes: ["notificationIssueTypes"],
  peopleFields: ["peopleFields"],
  keybindings: ["keybindings"],
  capabilities: ["capabilities"],
  changePermissions: ["changePermissions"],
} as const;

export const useSettings = () => useQuery({ queryKey: SETTINGS_KEYS.settings, queryFn: api.getSettings });
export const useConnection = () => useQuery({ queryKey: SETTINGS_KEYS.connection, queryFn: api.connectionGet });
export const useProjects = () => useQuery({ queryKey: SETTINGS_KEYS.projects, queryFn: api.projectsList });
export const useScope = () => useQuery({ queryKey: SETTINGS_KEYS.scope, queryFn: api.scopeGet });
/** The issue types Settings → Sync can leave out. */
export const useIssueTypes = () => useQuery({ queryKey: SETTINGS_KEYS.issueTypes, queryFn: api.syncIssueTypes });
// Asked again whenever the page shows: a sync brings new types, Settings → Custom fields opts in people fields.
/** The issue types Settings → Notifications can leave out. */
export const useNotificationIssueTypes = () =>
  useQuery({ queryKey: SETTINGS_KEYS.notificationIssueTypes, queryFn: api.notificationIssueTypes, refetchOnMount: "always" });
export const usePeopleFields = () => useQuery({ queryKey: SETTINGS_KEYS.peopleFields, queryFn: api.notificationPeopleFields, refetchOnMount: "always" });
export const useKeybindings = () => useQuery({ queryKey: SETTINGS_KEYS.keybindings, queryFn: api.getKeybindings });
export const useCapabilities = () => useQuery({ queryKey: SETTINGS_KEYS.capabilities, queryFn: api.jiraCapabilities });
/** The changes Jira doesn't allow in some tracked projects; asked only while changes are on. */
export const useChangePermissions = (enabled: boolean) =>
  useQuery({ queryKey: SETTINGS_KEYS.changePermissions, queryFn: api.connectionChangePermissions, enabled, retry: false });
