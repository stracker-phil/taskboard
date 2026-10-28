import { vi } from "vitest";
import type { api as realApi } from "../api";
// Must not import the store or "../api": they would load the real api module while it is being mocked.
import {
  EMPTY_UI_STATE,
  type Connection,
  type ConnectionInput,
  type ConnectionView,
  type EventName,
  type EventPayloads,
  type Keybindings,
  type ScopeView,
  type Settings,
  type SyncOverview,
  type SyncSettings,
  type UiState,
} from "../api/types";

type Api = typeof realApi;

/** A connection that was never set up. */
const emptyConnection = (): Connection => ({
  siteUrl: "",
  email: "",
  tokenKind: "classic",
  tokenExpires: null,
  cloudId: null,
  accountId: null,
  displayName: null,
  timeZone: null,
});

const defaultSync = (): SyncSettings => ({ scopeOverride: null, focusedMinutes: 5, backgroundMinutes: 15, keepDoneDays: 90 });
type Handler = (payload: never) => void;

const listeners = new WeakMap<object, Map<string, Set<Handler>>>();

/** An `api` where every call resolves to a valid default; override per test with mockResolvedValue. */
export function createApiMock(): Api {
  const byEvent = new Map<string, Set<Handler>>();
  const mock: Api = {
    getSettings: vi.fn(async (): Promise<Settings> => ({ connection: emptyConnection(), sync: defaultSync() })),
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
    connectionGet: vi.fn(async (): Promise<ConnectionView> => ({ connection: emptyConnection(), hasToken: false, expiry: { state: "fine" } })),
    connectionSave: vi.fn(async (input: ConnectionInput, token: string | null): Promise<ConnectionView> => ({
      connection: { ...emptyConnection(), ...input },
      hasToken: token !== null,
      expiry: { state: "fine" },
    })),
    connectionTest: vi.fn().mockResolvedValue({ displayName: "Alex Morgan", accountId: "demo-account-1", timeZone: "Europe/Vienna", cloudId: null }),
    projectsList: vi.fn().mockResolvedValue([]),
    projectsRefresh: vi.fn().mockResolvedValue([]),
    projectTrack: vi.fn().mockResolvedValue(undefined),
    scopeGet: vi.fn(async (): Promise<ScopeView> => ({ jql: "", isDefault: true })),
    scopeValidate: vi.fn().mockResolvedValue(undefined),
    scopeSave: vi.fn(async (jql: string | null): Promise<ScopeView> => ({ jql: jql ?? "", isDefault: jql === null })),
    syncSave: vi.fn(async (sync: SyncSettings) => sync),
    cacheReset: vi.fn().mockResolvedValue(undefined),
    syncOverview: vi.fn(async (): Promise<SyncOverview> => ({ state: { state: "idle", lastOk: null }, lastFullAt: null, projects: [] })),
    syncRun: vi.fn().mockResolvedValue(undefined),
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
