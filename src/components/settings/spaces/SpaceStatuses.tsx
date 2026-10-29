import { useQueryClient } from "@tanstack/react-query";
import { ChevronsUpDown, Search } from "lucide-react";
import { useState } from "react";
import { api, type StatusStyleRow, type Style } from "../../../api";
import { STATUS_STYLES_KEY, useStatusStyles } from "../../../queries/views";
import { fail } from "../../../state/store";
import { StatusPill } from "../../issue/StatusPill";
import { Group } from "../controls";

const STYLE_NAMES: Record<Style, string> = {
  todo: "To do",
  progress: "In progress",
  review: "In review",
  blocked: "Blocked",
  done: "Done",
};
const STYLE_ORDER = Object.keys(STYLE_NAMES) as Style[];
const CATEGORY_NAMES: Record<string, string> = { new: "To do", indeterminate: "In progress", done: "Done" };
const CATEGORY_ORDER = ["new", "indeterminate", "done"];

/** A Space's statuses by Jira category, searchable, each with its issue count and its style in this Space. */
export function SpaceStatuses({ projectId }: { projectId: string }) {
  const client = useQueryClient();
  const { data: statuses = [] } = useStatusStyles(projectId);

  const choose = (status: StatusStyleRow, style: Style | null) =>
    void api.statusStyleSet(projectId, status.id, style).then(
      (rows) => client.setQueryData([...STATUS_STYLES_KEY, projectId], rows),
      (e: unknown) => fail(e, `Could not change ${status.name}`),
    );

  const openMenu = (status: StatusStyleRow) =>
    void api
      .popupMenu([
        { text: `Automatic (${STYLE_NAMES[status.detected]})`, run: () => choose(status, null) },
        "separator",
        ...STYLE_ORDER.map((style) => ({ text: STYLE_NAMES[style], run: () => choose(status, style) })),
      ])
      .catch((e: unknown) => fail(e, "Could not open the menu"));

  const [search, setSearch] = useState("");
  const needle = search.trim().toLowerCase();
  const shown = statuses.filter((status) => !needle || status.name.toLowerCase().includes(needle));
  const sections = CATEGORY_ORDER.map((category) => ({ category, rows: shown.filter((s) => s.category === category) })).filter((s) => s.rows.length > 0);

  return (
    <Group title="Statuses" description="The statuses this Space's issues are in, in the style each one shows in here. Automatic picks a style from the Jira category and the name.">
      {statuses.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Statuses appear after the first sync.</span>
        </div>
      ) : (
        <div className="settings-row status-search">
          <label className="columns-popover__search status-search__field">
            <Search size={13} strokeWidth={2} aria-hidden="true" />
            <input aria-label="Search statuses" placeholder="Search statuses" value={search} onChange={(e) => setSearch(e.target.value)} />
          </label>
        </div>
      )}
      {needle && shown.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">No status matches “{search.trim()}”.</span>
        </div>
      )}
      {sections.map(({ category, rows }) => (
        <section key={category} className="status-section" aria-label={CATEGORY_NAMES[category] ?? category}>
          <h3 className="status-section__title">{CATEGORY_NAMES[category] ?? category}</h3>
          {rows.map((status) => {
            const style = status.chosen ?? status.detected;
            return (
              <div key={status.id} className="settings-row status-row">
                <StatusPill style={style} name={status.name} />
                <span className="status-row__count">{status.issues === 1 ? "1 issue" : `${status.issues} issues`}</span>
                <span className="status-row__spacer" />
                <button type="button" className="btn status-row__style" aria-label={`Style of ${status.name}`} onClick={() => openMenu(status)}>
                  <span className={`status-square status-square--${style}`} aria-hidden="true" />
                  {status.chosen ? STYLE_NAMES[status.chosen] : `Automatic · ${STYLE_NAMES[status.detected]}`}
                  <ChevronsUpDown size={12} aria-hidden="true" />
                </button>
              </div>
            );
          })}
        </section>
      ))}
    </Group>
  );
}
