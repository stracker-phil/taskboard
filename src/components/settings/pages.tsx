/** The core settings pages. Modules register their own pages from their install function. */
import { PlugZap, RefreshCw, SlidersHorizontal } from "lucide-react";
import { registerSettingsPage } from "../../registry/settingsPages";
import { GeneralPage } from "./GeneralPage";
import { ConnectionPage } from "./jira/ConnectionPage";
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
      { id: "projects", title: "Projects" },
    ],
    Body: ConnectionPage,
  });
  registerSettingsPage({
    id: "sync",
    group: "Jira",
    label: "Sync",
    icon: RefreshCw,
    order: 1,
    sections: [
      { id: "scope", title: "Scope" },
      { id: "intervals", title: "Intervals" },
      { id: "cache", title: "Cache" },
    ],
    Body: SyncPage,
  });
  registerSettingsPage({ id: "general", group: "System", label: "General", icon: SlidersHorizontal, order: 0, sections: [{ id: "about", title: "About" }], Body: GeneralPage });
}
