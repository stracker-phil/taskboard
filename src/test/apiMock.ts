import { vi } from "vitest";
import type { api as realApi } from "../api";
// Must not import the store or "../api": they would load the real api module while it is being mocked.
import {
  EMPTY_UI_STATE,
  type Accent,
  type Appearance,
  type Assignees,
  type Behaviour,
  type BoardData,
  type BulkOutcome,
  type CapabilityRow,
  type ChangeNotAllowed,
  type Connection,
  type ConnectionInput,
  type ConnectionView,
  type Diagnostics,
  type DropOutcome,
  type DropTargets,
  type EventName,
  type EventPayloads,
  type FilterOptions,
  type IssueChange,
  type IssueDetail,
  type Keybindings,
  type LinkTarget,
  type ListConditions,
  type MeMatch,
  type MenuCommand,
  type Nav,
  type PaletteResults,
  type Pins,
  type Priorities,
  type Related,
  type SavedSearch,
  type ScopeView,
  type Settings,
  type SidebarTree,
  type SpaceConfig,
  type SubtaskMode,
  type SyncOverview,
  type SyncSettings,
  type TeamMatrix,
  type UiState,
  type View,
  type ViewData,
  type WriteOutcome,
} from "../api/types";
import { capabilityRows } from "./capabilities";
import { filterOptions } from "./filters";
import { issueDetail } from "./issues";
import { nav } from "./nav";
import { settings } from "./settings";
import { spaceConfig } from "./spaces";
import { view } from "./views";

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

/** A sidebar with Everything only: nothing tracked yet. */
const emptySidebar = (): SidebarTree => ({
  everything: { id: 1, kind: "everything", name: "Everything", count: 0, ownConditions: 0, description: "", error: null, icon: null, color: null },
  home: [],
  pinnedLists: [],
  spaces: [],
});

/** A unit-result command; Tauri answers null for `invoke<void>`, never undefined. */
const done = () => vi.fn().mockResolvedValue(null);

type Handler = (payload: never) => void;

const listeners = new WeakMap<object, Map<string, Set<Handler>>>();

