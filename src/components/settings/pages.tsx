/** The core settings pages. Modules register their own pages from their install function. */
import { SlidersHorizontal } from "lucide-react";
import { registerSettingsPage } from "../../registry/settingsPages";
import { GeneralPage } from "./GeneralPage";

/** Ids match the pages `settings:open:<page>` and `open_settings` name. */
export function registerCoreSettingsPages() {
  registerSettingsPage({ id: "general", group: "System", label: "General", icon: SlidersHorizontal, order: 0, sections: [{ id: "about", title: "About" }], Body: GeneralPage });
}
