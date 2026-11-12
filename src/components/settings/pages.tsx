/** The core settings pages. Modules register their own pages from their install function. */
import { Activity, Bell, CircleDot, FolderKanban, Info, Keyboard, LayoutGrid, Palette, PlugZap, RefreshCw, ShieldCheck, SlidersHorizontal, TextCursorInput, Users } from "lucide-react";
import { registerSettingsPage } from "../../registry/settingsPages";
import { AppearancePage } from "./AppearancePage";
import { AboutPage } from "./AboutPage";
import { BehaviorPage } from "./BehaviorPage";
import { NotificationsPage } from "./notifications/NotificationsPage";
import { PrivacyPage } from "./privacy/PrivacyPage";
import { ShortcutsPage } from "./ShortcutsPage";
import { ConnectionPage } from "./jira/ConnectionPage";
import { ProjectsPage } from "./jira/ProjectsPage";
import { SpacesPage } from "./jira/SpacesPage";
import { StatusesPage } from "./jira/StatusesPage";
import { CustomFieldsPage } from "./jira/CustomFieldsPage";
import { SyncPage } from "./jira/SyncPage";
import { TeamPage } from "./jira/TeamPage";
import { StatusPage } from "./status/StatusPage";

/** Ids match the pages `settings:open:<page>` and `open_settings` name. */
export function registerCoreSettingsPages() {
  registerSettingsPage({
    id: "connection",
    group: "Jira",
    label: "Connection",
    icon: PlugZap,
    order: 0,
    sections: [
      { id: "site", title: "Jira site" },
      { id: "token", title: "Token" },
      { id: "scopes", title: "Token scopes" },
      { id: "reads", title: "What Taskboard reads" },
      { id: "changes", title: "Changes" },
    ],
    Body: ConnectionPage,
  });
  registerSettingsPage({
    id: "privacy",
    group: "Jira",
    label: "Privacy & Data",
    icon: ShieldCheck,
    order: 1,
    sections: [{ id: "cache", title: "Jira data" }],
    Body: PrivacyPage,
  });
  registerSettingsPage({
    id: "projects",
    group: "Jira",
    label: "Projects",
    icon: FolderKanban,
    order: 2,
    sections: [{ id: "projects", title: "Projects" }],
    Body: ProjectsPage,
  });
  registerSettingsPage({
    id: "spaces",
    group: "Workspace",
    label: "Spaces",
    icon: LayoutGrid,
    order: 0,
    // A Space's own settings open from its row; these titles let the search find them.
    sections: [
      { id: "spaces", title: "Spaces" },
      { id: "layouts", title: "Layouts" },
      { id: "statuses", title: "Statuses" },
      { id: "fields", title: "Custom fields" },
    ],
    Body: SpacesPage,
  });
  registerSettingsPage({
    id: "team",
    group: "Workspace",
    label: "Team",
    icon: Users,
    order: 1,
    sections: [{ id: "team", title: "Team" }],
    Body: TeamPage,
  });
  registerSettingsPage({
    id: "statuses",
    group: "Workspace",
    label: "Statuses",
    icon: CircleDot,
    order: 2,
    sections: [{ id: "statuses", title: "Statuses" }],
    Body: StatusesPage,
  });
  registerSettingsPage({
    id: "fields",
    group: "Workspace",
    label: "Custom fields",
    icon: TextCursorInput,
    order: 3,
    sections: [{ id: "fields", title: "Custom fields" }],
    Body: CustomFieldsPage,
  });
  registerSettingsPage({
    id: "sync",
    group: "Jira",
    label: "Sync",
    icon: RefreshCw,
    order: 3,
    sections: [
      { id: "scope", title: "Scope" },
      { id: "issue-types", title: "Issue types" },
      { id: "intervals", title: "Intervals" },
      { id: "synced", title: "Synced" },
    ],
    Body: SyncPage,
  });
  registerSettingsPage({ id: "behavior", group: "Workspace", label: "Behavior", icon: SlidersHorizontal, order: 4, sections: [{ id: "behaviour", title: "Behavior" }, { id: "meMode", title: "Me Mode" }], Body: BehaviorPage });
  registerSettingsPage({
    id: "notifications",
    group: "Workspace",
    label: "Notifications",
    icon: Bell,
    order: 5,
    sections: [
      { id: "which-issues", title: "Which issues" },
      { id: "notify-about", title: "Notify me about" },
    ],
    Body: NotificationsPage,
  });
  registerSettingsPage({ id: "appearance", group: "System", label: "Appearance", icon: Palette, order: 1, sections: [{ id: "appearance", title: "Appearance" }], Body: AppearancePage });
  registerSettingsPage({ id: "shortcuts", group: "System", label: "Shortcuts", icon: Keyboard, order: 2, sections: [{ id: "shortcuts", title: "Shortcuts" }], Body: ShortcutsPage });
  registerSettingsPage({ id: "status", group: "System", label: "Status", icon: Activity, order: 3, sections: [{ id: "status", title: "Status" }], Body: StatusPage });
  registerSettingsPage({ id: "about", group: "System", label: "About", icon: Info, order: 4, sections: [{ id: "about", title: "About" }], Body: AboutPage });
}
