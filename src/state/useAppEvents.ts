import { useEffect } from "react";
import { api, toAppError, type LaunchEvent } from "../api";
import { getCommand } from "../commands/registry";
import { useStore } from "./store";

function handleLaunchEvents(events: LaunchEvent[]) {
  const { toast } = useStore.getState();
  for (const event of events) {
    if (event.type === "error") toast("error", event.message);
  }
}

/** A menu item's page command runs when it can now (a menu can't know the page's state). */
function runPageCommand(id: string) {
  const command = getCommand(id);
  if (command && (command.when?.() ?? true)) void command.run();
}

/** The main window's state from Rust: loads the window layout, takes the launch requests (now and on `launch`),
 *  follows `toggle_sidebar` (the View menu and the `view:toggleSidebar` bus command) and runs the menus' page
 *  commands (`page_command`). */
export function useAppEvents() {
  useEffect(() => {
    const { loadUiState, toggleSidebar, toast } = useStore.getState();
    const drainLaunches = () => api.takeLaunchEvents().then(handleLaunchEvents, (e) => toast("error", `Launch request failed: ${toAppError(e).message}`));
    void loadUiState();
    void drainLaunches();
    const unlisten = [
      api.listen("launch", () => void drainLaunches()),
      api.listen("toggle_sidebar", toggleSidebar),
      api.listen("page_command", runPageCommand),
    ];
    return () => unlisten.forEach((p) => void p.then((fn) => fn()).catch(() => {}));
  }, []);
}
