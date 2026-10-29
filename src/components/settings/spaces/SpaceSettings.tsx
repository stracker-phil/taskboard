import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { ChevronLeft, ChevronsUpDown } from "lucide-react";
import { api, type FieldSetting, type FieldVisibility, type GroupBy, type Layout, type SpaceConfig, type SpaceLayouts, type SpaceNode } from "../../../api";
import { SPACE_CONFIG_KEY, useSpaceConfig } from "../../../queries/views";
import { GROUP_NAMES } from "../../../shell/toolbar/groupMenu";
import { fail } from "../../../state/store";
import { SpaceFields } from "../../SpaceEditor";
import { openMenu } from "../../menu";
import { SpaceLookButton } from "../../SpaceLookButton";
import { Group, Row, Segmented, SwitchInput } from "../controls";
import { SpaceStatuses } from "./SpaceStatuses";

const LAYOUTS: Layout[] = ["list", "board", "table"];
const LAYOUT_NAMES: Record<Layout, string> = { list: "List", board: "Board", table: "Table" };
const GROUPS: GroupBy[] = ["status", "statusType", "assignee", "priority", "epic", "release", "due", "none"];
const VISIBILITY_NAMES: Record<FieldVisibility, string> = {
  visible: "Visible",
  hiddenInList: "Hidden in list",
  hiddenInDetail: "Hidden in detail",
  hidden: "Hidden everywhere",
};
const VISIBILITY_HELP: Record<FieldVisibility, string> = {
  visible: "A column choice, and shown in the Issue view when filled",
  hiddenInList: "Shown in the Issue view, never offered as a column",
  hiddenInDetail: "A column choice, left out of the Issue view",
  hidden: "Neither a column nor in the Issue view",
};
const VISIBILITIES = Object.keys(VISIBILITY_NAMES) as FieldVisibility[];

/** One Space's settings page, opened from the Spaces list or the sidebar's Space Settings…. */
export function SpaceSettings({ space, onBack }: { space: SpaceNode; onBack: () => void }) {
  const { data: config } = useSpaceConfig(space.projectId);
  const [editingLook, setEditingLook] = useState(false);
  return (
    <>
      <div className="space-settings-head">
        <button type="button" className="btn space-settings-back" onClick={onBack}>
          <ChevronLeft size={14} aria-hidden="true" />
          Spaces
        </button>
        <SpaceLookButton space={space} size={22} className="space-settings-head__tile" open={editingLook} onOpenChange={setEditingLook} />
        <h3 className="space-settings-head__name">{space.name}</h3>
        <span className="mono settings-row-desc">{space.key}</span>
      </div>
      <Group title="General">
        <SpaceFields space={space} />
      </Group>
      {config && <LayoutsGroup config={config} />}
      <SpaceStatuses projectId={space.projectId} />
      {config && <FieldsGroup config={config} />}
    </>
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
    <Group title="Layouts" anchor="layouts" description="The layouts this Space's views can show. A view in a layout you switch off shows the default layout until you turn it on again.">
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

function FieldsGroup({ config }: { config: SpaceConfig }) {
  const save = useSave(config.projectId);
  const choose = (field: FieldSetting, button: HTMLElement) =>
    openMenu(
      VISIBILITIES.map((visibility) => ({
        text: VISIBILITY_NAMES[visibility],
        checked: visibility === field.visibility,
        run: () => save(api.spaceSetField(config.projectId, field.id, visibility), `Could not change ${field.name}`),
      })),
      button,
      `Visibility of ${field.name}`,
    );

  return (
    <Group title="Custom fields" anchor="fields" description="Where each custom field shows in this Space: as a column choice in its views, in the Issue view, both or neither.">
      {config.fields.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">Custom fields appear after the first sync.</span>
        </div>
      )}
      {config.fields.map((field) => (
        <Row key={field.id} label={field.name} description={VISIBILITY_HELP[field.visibility]}>
          <button type="button" className="btn status-row__style" aria-label={`Visibility of ${field.name}`} aria-haspopup="menu" onClick={(e) => choose(field, e.currentTarget)}>
            {VISIBILITY_NAMES[field.visibility]}
            <ChevronsUpDown size={12} aria-hidden="true" />
          </button>
        </Row>
      ))}
    </Group>
  );
}
