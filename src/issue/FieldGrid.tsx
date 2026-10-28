import { CircleDot, Flag, RotateCw, Tag, User, Zap, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import type { Cell, IssueDetail } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { PriorityFlag } from "../components/issue/PriorityFlag";
import { StatusPill } from "../components/issue/StatusPill";

function Field({ icon: Icon, label, children }: { icon?: LucideIcon; label: string; children: ReactNode }) {
  return (
    <div className="field-grid__item">
      <dt>
        {Icon && <Icon size={15} strokeWidth={1.8} aria-hidden="true" />}
        {label}
      </dt>
      <dd>{children}</dd>
    </div>
  );
}

const NONE = <span className="field-grid__none">None</span>;

function CellValue({ cell }: { cell: Cell | null }) {
  if (!cell) return NONE;
  if (cell.kind === "person")
    return (
      <>
        <Avatar person={cell.value} size={24} />
        {cell.value.name}
      </>
    );
  return <>{cell.value}</>;
}

/** Status, Assignee, Epic, Release, Priority, Sprint and the custom fields the owner shows. The epic chip opens
 *  the epic in this view. */
export function FieldGrid({ issue, onOpenEpic }: { issue: IssueDetail; onOpenEpic: (id: string) => void }) {
  return (
    <dl className="field-grid">
      <Field icon={CircleDot} label="Status">
        <StatusPill style={issue.style} name={issue.statusName} />
      </Field>
      <Field icon={User} label="Assignee">
        {issue.assignees.length > 0
          ? issue.assignees.map((person) => (
              <span key={person.accountId} className="field-grid__person">
                <Avatar person={person} size={24} />
                {person.name}
              </span>
            ))
          : NONE}
      </Field>
      {!issue.isEpic && (
        <Field icon={Zap} label="Epic">
          {issue.epic ? (
            <button type="button" className={`epic-chip epic-chip--detail epic-chip--${issue.epic.color}`} onClick={() => onOpenEpic(issue.epic!.id)}>
              <Zap size={12} strokeWidth={2.4} aria-hidden="true" />
              {issue.epic.name}
            </button>
          ) : (
            NONE
          )}
        </Field>
      )}
      <Field icon={Tag} label="Release">
        {issue.releases.length > 0 ? issue.releases.map((release) => <span key={release} className="release-chip">{release}</span>) : NONE}
      </Field>
      <Field icon={Flag} label="Priority">
        {issue.priority ? (
          <>
            <PriorityFlag level={issue.priority.level} />
            {issue.priority.name}
          </>
        ) : (
          NONE
        )}
      </Field>
      {!issue.isEpic && (
        <Field icon={RotateCw} label="Sprint">
          {issue.sprint ?? NONE}
        </Field>
      )}
      {issue.custom.map((field) => (
        <Field key={field.id} label={field.name}>
          <CellValue cell={field.value} />
        </Field>
      ))}
    </dl>
  );
}