/** An `api` where every call resolves to a valid default; override per test with mockResolvedValue. */
export function createApiMock(): Api {
  const byEvent = new Map<string, Set<Handler>>();
  const mock: Api = {
    getSettings: vi.fn(async (): Promise<Settings> => settings()),
    updateSettings: vi.fn(async (s: Settings) => s),
    appearanceSet: vi.fn(async (appearance: Appearance): Promise<Settings> => settings({ appearance })),
    accentSet: vi.fn(async (accent: Accent): Promise<Settings> => settings({ accent })),
    behaviourSet: vi.fn(async (behaviour: Behaviour): Promise<Settings> => settings({ behaviour })),
    jiraCapabilities: vi.fn(async (): Promise<CapabilityRow[]> => capabilityRows()),
    jiraCapabilitySet: vi.fn(async (): Promise<Settings> => settings()),
    getKeybindings:vi.fn().mockResolvedValue({}),
    setKeybindings: vi.fn(async (bindings: Keybindings) => bindings),
    getUiState: vi.fn().mockResolvedValue(EMPTY_UI_STATE),
    setUiState: vi.fn(async (ui: UiState) => ui),
    openLogFolder: done(),
    openUrl: done(),
    doubleClickMs: vi.fn().mockResolvedValue(500),
    attachmentPreview: done(),
    inlineImageUrl: vi.fn((issueId: string, attachmentId: string) => `inline-image://localhost/${issueId}/${attachmentId}`),
    logError: done(),
    takeLaunchEvents: vi.fn().mockResolvedValue([]),
    runCommand: done(),
    connectionGet: vi.fn(async (): Promise<ConnectionView> => ({ connection: emptyConnection(), hasToken: false, expiry: { state: "fine" } })),
    connectionChangePermissions: vi.fn(async (): Promise<ChangeNotAllowed[]> => []),
    connectionSave: vi.fn(async (input: ConnectionInput, token: string | null): Promise<ConnectionView> => ({
      connection: { ...emptyConnection(), ...input },
      hasToken: token !== null,
      expiry: { state: "fine" },
    })),
    connectionTest: vi.fn().mockResolvedValue({ displayName: "Alex Morgan", accountId: "demo-account-1", timeZone: "Europe/Berlin", cloudId: null }),
    projectsList: vi.fn().mockResolvedValue([]),
    projectsRefresh: vi.fn().mockResolvedValue([]),
    projectTrack: done(),
    scopeGet: vi.fn(async (): Promise<ScopeView> => ({ jql: "", isDefault: true })),
    scopeValidate: done(),
    scopeSave: vi.fn(async (jql: string | null): Promise<ScopeView> => ({ jql: jql ?? "", isDefault: jql === null })),
    syncSave: vi.fn(async (sync: SyncSettings) => sync),
    cacheReset: done(),
    sidebarTree: vi.fn(async () => emptySidebar()),
    listCreate: vi.fn(),
    listRename: done(),
    listDuplicate: vi.fn(),
    listDelete: done(),
    listMove: done(),
    listSetFilter: done(),
    listSetLook: done(),
    listConditions: vi.fn(async (): Promise<ListConditions> => ({ kind: "everything", locked: [], own: [], error: null, view: [], viewError: null, projectFixed: false, fixed: null })),
    listSetWindow: done(),
    filterOptions: vi.fn(async (): Promise<FilterOptions> => filterOptions()),
    listMatchCount: vi.fn(),
    filterParseKeys: vi.fn(),
    spaceMove: done(),
    spaceUpdate: done(),
    spaceSetAccent: done(),
    spaceDefaultColumns: done(),
    spaceConfig: vi.fn(async (projectId: string): Promise<SpaceConfig> => spaceConfig({ projectId })),
    spaceSetLayouts: vi.fn(),
    spaceSetField: vi.fn(),
    fieldChoices: vi.fn().mockResolvedValue([]),
    fieldChoiceSet: vi.fn().mockResolvedValue([]),
    teamMatrix: vi.fn(async (): Promise<TeamMatrix> => ({ spaces: [], people: [] })),
    spaceSetMember: vi.fn(async (): Promise<TeamMatrix> => ({ spaces: [], people: [] })),
    navCurrent: vi.fn(async (): Promise<Nav> => nav(null)),
    navGo: vi.fn(async (listId: number, viewId: number | null): Promise<Nav> => nav({ listId, viewId: viewId ?? 0 })),
    navTabNew: vi.fn(async (listId: number | null, viewId: number | null): Promise<Nav> => nav(listId === null ? null : { listId, viewId: viewId ?? 0 })),
    navTabActivate: vi.fn(async (): Promise<Nav> => nav(null)),
    navTabClose: vi.fn(async (): Promise<Nav> => nav(null)),
    navTabMove: vi.fn(async (): Promise<Nav> => nav(null)),
    navSetExpanded: vi.fn(async (expanded: number[]): Promise<Nav> => nav(null, { expanded })),
    navSetIssue: vi.fn(async (): Promise<Nav> => nav(null)),
    navBack: vi.fn(async (): Promise<Nav> => nav(null)),
    navForward: vi.fn(async (): Promise<Nav> => nav(null)),
    viewsOf: vi.fn().mockResolvedValue([]),
    viewCreate: vi.fn(),
    viewRename: vi.fn(),
    viewDuplicate: vi.fn(),
    viewSetLayout: vi.fn(),
    viewSetLook: vi.fn(),
    viewSetFilter: vi.fn(async (id: number): Promise<View> => view({ id })),
    viewSetOnlyMine: vi.fn(async (id: number, on: boolean): Promise<View> => view({ id, onlyMine: on })),
    viewSetColumns: vi.fn(),
    viewSetGroup: vi.fn(),
    viewSetSort: vi.fn(),
    viewSetWidths: vi.fn(),
    viewSetWrap: vi.fn(),
    viewSetInlineLabels: vi.fn(),
    viewSetOpenWith: vi.fn(),
    viewSetColumnCollapsed: vi.fn(),
    viewSetSubtasks: vi.fn(async (id: number, subtasks: SubtaskMode): Promise<View> => view({ id, subtasks })),
    viewMove: vi.fn(),
    viewDelete: vi.fn(),
    userAvatar: vi.fn(async (): Promise<string | null> => null),
    viewRows: vi.fn(async (): Promise<ViewData> => ({ groupBy: "none", groups: [], rows: {}, columns: [], subtasks: "hidden" })),
    viewBoard: vi.fn(async (): Promise<BoardData> => ({ columns: [], rows: {}, cardFields: [] })),
    issueGet: vi.fn(async (id: string): Promise<IssueDetail> => issueDetail({ id })),
    issueActivity: vi.fn().mockResolvedValue([]),
    issueRelated: vi.fn(async (): Promise<Related> => ({ children: { open: [], done: [] }, links: [] })),
    issueWebLinks: vi.fn().mockResolvedValue([]),
    issueMarkdown: vi.fn().mockResolvedValue("# Demo issue\n"),
    issueSyncWebLinks: vi.fn().mockResolvedValue(false),
    issueAddWebLink: vi.fn(async (): Promise<WriteOutcome> => ({ done: "Added the link to NW-1042", reverse: null })),
    issueSyncHistory: vi.fn().mockResolvedValue(false),
    issueForget: done(),
    issueComment: vi.fn(async (): Promise<WriteOutcome> => ({ done: "Done", reverse: null })),
    issueDropTargets: vi.fn(async (): Promise<DropTargets> => ({ targets: [], note: null })),
    issueDrop: vi.fn(async (): Promise<DropOutcome> => ({ kind: "done", outcome: { done: "Done", reverse: null } })),
    issueTransitions: vi.fn().mockResolvedValue([]),
    issueAssignees: vi.fn(async (): Promise<Assignees> => ({ people: [], current: null })),
    issuePriorities: vi.fn(
      async (): Promise<Priorities> => ({
        options: [
          { name: "Highest", level: 1 },
          { name: "High", level: 2 },
          { name: "Medium", level: 3 },
          { name: "Low", level: 4 },
          { name: "Lowest", level: 5 },
        ],
        current: "Medium",
      }),
    ),
    issuesBulkStatuses: vi.fn().mockResolvedValue([]),
    issuesBulkPriorities: vi.fn().mockResolvedValue([]),
    issuesBulkAssignees: vi.fn().mockResolvedValue([]),
    issuesWriteMany: vi.fn(async (changes: IssueChange[]): Promise<BulkOutcome> => ({ done: "Done", changed: changes.length, refused: 0, reverse: null })),
    meUserFields: vi.fn().mockResolvedValue([]),
    viewSetPerson: vi.fn(async (id: number, person: string | null) => ({ ...view(), id, person })),
    viewSetMeRules: vi.fn(async (id: number, rules: MeMatch[] | null): Promise<View> => view({ id, meRules: rules })),
    viewPeople: vi.fn().mockResolvedValue([]),
    pinRename: done(),
    pinMove: done(),
    meRuleEdit: vi.fn().mockResolvedValue(settings()),
    modesGet: vi.fn().mockResolvedValue({ meEverywhere: false, unreadOnly: false, bookmarksBar: false }),
    modeSet: vi.fn(async (mode: string, on: boolean) => ({ meEverywhere: mode === "meEverywhere" && on, unreadOnly: mode === "unreadOnly" && on, bookmarksBar: mode === "bookmarksBar" && on })),
    issueWrite: vi.fn(async (): Promise<WriteOutcome> => ({ done: "Done", reverse: null })),
    issueToggleMine: vi.fn(async (): Promise<WriteOutcome> => ({ done: "Done", reverse: null })),
    issueSyncComments: vi.fn().mockResolvedValue(false),
    issuesMarkSeen: done(),
    issueViewed: done(),
    issuePreviewed: done(),
    issueMarkUnread: done(),
    listMarkAllSeen: done(),
    pinsList: vi.fn(async (): Promise<Pins> => ({ active: [], recentlyUnpinned: [] })),
    pinCreate: done(),
    pinRemove: done(),
    pinToggleIssue: done(),
    placeGet:vi.fn().mockResolvedValue(null),
    placeSet: done(),
    copyText: vi.fn().mockResolvedValue(undefined),
    appLink: vi.fn(async (target: LinkTarget) =>
      target.kind === "issue" ? `taskboard-debug://issue/${target.key}` : `taskboard-debug://level/${target.listId}/${target.viewId ?? ""}`,
    ),
    paletteQuery: vi.fn(async (): Promise<PaletteResults> => ({ issues: [], lists: [], views: [] })),
    savedSearches: vi.fn(async (): Promise<SavedSearch[]> => []),
    recentSearches: vi.fn(async (): Promise<string[]> => []),
    searchRemember: vi.fn(async (text: string): Promise<string[]> => [text]),
    searchForget: vi.fn(async (): Promise<string[]> => []),
    menuCommands: vi.fn(async (): Promise<MenuCommand[]> => []),
    columnsCatalogue: vi.fn().mockResolvedValue([]),
    statusStylesList: vi.fn().mockResolvedValue([]),
    statusStyleSet: vi.fn().mockResolvedValue([]),
    statusStylesAll: vi.fn().mockResolvedValue([]),
    statusStyleSetApp: vi.fn().mockResolvedValue([]),
    syncOverview: vi.fn(async (): Promise<SyncOverview> => ({ state: { state: "idle", lastOk: null }, lastFullAt: null, projects: [] })),
    syncRun: done(),
    diagnosticsGet: vi.fn(async (): Promise<Diagnostics> => ({ groups: [] })),
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
