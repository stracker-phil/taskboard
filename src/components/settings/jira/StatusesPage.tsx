import { useQueryClient } from "@tanstack/react-query";
import { ChevronsUpDown } from "lucide-react";
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

/** Every cached Jira status with the style it shows in; Automatic follows its category and name. */
export function StatusesPage() {
  const client = useQueryClient();
  const { data: statuses = [] } = useStatusStyles();

  const choose = (status: StatusStyleRow, style: Style | null) =>
    void api.statusStyleSet(status.id, style).then(
      (rows) => client.setQueryData(STATUS_STYLES_KEY, rows),
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

  return (
    <Group title="Statuses" anchor="statuses" description="Every Jira status shows in one of five styles. Automatic picks one from the status's category and name.">
      {statuses.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">Statuses appear after the first sync.</span>
        </div>
      )}
      {statuses.map((status) => {
        const style = status.chosen ?? status.detected;
        return (
          <div key={status.id} className="settings-row">
            <div className="settings-row-text">
              <StatusPill style={style} name={status.name} />
              <div className="settings-row-desc">Jira category: {CATEGORY_NAMES[status.category] ?? status.category}</div>
            </div>
            <div className="settings-row-control">
              <button type="button" className="btn" aria-label={`Style of ${status.name}`} onClick={() => openMenu(status)}>
                {status.chosen ? STYLE_NAMES[status.chosen] : "Automatic"}
                <ChevronsUpDown size={12} aria-hidden="true" />
              </button>
            </div>
          </div>
        );
      })}
    </Group>
  );
}
