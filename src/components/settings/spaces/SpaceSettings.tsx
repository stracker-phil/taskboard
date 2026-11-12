import { useQueryClient } from "@tanstack/react-query";
import { useContext, useState } from "react";
import { ChevronLeft, ChevronsUpDown } from "lucide-react";
import { api, type GroupBy, type Layout, type SpaceConfig, type SpaceLayouts, type SpaceNode } from "../../../api";
import { SPACE_CONFIG_KEY, useSpaceConfig } from "../../../queries/views";
import { GROUP_NAMES } from "../../../shell/toolbar/groupMenu";
import { fail } from "../../../state/store";
import { SpaceFields } from "../../SpaceEditor";
import { openMenu } from "../../menu";
import { SpaceLookButton } from "../../SpaceLookButton";
import { SegmentedControl } from "../../SegmentedControl";
import { Group, Row, ScrollTarget, Segmented, SwitchInput } from "../controls";
import { useSettings } from "../../../queries/settings";
import { attempt } from "../../../state/requests";
import { AccentPicker } from "../AccentPicker";
import { SpaceCustomFields } from "./SpaceCustomFields";
import { SpaceStatuses } from "./SpaceStatuses";
import { SpaceTeam } from "./SpaceTeam";

const LAYOUTS: Layout[] = ["list", "board", "table"];
const LAYOUT_NAMES: Record<Layout, string> = { list: "List", board: "Board", table: "Table" };
const GROUPS: GroupBy[] = ["status", "statusType", "assignee", "priority", "epic", "release", "due", "none"];
const TABS = ["general", "team", "statuses", "fields"] as const;
type Tab = (typeof TABS)[number];
const TAB_NAMES: Record<Tab, string> = { general: "General", team: "Team", statuses: "Statuses", fields: "Custom Fields" };

/** The tab holding the section a link or the settings search asked for. */
const tabOf = (section: string | undefined): Tab => TABS.find((tab) => tab === section) ?? "general";

/** One Space's settings page: in Settings → Spaces with a way back to the list, or alone in the main window's
 *  sheet (the sidebar's Space Settings…), without one. */
export function SpaceSettings({ space, onBack }: { space: SpaceNode; onBack?: () => void }) {
  const { data: config } = useSpaceConfig(space.projectId);
  const [editingLook, setEditingLook] = useState(false);
  const target = useContext(ScrollTarget)?.id;
  const [tab, setTab] = useState(() => tabOf(target));
  const [shownTarget, setShownTarget] = useState(target);
  if (target !== shownTarget) {
    setShownTarget(target);
    setTab(tabOf(target));
  }
  return (
    <>
      <div className="space-settings-head">
        {onBack && (
          <button type="button" className="btn space-settings-back" onClick={onBack}>
            <ChevronLeft size={14} aria-hidden="true" />
            Spaces
          </button>
        )}
        <SpaceLookButton space={space} size={22} className="space-settings-head__tile" open={editingLook} onOpenChange={setEditingLook} />
        <h3 className="space-settings-head__name">{space.name}</h3>
        <span className="mono settings-row-desc">{space.key}</span>
      </div>
      <div className="space-settings-tabs">
        <SegmentedControl label="Space settings" options={TABS} labels={TAB_NAMES} value={tab} onChange={setTab} />
      </div>
      <div key={tab} className="space-settings-tab">
        {tab === "general" && (
          <>
            <Group title="General">
              <SpaceFields space={space} />
              <SpaceAccent space={space} />
            </Group>
            {config && <LayoutsGroup config={config} />}
          </>
        )}
        {tab === "team" && <SpaceTeam projectId={space.projectId} />}
        {tab === "statuses" && <SpaceStatuses projectId={space.projectId} />}
        {tab === "fields" && config && <SpaceCustomFields config={config} />}
      </div>
    </>
  );
}

/** The Space's own accent, shown in the main window while the Space is; the first swatch follows the app's. */
function SpaceAccent({ space }: { space: SpaceNode }) {
  const { data: settings } = useSettings();
  return (
    <Row label="Accent color" description="Used while this Space is shown">
      <AccentPicker
        label="Accent color"
        value={space.accent}
        appAccent={settings?.accent ?? "system"}
        onChange={(accent) => void attempt(api.spaceSetAccent(space.projectId, accent), "Could not change the accent color")}
      />
    </Row>
  );
}

function useSave(projectId: string) {
  const client = useQueryClient();
  return (call: Promise<SpaceConfig>, failure: string) =>
    void call.then(
      (saved) => client.setQueryData([...SPACE_CONFIG_KEY, projectId], saved),
      (e: unknown) => fail(e, failure),
    );
}

function LayoutsGroup({ config }: { config: SpaceConfig }) {
  const save = useSave(config.projectId);
  const current = config.layouts;
  const set = (next: SpaceLayouts) => save(api.spaceSetLayouts(config.projectId, next), "Could not change the layouts");
  const toggle = (layout: Layout, on: boolean) => {
    const layouts = LAYOUTS.filter((l) => (l === layout ? on : current.layouts.includes(l)));
    if (layouts.length === 0) return;
    const defaultLayout = layouts.includes(current.defaultLayout) ? current.defaultLayout : layouts[0];
    set({ ...current, layouts, defaultLayout });
  };
  const pickGroup = (button: HTMLElement) =>
    openMenu(
      GROUPS.map((group) => ({ text: GROUP_NAMES[group], checked: group === current.defaultGroup, run: () => set({ ...current, defaultGroup: group }) })),
      button,
      "Grouping of new views",
    );

  return (
    <Group title="Layouts" description="The layouts this Space's views can show. A view in a layout you switch off shows the default layout until you turn it on again.">
      {LAYOUTS.map((layout) => {
        const on = current.layouts.includes(layout);
        const last = on && current.layouts.length === 1;
        return (
          <Row key={layout} label={LAYOUT_NAMES[layout]} description={last ? "A Space keeps at least one layout" : undefined}>
            <SwitchInput aria-label={`Offer ${LAYOUT_NAMES[layout]}`} checked={on} disabled={last} onChange={(next) => toggle(layout, next)} />
          </Row>
        );
      })}
      <Segmented
        label="Default layout"
        description="What a new view in this Space starts as"
        options={current.layouts}
        labels={LAYOUT_NAMES}
        value={current.defaultLayout}
        onChange={(defaultLayout) => set({ ...current, defaultLayout })}
      />
      <Row label="New views group by" description="Each view can change its own grouping">
        <button type="button" className="btn status-row__style" aria-label="Grouping of new views" aria-haspopup="menu" onClick={(e) => pickGroup(e.currentTarget)}>
          {GROUP_NAMES[current.defaultGroup]}
          <ChevronsUpDown size={12} aria-hidden="true" />
        </button>
      </Row>
    </Group>
  );
}
