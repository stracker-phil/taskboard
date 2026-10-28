// Re-exports the IPC types generated from Rust (`bun run types`); everything defined below is frontend-only.
import type { IssuesChanged, LocalChanged, Settings, UiState } from "./types.gen";

export type * from "./types.gen";

/** Shortcut overrides by command id; `null` unbinds. Rust: the `Keybindings` alias (a map, so ts-rs can't export it). */
export type Keybindings = Record<string, string | null>;

/** Until Rust's state loads. */
export const EMPTY_UI_STATE: UiState = { sidebarHidden: false };

/** The app as macOS knows it (the bundle's name and version). */
export interface AppInfo {
  name: string;
  version: string;
}

/** Every event Rust emits (`src-tauri/src/events.rs`) with its payload. */
export interface EventPayloads {
  /** Launch requests are waiting; take them with `takeLaunchEvents`. */
  launch: null;
  /** The hidden main window was shown again (the Dock icon brought it back). */
  window_opened: null;
  /** Settings were saved, from any window. */
  settings_changed: Settings;
  /** Shortcut overrides were saved, from any window. */
  keybindings_changed: Keybindings;
  /** To the Settings window: show this page (or section). */
  open_settings: string;
  /** To the main window: show or hide the sidebar. */
  toggle_sidebar: null;
  /** Issues in the cache changed. */
  issues_changed: IssuesChanged;
  /** Parts of the owner's own setup changed. */
  local_changed: LocalChanged;
}

export type EventName = keyof EventPayloads;
