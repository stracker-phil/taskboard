import { useEffect, useRef, useState, useSyncExternalStore, type KeyboardEvent } from "react";
import { api, type IssueDetail } from "../api";
import { attempt } from "../state/requests";
import { showOutcome } from "./runWrite";
import { useJiraChange } from "./useJiraChanges";

/** How long a rename asked for by key waits for its issue's title to show (the Preview opening). */
const REQUEST_TTL_MS = 2000;

let request: { issueId: string; at: number } | null = null;
const listeners = new Set<() => void>();

/** Starts renaming the issue whose title shows now, or shows within a moment (F2 on a row opens its Preview). */
export function requestRename(issueId: string) {
  request = { issueId, at: Date.now() };
  listeners.forEach((listener) => listener());
}

function useRenameRequest(issueId: string): boolean {
  const asked = useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => request,
  );
  return asked !== null && asked.issueId === issueId && Date.now() - asked.at < REQUEST_TTL_MS;
}

/** Mount with `key={issue.id}`, so an edit or a sent title never carries over to another issue. */
interface Props {
  issue: IssueDetail;
  as: "h1" | "h2";
  className: string;
}

/** An issue's title. While title changes are on, a double-click or F2 edits it where it stands: the heading itself
 *  becomes editable, so nothing moves. Enter or leaving it saves, Escape cancels. The new title shows until Jira's
 *  answer replaces it; a refusal puts the old one back. */
export function EditableTitle({ issue, as: Heading, className }: Props) {
  const allowed = useJiraChange("changeTitle") && !issue.gone;
  const [editing, setEditing] = useState(false);
  const [sent, setSent] = useState<{ text: string; before: string } | null>(null);
  const asked = useRenameRequest(issue.id);
  const ref = useRef<HTMLHeadingElement>(null);
  // Enter saves and ends the edit; the blur that may follow mustn't save again.
  const finished = useRef(false);
  const shown = sent && issue.summary === sent.before ? sent.text : issue.summary;
  // Once Jira's title arrives the sent one is done; a later change back mustn't bring it up again.
  useEffect(() => {
    if (sent && issue.summary !== sent.before) setSent(null);
  }, [sent, issue.summary]);

  useEffect(() => {
    if (!asked || !allowed) return;
    request = null;
    setEditing(true);
  }, [asked, allowed]);

  useEffect(() => {
    const element = ref.current;
    if (!editing || !element) return;
    finished.current = false;
    element.focus();
    const range = document.createRange();
    range.selectNodeContents(element);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  }, [editing]);

  const finish = (save: boolean) => {
    if (finished.current) return;
    finished.current = true;
    const text = (ref.current?.textContent ?? "").replace(/\s+/g, " ").trim();
    setEditing(false);
    if (!save || text === "" || text === shown) return;
    const before = issue.summary;
    setSent({ text, before });
    void attempt(api.issueWrite(issue.id, { kind: "title", text }), "Could not change the title").then((outcome) => {
      if (outcome) showOutcome(issue.id, outcome);
      else setSent(null);
    });
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      finish(true);
    } else if (e.key === "Escape") {
      e.preventDefault();
      finish(false);
    }
  };

  const classes = [className, issue.gone && "issue-title--gone", allowed && "issue-title--editable", editing && "issue-title--editing"].filter(Boolean).join(" ");
  if (!editing) {
    return (
      <Heading key="shown" className={classes} onDoubleClick={allowed ? () => setEditing(true) : undefined}>
        {shown}
      </Heading>
    );
  }
  return (
    <Heading
      key="editing"
      ref={ref}
      className={classes}
      contentEditable="plaintext-only"
      suppressContentEditableWarning
      role="textbox"
      aria-label={`Title of ${issue.key}`}
      spellCheck
      onKeyDown={onKeyDown}
      // Leaving the app keeps the edit open; only a blur inside the window saves it.
      onBlur={() => {
        if (document.hasFocus()) finish(true);
      }}
    >
      {shown}
    </Heading>
  );
}
