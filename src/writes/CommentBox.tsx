import { useRef, useState, type KeyboardEvent } from "react";
import { flushSync } from "react-dom";
import { api } from "../api";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { useJiraChange } from "./useJiraChanges";

/** Absent while changes to Jira, or comments, are off. */
export function CommentBox({ issueId }: { issueId: string }) {
  const jiraChanges = useJiraChange("comment");
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const input = useRef<HTMLTextAreaElement>(null);
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
  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    // Typing here never reaches the page's bubbling key listeners (the hover peek).
    e.stopPropagation();
    if (e.key === "Enter" && e.metaKey) {
      e.preventDefault();
      send();
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
        rows={3}
        value={text}
        disabled={sending}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={onKeyDown}
      />
      <div className="comment-box__actions">
        <span className="comment-box__hint">⌘↩ to send</span>
        <button type="submit" className="btn btn-primary" disabled={empty || sending}>
          {sending ? "Sending…" : "Comment"}
        </button>
      </div>
    </form>
  );
}
