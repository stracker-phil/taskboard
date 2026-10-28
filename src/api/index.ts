import { getName, getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppError, AppInfo, ErrorKind, EventName, EventPayloads, Keybindings, LaunchEvent, Settings, UiState } from "./types";

export * from "./types";

/** The app's IPC entry point (ADR 0002). Tests mock this module. */
export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  /** Returns the settings as Rust saved them. */
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  getKeybindings: () => invoke<Keybindings>("get_keybindings"),
  setKeybindings: (bindings: Keybindings) => invoke<Keybindings>("set_keybindings", { bindings }),
  getUiState: () => invoke<UiState>("get_ui_state"),
  setUiState: (ui: UiState) => invoke<UiState>("set_ui_state", { ui }),
  openLogFolder: () => invoke<void>("open_log_folder"),
  /** https links only; Rust refuses anything else. */
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  logError: (message: string) => invoke<void>("log_frontend_error", { message }),
  takeLaunchEvents: () => invoke<LaunchEvent[]>("take_launch_events"),
  /** Runs a command of the Rust command bus by id (`app:settings`, `view:toggleSidebar`, …); rejects when it is unknown, disabled or fails. */
  runCommand: (id: string) => invoke<void>("run_command", { id }),
  appInfo: async (): Promise<AppInfo> => {
    const [name, version] = await Promise.all([getName(), getVersion()]);
    return { name, version };
  },
  listen: <E extends EventName>(event: E, handler: (payload: EventPayloads[E]) => void): Promise<UnlistenFn> =>
    listen<EventPayloads[E]>(event, (e) => handler(e.payload)),
};

/** Every `ErrorKind` Rust sends; the `Record` makes the compiler check the list is complete. */
const ERROR_KINDS: Record<ErrorKind, true> = {
  io: true,
  invalid_input: true,
  not_found: true,
  not_connected: true,
  internal: true,
};

export function toAppError(e: unknown): AppError {
  if (!(e !== null && typeof e === "object" && "message" in e)) return { kind: "io", message: String(e) };
  if ("kind" in e && Object.hasOwn(ERROR_KINDS, String(e.kind))) return e as AppError;
  return { kind: "io", message: String(e.message) };
}
