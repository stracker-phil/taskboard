/** The core settings pages and the rows the search finds on them. Modules register their own pages from their
 *  install function. */
import { Activity, Bell, CircleDot, FolderKanban, Info, Keyboard, LayoutGrid, Palette, PlugZap, RefreshCw, ShieldCheck, SlidersHorizontal, TextCursorInput, Users } from "lucide-react";
import { allCommands } from "../../commands/registry";
import { registerSettingsPage, type SettingsRow } from "../../registry/settingsPages";
import { AppearancePage } from "./AppearancePage";
import { AboutPage } from "./AboutPage";
import { BehaviorPage } from "./BehaviorPage";
import { NotificationsPage } from "./notifications/NotificationsPage";
import { PrivacyPage } from "./privacy/PrivacyPage";
import { MENU_KEYS, menuKeyAnchor, shortcutAnchor, ShortcutsPage } from "./ShortcutsPage";
import { ConnectionPage } from "./jira/ConnectionPage";
import { ProjectsPage } from "./jira/ProjectsPage";
import { SpacesPage } from "./jira/SpacesPage";
import { StatusesPage } from "./jira/StatusesPage";
import { CustomFieldsPage } from "./jira/CustomFieldsPage";
import { SyncPage } from "./jira/SyncPage";
import { TeamPage } from "./jira/TeamPage";
import { STATUS_GROUPS, StatusPage } from "./status/StatusPage";

const CONNECTION_ROWS: SettingsRow[] = [
  { id: "site", title: "Jira site" },
  { id: "site-url", title: "Site URL", keywords: ["address", "atlassian"] },
  { id: "email", title: "Email", keywords: ["account"] },
  { id: "api-token", title: "API token", keywords: ["password", "keychain"] },
  { id: "token-kind", title: "Token kind", keywords: ["classic", "scoped"] },
  { id: "cloud-id", title: "Cloud ID", under: "token-kind" },
  { id: "token", title: "Token" },
  { id: "token-expires", title: "Expires on", keywords: ["expiry", "token"] },
  { id: "scopes", title: "Token scopes", keywords: ["permissions"] },
  { id: "reads", title: "What Taskboard reads" },
  { id: "changes", title: "Changes", keywords: ["write", "edit"] },
  { id: "allow-changes", title: "Allow changes to Jira", keywords: ["write", "read-only"] },
];

const SYNC_ROWS: SettingsRow[] = [
  { id: "scope", title: "Scope" },
  { id: "scope-query", title: "Query", keywords: ["jql", "scope"] },
  { id: "issue-types", title: "Issue types" },
  { id: "leave-out", title: "Leave out", keywords: ["issue types", "exclude"] },
  { id: "intervals", title: "Intervals", keywords: ["how often"] },
  { id: "interval-front", title: "While Taskboard is in front", keywords: ["interval", "minutes"] },
  { id: "interval-background", title: "In the background", keywords: ["interval", "minutes"] },
  { id: "keep-done", title: "Keep done issues", keywords: ["days", "closed"] },
  { id: "synced", title: "Synced" },
  { id: "sync-state", title: "Sync", keywords: ["sync now", "last sync"] },
];

const NOTIFICATION_ROWS: SettingsRow[] = [
  { id: "which-issues", title: "Which issues" },
  { id: "notify-allIssues", title: "All issues in my Spaces" },
  { id: "notify-involved", title: "Only issues I'm involved in" },
  { id: "notify-assigned", title: "Assigned to me" },
  { id: "notify-reported", title: "Created by me", keywords: ["reporter"] },
  { id: "notify-peopleFields", title: "Named in a people field", keywords: ["user field"] },
  { id: "notify-types", title: "Issue types" },
  { id: "ignore-automations", title: "Ignore changes made by automations", keywords: ["bots", "apps"] },
  { id: "notify-about", title: "Notify me about", keywords: ["mentions", "comments", "macos"] },
];

/** Every command's key, and the menu's fixed ones; read when the page registers, after the commands. */
function shortcutRows(): SettingsRow[] {
  return [
    { id: "shortcuts-window", title: "In the Taskboard window" },
    ...allCommands().map((command) => ({ id: shortcutAnchor(command.id), title: command.title })),
    { id: "shortcuts-menu", title: "In the menu" },
    ...MENU_KEYS.map((key) => ({ id: menuKeyAnchor(key.title), title: key.title })),
  ];
}

