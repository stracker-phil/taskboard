import { Modal } from "../components/Modal";
import { SpaceSettings } from "../components/settings/spaces/SpaceSettings";
import { useSidebar } from "../queries/sidebar";
import { useStore } from "../state/store";

/** A Space's settings over the main window (its sidebar menu's Space Settings…): the same page as Settings → Spaces,
 *  in the Settings window's look. Settings → Spaces still lists every Space; both edit the same settings. */
export function SpaceSheet() {
  const projectId = useStore((s) => s.spaceSheet);
  const close = useStore((s) => s.setSpaceSheet);
  const { data: tree } = useSidebar();
  const space = tree?.spaces.find((s) => s.projectId === projectId);
  if (!space) return null;
  return (
    <Modal onClose={() => close(null)} labelledBy="space-sheet-title" className="space-sheet">
      <div className="settings-pane">
        <header>
          <h2 id="space-sheet-title">Space Settings</h2>
          <span className="space-sheet__spacer" />
          <button type="button" className="btn btn-primary" onClick={() => close(null)}>
            Done
          </button>
        </header>
        <div className="settings-body scroller">
          <SpaceSettings space={space} />
        </div>
      </div>
    </Modal>
  );
}
