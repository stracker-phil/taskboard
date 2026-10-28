import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Query keys of the Settings pages; `settings_changed` refreshes all but the projects. */
export const SETTINGS_KEYS = {
  settings: ["settings"],
  connection: ["connection"],
  projects: ["projects"],
  scope: ["scope"],
  keybindings: ["keybindings"],
} as const;

export const useSettings = () => useQuery({ queryKey: SETTINGS_KEYS.settings, queryFn: api.getSettings });
export const useConnection = () => useQuery({ queryKey: SETTINGS_KEYS.connection, queryFn: api.connectionGet });
export const useProjects = () => useQuery({ queryKey: SETTINGS_KEYS.projects, queryFn: api.projectsList });
export const useScope = () => useQuery({ queryKey: SETTINGS_KEYS.scope, queryFn: api.scopeGet });
export const useKeybindings = () => useQuery({ queryKey: SETTINGS_KEYS.keybindings, queryFn: api.getKeybindings });