/** Ids match the pages `app:settings:<page>` and `open_settings` name; a row id opens its page scrolled to it. */
export function registerCoreSettingsPages() {
  registerSettingsPage({ id: "connection", group: "Jira", label: "Connection", icon: PlugZap, order: 0, keywords: ["account", "login"], rows: CONNECTION_ROWS, Body: ConnectionPage });
  registerSettingsPage({
    id: "privacy",
    group: "Jira",
    label: "Privacy & Data",
    icon: ShieldCheck,
    order: 1,
    rows: [
      { id: "cache", title: "Jira data" },
      { id: "cache-memory", title: "Jira data in memory", keywords: ["refetch", "cache"] },
      { id: "data-folder", title: "Data folder" },
    ],
    Body: PrivacyPage,
  });
  registerSettingsPage({ id: "projects", group: "Jira", label: "Projects", icon: FolderKanban, order: 2, keywords: ["track"], rows: [{ id: "jira-projects", title: "Jira projects", keywords: ["refresh", "track"] }], Body: ProjectsPage });
  registerSettingsPage({ id: "sync", group: "Jira", label: "Sync", icon: RefreshCw, order: 3, rows: SYNC_ROWS, Body: SyncPage });
  // A Space's own settings open from its row; the keywords let the search find them.
  registerSettingsPage({ id: "spaces", group: "Workspace", label: "Spaces", icon: LayoutGrid, order: 0, keywords: ["layouts", "client", "space accent", "space statuses", "space custom fields", "space team"], Body: SpacesPage });
  registerSettingsPage({ id: "team", group: "Workspace", label: "Team", icon: Users, order: 1, keywords: ["people", "members", "assignees"], Body: TeamPage });
  registerSettingsPage({ id: "statuses", group: "Workspace", label: "Statuses", icon: CircleDot, order: 2, keywords: ["status styles", "workflow"], Body: StatusesPage });
  registerSettingsPage({ id: "fields", group: "Workspace", label: "Custom fields", icon: TextCursorInput, order: 3, keywords: ["columns"], Body: CustomFieldsPage });
  registerSettingsPage({
    id: "behavior",
    group: "Workspace",
    label: "Behavior",
    icon: SlidersHorizontal,
    order: 4,
    rows: [
      { id: "read-after", title: "Count an issue as read after", keywords: ["unread", "seen"] },
      { id: "open-in", title: "Open issues in", keywords: ["preview", "click"] },
      { id: "bookmark-for", title: "Bookmark for", keywords: ["pin", "duration"] },
      { id: "meMode", title: "Me Mode", keywords: ["only my issues"] },
      { id: "me-fields", title: "Fields", keywords: ["me mode", "user field"] },
    ],
    Body: BehaviorPage,
  });
  registerSettingsPage({ id: "notifications", group: "Workspace", label: "Notifications", icon: Bell, order: 5, keywords: ["alerts", "unread", "inbox"], rows: NOTIFICATION_ROWS, Body: NotificationsPage });
  registerSettingsPage({
    id: "appearance",
    group: "System",
    label: "Appearance",
    icon: Palette,
    order: 1,
    keywords: ["look"],
    rows: [
      { id: "mode", title: "Mode", keywords: ["dark", "light", "automatic"] },
      { id: "accent", title: "Accent color", keywords: ["color", "highlight"] },
    ],
    Body: AppearancePage,
  });
  registerSettingsPage({ id: "shortcuts", group: "System", label: "Shortcuts", icon: Keyboard, order: 2, keywords: ["keys", "keyboard"], rows: shortcutRows(), Body: ShortcutsPage });
  registerSettingsPage({ id: "status", group: "System", label: "Status", icon: Activity, order: 3, keywords: ["diagnostics"], rows: STATUS_GROUPS, Body: StatusPage });
  registerSettingsPage({
    id: "about",
    group: "System",
    label: "About",
    icon: Info,
    order: 4,
    rows: [
      { id: "about-version", title: "Version" },
      { id: "about-data", title: "Data", keywords: ["data folder"] },
      { id: "about-logs", title: "Logs", keywords: ["log folder", "bug report"] },
    ],
    Body: AboutPage,
  });
}
