/** The core settings pages. Modules register their own pages from their install function. */
import { CircleDot, FolderKanban, Keyboard, LayoutGrid, Palette, PlugZap, RefreshCw, SlidersHorizontal } from "lucide-react";
import { registerSettingsPage } from "../../registry/settingsPages";
import { AppearancePage } from "./AppearancePage";
import { GeneralPage } from "./GeneralPage";
import { ShortcutsPage } from "./ShortcutsPage";
import { ConnectionPage } from "./jira/ConnectionPage";
import { ProjectsPage } from "./jira/ProjectsPage";
import { SpacesPage } from "./jira/SpacesPage";
import { StatusesPage } from "./jira/StatusesPage";
import { SyncPage } from "./jira/SyncPage";

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
    sections: [{ id: "spaces", title: "Spaces" }],
    Body: SpacesPage,
  });
  registerSettingsPage({
    id: "statuses",
    group: "Jira",
    label: "Statuses",
    icon: CircleDot,
    order: 3,
    sections: [{ id: "statuses", title: "Statuses" }],
    Body: StatusesPage,
  });
  registerSettingsPage({
    id: "sync",
    group: "Jira",
    label: "Sync",
    icon: RefreshCw,
    order: 4,
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
}
