import { getName, getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActivityItem,
  AppError,
  AppInfo,
  Appearance,
  Behaviour,
  BoardData,
  ConnectionInput,
  ConnectionTest,
  ConnectionView,
  Diagnostics,
  ErrorKind,
  EventName,
  EventPayloads,
  Filter,
  GroupBy,
  FieldVisibility,
  Keybindings,
  LaunchEvent,
  Layout,
  FilterOptions,
  IssueDetail,
  IssueOpening,
  ListConditions,
  MatchCount,
  Narrowing,
  Nav,
  PaletteResults,
  SpaceConfig,
  SpaceLayouts,
  PinDuration,
  PinKind,
  Pins,
  ProjectRow,
  Related,
  ScopeView,
  Settings,
  SidebarTree,
  Sort,
  SpaceUpdate,
  Column,
  StatusStyleRow,
  SubtaskMode,
  Style,
  SyncOverview,
  SyncSettings,
  UiState,
  View,
  ViewData,
  ViewPlace,
  WebLink,
} from "./types";

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
  /** Settings → Status: the connection, the sync and this Mac, each row with a tone and maybe a command. */
  diagnosticsGet: () => invoke<Diagnostics>("diagnostics_get"),
  /** https links only; Rust refuses anything else. */
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  /** Downloads the attachment from the Jira site (once per app run) and shows it in Quick Look. */
  attachmentPreview: (issueId: string, attachmentId: string) => invoke<void>("attachment_preview", { issueId, attachmentId }),
  logError: (message: string) => invoke<void>("log_frontend_error", { message }),
  takeLaunchEvents: () => invoke<LaunchEvent[]>("take_launch_events"),
  /** Runs a command of the Rust command bus by id (`app:settings`, `view:toggleSidebar`, …); rejects when it is unknown, disabled or fails. */
  runCommand: (id: string) => invoke<void>("run_command", { id }),
  /** The saved connection, whether a token is in the Keychain and when it runs out; never the token. */
  connectionGet: () => invoke<ConnectionView>("connection_get"),
  /** `token` goes to the Keychain; `null` keeps the stored one. */
  connectionSave: (input: ConnectionInput, token: string | null) => invoke<ConnectionView>("connection_save", { input, token }),
  /** Tests the saved connection. Errors: `auth`, `offline`, `not_found` (the site gave no cloud id), `not_connected`. */
  connectionTest: () => invoke<ConnectionTest>("connection_test"),
  /** The projects fetched last time, with the owner's tracked flag. */
  projectsList: () => invoke<ProjectRow[]>("projects_list"),
  /** Fetches the projects from Jira, then lists them. */
  projectsRefresh: () => invoke<ProjectRow[]>("projects_refresh"),
  projectTrack: (projectId: string, tracked: boolean) => invoke<void>("project_track", { projectId, tracked }),
  scopeGet: () => invoke<ScopeView>("scope_get"),
  /** Rejects with `invalid_jql` and Jira's messages. */
  scopeValidate: (jql: string) => invoke<void>("scope_validate", { jql }),
  /** `null` goes back to the default scope; an invalid query rejects with `invalid_jql`. */
  scopeSave: (jql: string | null) => invoke<ScopeView>("scope_save", { jql }),
  /** Saves the intervals and days to keep; the scope changes through `scopeSave`. */
  syncSave: (sync: SyncSettings) => invoke<SyncSettings>("sync_save", { sync }),
  /** The sync state and what the cache holds per tracked project. */
  syncOverview: () => invoke<SyncOverview>("sync_overview"),
  /** Queues a full sync and returns at once; progress arrives as `sync_progress` and `sync_state` events. */
  syncRun: () => invoke<void>("sync_run"),
  /** Deletes the Jira cache; Spaces, filter lists, views and pins stay. */
  cacheReset: () => invoke<void>("cache_reset"),
  /** Everything, Home and the tracked Spaces with their filter lists and open counts. */
  sidebarTree: () => invoke<SidebarTree>("sidebar_tree"),
  /** `parentId`: a Space's node, or Everything's for a list under Home. Resolves to the new list's id. */
  listCreate: (parentId: number, name: string, filter: Filter | null) => invoke<number>("list_create", { parentId, name, filter }),
  listRename: (id: number, name: string) => invoke<void>("list_rename", { id, name }),
  listDuplicate: (id: number) => invoke<number>("list_duplicate", { id }),
  /** Also deletes its views and the pins on it. */
  listDelete: (id: number) => invoke<void>("list_delete", { id }),
  listMove: (id: number, position: number) => invoke<void>("list_move", { id, position }),
  listSetFilter: (id: number, filter: Filter) => invoke<void>("list_set_filter", { id, filter }),
  /** What `id` inherits ANDed with `filter` in place of its own conditions. */
  /** A level's locked conditions (with display names), a filter list's own ones and the view's own ones. */
  listConditions: (id: number, viewId: number | null) => invoke<ListConditions>("list_conditions", { id, viewId }),
  /** The fields and cached values the Filter popover's selects offer. */
  filterOptions: () => invoke<FilterOptions>("filter_options"),
  listMatchCount: (id: number, filter: Filter) => invoke<MatchCount>("list_match_count", { id, filter }),
  spaceMove: (projectId: string, position: number) => invoke<void>("space_move", { projectId, position }),
  spaceUpdate: (projectId: string, update: SpaceUpdate) => invoke<void>("space_update", { projectId, update }),
  /** "Use as Space default": the columns new views in this Space start with. */
  spaceDefaultColumns: (projectId: string, columns: string[]) => invoke<void>("space_default_columns", { projectId, columns }),
  /** Where the main window is; Everything when the saved place is gone. */
  navCurrent: () => invoke<Nav>("nav_current"),
  /** Opens a level at `viewId`, or at its last or default view when `viewId` is null. */
  navGo: (listId: number, viewId: number | null) => invoke<Nav>("nav_go", { listId, viewId }),
  /** The Space nodes the owner expanded in the sidebar. */
  navSetExpanded: (expanded: number[]) => invoke<Nav>("nav_set_expanded", { expanded }),
  /** A filter list's icon by name; null goes back to the funnel. */
  /** No icon is the funnel; no color shows the icon plain. */
  listSetLook: (id: number, icon: string | null, color: string | null) => invoke<void>("list_set_look", { id, icon, color }),
  viewsOf: (listId: number) => invoke<View[]>("views_of", { listId }),
  viewCreate: (listId: number, name: string, layout: Layout) => invoke<View>("view_create", { listId, name, layout }),
  viewRename: (id: number, name: string) => invoke<View>("view_rename", { id, name }),
  viewDuplicate: (id: number) => invoke<View>("view_duplicate", { id }),
  viewSetLayout: (id: number, layout: Layout) => invoke<View>("view_set_layout", { id, layout }),
  viewSetColumns: (id: number, columns: string[]) => invoke<View>("view_set_columns", { id, columns }),
  viewSetGroup: (id: number, groupBy: GroupBy) => invoke<View>("view_set_group", { id, groupBy }),
  viewSetSort: (id: number, sort: Sort) => invoke<View>("view_set_sort", { id, sort }),
  viewSetWrap: (id: number, wrap: boolean) => invoke<View>("view_set_wrap", { id, wrap }),
  /** Labels as tags after the title while the view has no Labels column. */
  viewSetInlineLabels: (id: number, inline: boolean) => invoke<View>("view_set_inline_labels", { id, inline }),
  /** How the view shows subtasks; the Board ignores it. */
  viewSetSubtasks: (id: number, mode: SubtaskMode) => invoke<View>("view_set_subtasks", { id, mode }),
  /** No icon shows the layout's; no color shows the icon plain. */
  viewSetLook: (id: number, icon: string | null, color: string | null) => invoke<View>("view_set_look", { id, icon, color }),
  /** Null follows Settings → General. */
  viewSetOpenWith: (id: number, openWith: IssueOpening | null) => invoke<View>("view_set_open_with", { id, openWith }),
  /** The Table layout's column widths by column id; Rust clamps them to 48–800. */
  viewSetWidths: (id: number, widths: Record<string, number>) => invoke<View>("view_set_widths", { id, widths }),
  viewMove: (id: number, position: number) => invoke<View[]>("view_move", { id, position }),
  /** Rejects with `invalid_input` for the last view of a level. */
  viewDelete: (id: number) => invoke<View[]>("view_delete", { id }),
  /** A view's issues, filtered by its level and `narrowing`, grouped and sorted by the view. */
  viewRows: (listId: number, viewId: number, narrowing: Narrowing) => invoke<ViewData>("view_rows", { listId, viewId, narrowing }),
  /** The Board layout: a column per status of the issues shown, with the cards' rows. */
  viewBoard: (listId: number, viewId: number, narrowing: Narrowing) => invoke<BoardData>("view_board", { listId, viewId, narrowing }),
  /** The view's own conditions, ANDed with its level's; rejects with `invalid_input` when they don't compile. */
  viewSetFilter: (id: number, filter: Filter) => invoke<View>("view_set_filter", { id, filter }),
  viewSetColumnCollapsed: (id: number, statusId: string, collapsed: boolean) => invoke<View>("view_set_column_collapsed", { id, statusId, collapsed }),
  /** How long an open issue takes to count as seen, and the default pin duration. */
  behaviourSet: (behaviour: Behaviour) => invoke<Settings>("behaviour_set", { behaviour }),
  /** Light, dark or following macOS; every window repaints. */
  appearanceSet: (appearance: Appearance) => invoke<Settings>("appearance_set", { appearance }),
  issueGet: (id: string) => invoke<IssueDetail>("issue_get", { id }),
  /** Remove from cache: only for an issue gone from Jira. */
  issueForget: (id: string) => invoke<void>("issue_forget", { id }),
  issueActivity: (id: string) => invoke<ActivityItem[]>("issue_activity", { id }),
  issueRelated: (id: string) => invoke<Related>("issue_related", { id }),
  /** The command palette's issues (by key and full text) and filter lists for the typed text. */
  paletteQuery: (text: string) => invoke<PaletteResults>("palette_query", { text }),
  /** Their highlight and change chip clear. */
  issuesMarkSeen: (ids: string[]) => invoke<void>("issues_mark_seen", { ids }),
  /** Unread again: from the activity item at `from` (its time) on, or the whole issue. */
  issueMarkUnread: (id: string, from: string | null = null) => invoke<void>("issue_mark_unread", { id, from }),
  pinsList: () => invoke<Pins>("pins_list"),
  /** Pins the target, or changes the duration and reason of its pin. */
  pinCreate: (kind: PinKind, target: string, duration: PinDuration, reason: string | null) =>
    invoke<void>("pin_create", { kind, target, duration, reason }),
  /** Ends the pin now; it moves to Recently unpinned. */
  pinRemove: (id: number) => invoke<void>("pin_remove", { id }),
  /** P: unpins the issue, or pins it until unpinned. */
  pinToggleIssue: (id: string) => invoke<void>("pin_toggle_issue", { id }),
  /** Marks every issue of the filter list with unseen changes seen. */
  listMarkAllSeen: (listId: number) => invoke<void>("list_mark_all_seen", { listId }),
  /** Fetches the issue's comments from Jira when the cached ones may be behind. */
  issueSyncComments: (id: string) => invoke<boolean>("issue_sync_comments", { id }),
  /** The issue's cached web links (pull requests, documents). */
  issueWebLinks: (id: string) => invoke<WebLink[]>("issue_web_links", { id }),
  /** Fetches the issue's web links from Jira; true when they changed. */
  issueSyncWebLinks: (id: string) => invoke<boolean>("issue_sync_web_links", { id }),
  /** Fetches the issue's history (Jira's changelog) at most every 5 minutes; true when it changed. */
  issueSyncHistory: (id: string) => invoke<boolean>("issue_sync_history", { id }),
  /** The person's profile picture as a `data:` URL, fetched and cached by Rust; null shows initials. */
  userAvatar: (accountId: string) => invoke<string | null>("user_avatar", { accountId }),
  placeGet: (listId: number, viewId: number) => invoke<ViewPlace | null>("place_get", { listId, viewId }),
  placeSet: (listId: number, viewId: number, place: ViewPlace) => invoke<void>("place_set", { listId, viewId, place }),
  /** The system clipboard; only text. */
  copyText: (text: string) => navigator.clipboard.writeText(text),
  /** Every column a view can show: the standard ones, then the site's custom fields. */
  columnsCatalogue: (listId: number) => invoke<Column[]>("columns_catalogue", { listId }),
  statusStylesList: (projectId: string) => invoke<StatusStyleRow[]>("status_styles_list", { projectId }),
  /** `style` null: back to the detected style. */
  statusStyleSet: (projectId: string, statusId: string, style: Style | null) =>
    invoke<StatusStyleRow[]>("status_style_set", { projectId, statusId, style }),
  spaceConfig: (projectId: string) => invoke<SpaceConfig>("space_config", { projectId }),
  spaceSetLayouts: (projectId: string, layouts: SpaceLayouts) => invoke<SpaceConfig>("space_set_layouts", { projectId, layouts }),
  spaceSetField: (projectId: string, fieldId: string, visibility: FieldVisibility) =>
    invoke<SpaceConfig>("space_set_field", { projectId, fieldId, visibility }),
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
  db: true,
  auth: true,
  offline: true,
  invalid_jql: true,
  internal: true,
};

export function toAppError(e: unknown): AppError {
  if (!(e !== null && typeof e === "object" && "message" in e)) return { kind: "io", message: String(e) };
  if ("kind" in e && Object.hasOwn(ERROR_KINDS, String(e.kind))) return e as AppError;
  return { kind: "io", message: String(e.message) };
}
