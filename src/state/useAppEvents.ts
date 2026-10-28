import { useEffect } from "react";
import { api, toAppError, type LaunchEvent } from "../api";
import { useStore } from "./store";

function handleLaunchEvents(events: LaunchEvent[]) {
  const { toast } = useStore.getState();
  for (const event of events) {
    if (event.type === "error") toast("error", event.message);
  }
}

/** The main window's state from Rust: loads the window layout, takes the launch requests (now and on `launch`)
 *  and follows `toggle_sidebar` (the View menu and the `view:toggleSidebar` bus command). */
export function useAppEvents() {
  useEffect(() => {
    const { loadUiState, toggleSidebar, toast } = useStore.getState();
    const drainLaunches = () => api.takeLaunchEvents().then(handleLaunchEvents, (e) => toast("error", `Launch request failed: ${toAppError(e).message}`));
    void loadUiState();
    void drainLaunches();
    const unlisten = [api.listen("launch", () => void drainLaunches()), api.listen("toggle_sidebar", toggleSidebar)];
    return () => unlisten.forEach((p) => void p.then((fn) => fn()).catch(() => {}));
  }, []);
}
