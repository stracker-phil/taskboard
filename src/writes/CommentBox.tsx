import { useRef, useState, type KeyboardEvent } from "react";
import { flushSync } from "react-dom";
import { api, type Person } from "../api";
import { useAutoGrow } from "../lib/useAutoGrow";
import { useIssueMentionable } from "../queries/issues";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { insertMention, matching, mentionAt } from "./mentions";
import { MentionList } from "./MentionList";
import { useJiraChange } from "./useJiraChanges";

/** Absent while changes to Jira, or comments, are off. */
export function CommentBox({ issueId }: { issueId: string }) {
  const jiraChanges = useJiraChange("comment");
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const input = useRef<HTMLTextAreaElement>(null);
  const mention = useMentionPicker(issueId, jiraChanges, text);
  useAutoGrow(input, text);
  if (!jiraChanges) return null;
  const empty = text.trim() === "";
  const send = () => {
    if (empty || sending) return;
    setSending(true);
    void attempt(api.issueComment(issueId, text), "Could not add the comment").then((outcome) => {
      if (!outcome) return setSending(false);
      // Re-enabled first: a disabled field ignores blur(), and blurring hands the keys back to the Issue view.
      flushSync(() => {
        setSending(false);
        setText("");
      });
      input.current?.blur();
      useStore.getState().toast("success", outcome.done);
    });
  };
  const pick = (person: Person) => {
    const field = input.current;
    if (!field || !mention.open) return;
    const next = insertMention(text, mention.caret, mention.open, person);
    flushSync(() => setText(next.text));
    field.setSelectionRange(next.caret, next.caret);
    mention.moved(next.text, next.caret);
  };
  const activeId = mention.open ? `mention-${mention.open.people[mention.active].accountId}` : undefined;
  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    // Typing here never reaches the page's bubbling key listeners (the hover peek).
    e.stopPropagation();
    if (e.key === "Enter" && e.metaKey) {
      e.preventDefault();
      send();
    } else if (mention.open && mention.keyDown(e.key, pick)) {
      e.preventDefault();
    } else if (e.key === "Escape") {
      e.currentTarget.blur();
    }
  };
  return (
    <form
      className="comment-box"
      onSubmit={(e) => {
        e.preventDefault();
        send();
      }}
    >
      <textarea
        ref={input}
        className="comment-box__input"
        aria-label="Add a comment"
        placeholder="Add a comment…"
        rows={2}
        value={text}
        disabled={sending}
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={!!mention.open}
        aria-controls={mention.open ? "mention-list" : undefined}
        aria-activedescendant={activeId}
        onChange={(e) => {
          setText(e.target.value);
          mention.moved(e.target.value, e.target.selectionStart);
        }}
        onSelect={(e) => mention.moved(e.currentTarget.value, e.currentTarget.selectionStart)}
        onFocus={() => mention.setFocused(true)}
        onBlur={() => mention.setFocused(false)}
        onKeyDown={onKeyDown}
      />
      {mention.open && (
        <MentionList
          people={mention.open.people}
          active={mention.active}
          input={input}
          mentionStart={mention.open.start}
          onHover={mention.setActive}
          onPick={pick}
        />
      )}
      <div className="comment-box__actions">
        <span className="comment-box__hint">Markdown · @ mentions · ⌘↩ to send</span>
        <button type="submit" className="btn btn-primary" disabled={empty || sending}>
          {sending ? "Sending…" : "Comment"}
        </button>
      </div>
    </form>
  );
}

/** The team members matching the `@name` before the caret, while the field has focus. Escape hides them until
 *  the caret leaves that `@name`. */
function useMentionPicker(issueId: string, enabled: boolean, text: string) {
  const { data: team = [] } = useIssueMentionable(issueId, enabled);
  const [caret, setCaret] = useState(0);
  const [focused, setFocused] = useState(false);
  const [dismissed, setDismissed] = useState<number | null>(null);
  const [chosen, setActive] = useState(0);
  const query = focused ? mentionAt(text, caret) : null;
  const people = query ? matching(team, query.query) : [];
  const open = query && people.length > 0 && dismissed !== query.start ? { ...query, people } : null;
  const active = Math.min(chosen, Math.max(people.length - 1, 0));

  const keyDown = (key: string, pick: (person: Person) => void): boolean => {
    if (!open) return false;
    if (key === "ArrowDown" || key === "ArrowUp") {
      const step = key === "ArrowDown" ? 1 : -1;
      setActive((active + step + people.length) % people.length);
    } else if (key === "Enter" || key === "Tab") {
      pick(people[active]);
    } else if (key === "Escape") {
      setDismissed(open.start);
    } else {
      return false;
    }
    return true;
  };

  return {
    open,
    active,
    caret,
    setActive,
    setFocused,
    keyDown,
    moved: (to: string, at: number) => {
      setCaret(at);
      setActive(0);
      if (!mentionAt(to, at)) setDismissed(null);
    },
  };
}
