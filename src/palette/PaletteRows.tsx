import { Check, CornerDownLeft, Zap } from "lucide-react";
import type { IssueHit, ListHit, SidebarTree, TextPart } from "../api";
import { SpaceTile } from "../components/SpaceTile";
import { StatusPill } from "../components/issue/StatusPill";
import { StatusSquare } from "../components/issue/StatusSquare";
import type { PaletteCommand } from "./commands";
import { PREFIXES, type Prefix } from "./prefixes";

function Marked({ parts }: { parts: TextPart[] }) {
  return (
    <>
      {parts.map((part, i) => (part.strong ? <strong key={i}>{part.text}</strong> : <span key={i}>{part.text}</span>))}
    </>
  );
}

export function IssueRow({ hit }: { hit: IssueHit }) {
  return (
    <>
      <span className="palette__issue-line">
        <StatusSquare style={hit.style} />
        <span className="palette__key">{hit.key}</span>
        <span className="palette__summary">
          <Marked parts={hit.summary} />
        </span>
        {hit.gone && <span className="palette__gone">gone</span>}
        <StatusPill style={hit.style} name={hit.statusName} variant="activity" />
        <CornerDownLeft className="palette__enter" size={12} strokeWidth={2} aria-hidden="true" />
      </span>
      {hit.comment && (
        <span className="palette__comment">
          Matched in comment: “<Marked parts={hit.comment} />”
        </span>
      )}
    </>
  );
}

export function ListRow({ hit, tree }: { hit: ListHit; tree: SidebarTree | undefined }) {
  const space = tree?.spaces.find((s) => s.projectId === hit.projectId);
  return (
    <>
      {space ? <SpaceTile space={space} size={20} /> : <span className="palette__tile-gap" />}
      <span className="palette__summary">{space && !hit.isSpace ? `${space.name} › ${hit.name}` : hit.name}</span>
      <span className="palette__meta">{hit.isSpace ? "Space" : "filter list"}</span>
    </>
  );
}

/** `meta` is its detail, else its section where the rows are not grouped by section. */
export function CommandRow({ command, showSection }: { command: PaletteCommand; showSection: boolean }) {
  const meta = command.detail ?? (showSection ? command.section : undefined);
  const Icon = command.checked ? Check : Zap;
  return (
    <>
      <Icon size={16} strokeWidth={command.checked ? 2.2 : 1.8} aria-label={command.checked ? "On" : undefined} aria-hidden={command.checked ? undefined : true} />
      <span className="palette__summary">{command.label}</span>
      {meta && <span className="palette__meta">{meta}</span>}
      {command.shortcut && <kbd className="palette__shortcut">{command.shortcut}</kbd>}
    </>
  );
}

export function PrefixRow({ prefix }: { prefix: Prefix }) {
  return (
    <>
      <kbd className="palette__prefix-key">{prefix.char}</kbd>
      <span className="palette__summary">{prefix.label}</span>
      <span className="palette__meta">{prefix.placeholder}</span>
    </>
  );
}

/** One button per prefix; the active one clears it again. */
export function PrefixButtons({ active, onPick }: { active: Prefix | null; onPick: (text: string) => void }) {
  return PREFIXES.map((prefix) => {
    const on = active?.char === prefix.char;
    return (
      <button
        key={prefix.char}
        type="button"
        tabIndex={-1}
        className={on ? "palette__prefix palette__prefix--on" : "palette__prefix"}
        aria-pressed={on}
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => onPick(on ? "" : prefix.char)}
      >
        <kbd className="palette__prefix-key">{prefix.char}</kbd>
        {prefix.label}
      </button>
    );
  });
}
