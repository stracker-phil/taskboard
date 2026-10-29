import { useEffect } from "react";
import { api, toAppError, type LaunchEvent, type Location } from "../api";
import { getCommand } from "../commands/registry";
import { narrowingKey } from "./slices/narrowing";
import { useStore } from "./store";

/** The full Issue view over its origin, with its row selected there, as the palette opens it. */
async function openIssue(id: string, origin: Location) {
  const { loadPlace, updatePlace, openIssue } = useStore.getState();
  await loadPlace(origin.listId, origin.viewId);
  updatePlace(origin.listId, origin.viewId, { selectedId: id, peekOpen: false });
  openIssue(id, origin);
}

/** Sets the current view's Me chip, or flips it when `on` is null. */
async function onlyMine(on: boolean | null) {
  const { location } = await api.navCurrent();
  if (!location) return;
  const key = narrowingKey(location.listId, location.viewId);
  const { narrowings, setNarrowing } = useStore.getState();
  setNarrowing(key, { me: on ?? !(narrowings[key]?.me ?? false) });
}

async function handleLaunchEvents(events: LaunchEvent[]) {
  const { toast } = useStore.getState();
  for (const event of events) {
    if (event.type === "error") toast("error", event.message);
    if (event.type === "openIssue") await openIssue(event.id, event.origin);
    if (event.type === "onlyMine") await onlyMine(event.on);
  }
}

/** A menu item's page command runs when it can now (a menu can't know the page's state). */
function runPageCommand(id: string) {
  const command = getCommand(id);
  if (command && (command.when?.() ?? true)) void command.run();
}

/** The main window's state from Rust: loads the window layout, takes the launch requests (now and on `launch`:
 *  errors, and the command line's `open` and `me`),
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
