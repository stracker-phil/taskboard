import { useQueryClient } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, ChevronUp } from "lucide-react";
import { useContext, useEffect, useState } from "react";
import { api, type SpaceNode } from "../../../api";
import { fail } from "../../../state/store";
import { SIDEBAR_KEY, useSpaces } from "../../../queries/sidebar";
import { SpaceTile } from "../../SpaceTile";
import { SPACE_TARGET } from "../../../registry/settingsPages";
import { Group, SectionTarget } from "../controls";
import { SpaceSettings } from "../spaces/SpaceSettings";

/** The tracked Spaces in sidebar order: a row each to open its settings and move it. `space:<projectId>` opens one
 *  Space's settings directly (its sidebar menu's Space Settings…). */
export function SpacesPage() {
  const client = useQueryClient();
  const { data: spaces = [] } = useSpaces();
  const target = useContext(SectionTarget);
  const [open, setOpen] = useState<string | null>(null);
  useEffect(() => {
    if (target?.startsWith(SPACE_TARGET)) setOpen(target.slice(SPACE_TARGET.length));
  }, [target]);
  const opened = spaces.find((space) => space.projectId === open);
  if (opened) return <SpaceSettings space={opened} onBack={() => setOpen(null)} />;

  const move = (space: SpaceNode, position: number) =>
    void api.spaceMove(space.projectId, position).then(
      () => void client.invalidateQueries({ queryKey: SIDEBAR_KEY }),
      (e: unknown) => fail(e, `Could not move ${space.name}`),
    );

  return (
    <Group title="Spaces" anchor="spaces" description="Each tracked project is a Space in the sidebar.">
      {spaces.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">Track projects under Projects to get Spaces.</span>
        </div>
      )}
      {spaces.map((space, index) => {
        return (
          <div key={space.projectId} className="space-settings-item">
            <div className="settings-row">
              <button
                type="button"
                className="space-settings-toggle"
                aria-label={`${space.name} settings`}
                onClick={() => setOpen(space.projectId)}
              >
                <ChevronRight size={14} className="space-settings-chevron" aria-hidden="true" />
                <SpaceTile space={space} size={22} />
                <span className="settings-row-text">
                  <span className="settings-row-label">{space.name}</span>
                  <span className="settings-row-desc">
                    <span className="mono">{space.key}</span>
                    {space.client ? ` · ${space.client}` : ""}
                  </span>
                </span>
              </button>
              <div className="settings-row-control">
                <button type="button" className="icon-btn" aria-label={`Move ${space.name} up`} disabled={index === 0} onClick={() => move(space, index - 1)}>
                  <ChevronUp size={14} />
                </button>
                <button
                  type="button"
                  className="icon-btn"
                  aria-label={`Move ${space.name} down`}
                  disabled={index === spaces.length - 1}
                  onClick={() => move(space, index + 1)}
                >
                  <ChevronDown size={14} />
                </button>
              </div>
            </div>
          </div>
        );
      })}
    </Group>
  );
}
