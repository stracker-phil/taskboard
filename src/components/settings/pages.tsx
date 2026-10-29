/** The core settings pages. Modules register their own pages from their install function. */
import { Activity, FolderKanban, Keyboard, LayoutGrid, Palette, PlugZap, RefreshCw, SlidersHorizontal } from "lucide-react";
import { registerSettingsPage } from "../../registry/settingsPages";
import { AppearancePage } from "./AppearancePage";
import { GeneralPage } from "./GeneralPage";
import { ShortcutsPage } from "./ShortcutsPage";
import { ConnectionPage } from "./jira/ConnectionPage";
import { ProjectsPage } from "./jira/ProjectsPage";
import { SpacesPage } from "./jira/SpacesPage";
import { SyncPage } from "./jira/SyncPage";
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
    ],
    Body: ConnectionPage,
  });
  registerSettingsPage({
    id: "projects",
    group: "Jira",
    label: "Projects",
    icon: FolderKanban,
    order: 1,
    sections: [{ id: "projects", title: "Projects" }],
    Body: ProjectsPage,
  });
  registerSettingsPage({
    id: "spaces",
    group: "Jira",
    label: "Spaces",
    icon: LayoutGrid,
    order: 2,
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
    id: "sync",
    group: "Jira",
    label: "Sync",
    icon: RefreshCw,
    order: 3,
    sections: [
      { id: "scope", title: "Scope" },
      { id: "intervals", title: "Intervals" },
      { id: "cache", title: "Cache" },
    ],
    Body: SyncPage,
  });
  registerSettingsPage({ id: "general", group: "System", label: "General", icon: SlidersHorizontal, order: 0, sections: [{ id: "behaviour", title: "Behaviour" }, { id: "about", title: "About" }], Body: GeneralPage });
  registerSettingsPage({ id: "appearance", group: "System", label: "Appearance", icon: Palette, order: 1, sections: [{ id: "appearance", title: "Appearance" }], Body: AppearancePage });
  registerSettingsPage({ id: "shortcuts", group: "System", label: "Shortcuts", icon: Keyboard, order: 2, sections: [{ id: "shortcuts", title: "Shortcuts" }], Body: ShortcutsPage });
  registerSettingsPage({ id: "status", group: "System", label: "Status", icon: Activity, order: 3, sections: [{ id: "status", title: "Status" }], Body: StatusPage });
}
