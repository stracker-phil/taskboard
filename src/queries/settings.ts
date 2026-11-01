import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Query keys of the Settings pages; `settings_changed` refreshes all but the projects. */
export const SETTINGS_KEYS = {
  settings: ["settings"],
  connection: ["connection"],
  projects: ["projects"],
  scope: ["scope"],
  keybindings: ["keybindings"],
  capabilities: ["capabilities"],
  changePermissions: ["changePermissions"],
} as const;

export const useSettings = () => useQuery({ queryKey: SETTINGS_KEYS.settings, queryFn: api.getSettings });
export const useConnection = () => useQuery({ queryKey: SETTINGS_KEYS.connection, queryFn: api.connectionGet });
export const useProjects = () => useQuery({ queryKey: SETTINGS_KEYS.projects, queryFn: api.projectsList });
export const useScope = () => useQuery({ queryKey: SETTINGS_KEYS.scope, queryFn: api.scopeGet });
export const useKeybindings = () => useQuery({ queryKey: SETTINGS_KEYS.keybindings, queryFn: api.getKeybindings });
export const useCapabilities = () => useQuery({ queryKey: SETTINGS_KEYS.capabilities, queryFn: api.jiraCapabilities });
/** The changes Jira doesn't allow in some tracked projects; asked only while changes are on. */
export const useChangePermissions = (enabled: boolean) =>
  useQuery({ queryKey: SETTINGS_KEYS.changePermissions, queryFn: api.connectionChangePermissions, enabled, retry: false });
