import { vi } from "vitest";
import type { api as realApi } from "../api";
// Must not import the store or "../api": they would load the real api module while it is being mocked.
import { EMPTY_UI_STATE, type EventName, type EventPayloads, type Keybindings, type Settings, type UiState } from "../api/types";

type Api = typeof realApi;
type Handler = (payload: never) => void;

const listeners = new WeakMap<object, Map<string, Set<Handler>>>();

/** An `api` where every call resolves to a valid default; override per test with mockResolvedValue. */
export function createApiMock(): Api {
  const byEvent = new Map<string, Set<Handler>>();
  const mock: Api = {
    getSettings: vi.fn().mockResolvedValue({}),
    updateSettings: vi.fn(async (s: Settings) => s),
    getKeybindings: vi.fn().mockResolvedValue({}),
    setKeybindings: vi.fn(async (bindings: Keybindings) => bindings),
    getUiState: vi.fn().mockResolvedValue(EMPTY_UI_STATE),
    setUiState: vi.fn(async (ui: UiState) => ui),
    openLogFolder: vi.fn().mockResolvedValue(undefined),
    openUrl: vi.fn().mockResolvedValue(undefined),
    logError: vi.fn().mockResolvedValue(undefined),
    takeLaunchEvents: vi.fn().mockResolvedValue([]),
    runCommand: vi.fn().mockResolvedValue(undefined),
    appInfo: vi.fn().mockResolvedValue({ name: "Taskboard", version: "1.2.3" }),
    listen: vi.fn(async (event: string, handler: Handler) => {
      const set = byEvent.get(event) ?? new Set<Handler>();
      set.add(handler);
      byEvent.set(event, set);
      return () => {
        set.delete(handler);
      };
    }),
  };
  listeners.set(mock, byEvent);
  return mock;
}

/** Fires a Rust event at every handler the given mock has registered for it (and not yet unlistened). */
export function emitEvent<E extends EventName>(api: Api, event: E, payload: EventPayloads[E]): void {
  for (const handler of [...(listeners.get(api)?.get(event) ?? [])]) (handler as (p: EventPayloads[E]) => void)(payload);
}
