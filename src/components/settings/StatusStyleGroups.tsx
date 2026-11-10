import { ChevronsUpDown } from "lucide-react";
import { type ReactNode, useRef, useState } from "react";
import type { Style } from "../../api";
import { useListGlide } from "../../lib/motion";
import { StatusPill } from "../issue/StatusPill";
import { StatusSquare } from "../issue/StatusSquare";
import { STYLE_NAMES, STYLE_ORDER } from "../issue/styleNames";
import { openMenu } from "../menu";
import { SearchField } from "./SearchField";

/** Progress first, done last, like a list's status groups (`STYLE_ORDER` follows the way work moves). */
const GROUP_ORDER: Style[] = ["progress", "review", "blocked", "todo", "backlog", "done"];

export interface StyledStatus {
  id: string;
  name: string;
  /** The style the status shows: the section it sits in. */
  style: Style;
  chosen: Style | null;
  /** What no choice means here, e.g. "Automatic" and the detected style. */
  fallback: { word: string; style: Style };
  /** Beside the pill, e.g. the issue count. */
  note?: ReactNode;
}

/** Statuses in sections by the style they show, searchable, each with a menu to choose its style. */
export function StatusStyleGroups({ statuses, choose }: { statuses: StyledStatus[]; choose: (status: StyledStatus, style: Style | null) => void }) {
  const [search, setSearch] = useState("");
  const list = useRef<HTMLDivElement>(null);
  useListGlide(list);
  const needle = search.trim().toLowerCase();
  const shown = statuses.filter((status) => !needle || status.name.toLowerCase().includes(needle));
  const sections = GROUP_ORDER.map((style) => ({ style, rows: shown.filter((s) => s.style === style) })).filter((s) => s.rows.length > 0);

  const pickStyle = (status: StyledStatus, button: HTMLElement) =>
    openMenu(
      [
        { text: `${status.fallback.word} (${STYLE_NAMES[status.fallback.style]})`, checked: status.chosen === null, run: () => choose(status, null) },
        "separator",
        ...STYLE_ORDER.map((style) => ({
          text: STYLE_NAMES[style],
          icon: <StatusSquare style={style} />,
          checked: status.chosen === style,
          run: () => choose(status, style),
        })),
      ],
      button,
      `Style of ${status.name}`,
    );

  return (
    <>
      <SearchField label="Search statuses" value={search} onChange={setSearch} />
      {needle && shown.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">No status matches “{search.trim()}”.</span>
        </div>
      )}
      <div ref={list}>
        {sections.map(({ style, rows }) => (
          <section key={style} className="status-section" aria-label={STYLE_NAMES[style]}>
            <h3 className="status-section__title">
              <StatusSquare style={style} />
              {STYLE_NAMES[style]}
            </h3>
            {rows.map((status) => (
              <div key={status.id} data-glide={status.id} className="settings-row status-row">
                <StatusPill style={status.style} name={status.name} />
                {status.note}
                <span className="status-row__spacer" />
                <button type="button" className="btn status-row__style" aria-label={`Style of ${status.name}`} aria-haspopup="menu" onClick={(e) => pickStyle(status, e.currentTarget)}>
                  <StatusSquare style={status.style} />
                  {status.chosen ? STYLE_NAMES[status.chosen] : `${status.fallback.word} · ${STYLE_NAMES[status.fallback.style]}`}
                  <ChevronsUpDown size={12} aria-hidden="true" />
                </button>
              </div>
            ))}
          </section>
        ))}
      </div>
    </>
  );
}
