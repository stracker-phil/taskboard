import { Plus } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { api } from "../api";
import { Disclosure } from "../components/Disclosure";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

/** The Web links section's "Add link" button. */
export function AddWebLinkButton({ open, onOpen }: { open: boolean; onOpen: () => void }) {
  return (
    <button type="button" className="btn web-links__add" aria-expanded={open} onClick={onOpen}>
      <Plus size={13} strokeWidth={2.2} aria-hidden="true" />
      Add link
    </button>
  );
}

/** Adds a web link (a pull request, a document) to the issue in Jira: the address, focused as it opens, and a title,
 *  which Rust fills in from a pull request or the address when left empty. ↩ adds, esc closes. It grows open. */
export function AddWebLinkForm({ issueId, open, onClose }: { issueId: string; open: boolean; onClose: () => void }) {
  const [url, setUrl] = useState("");
  const [title, setTitle] = useState("");
  const [sending, setSending] = useState(false);
  const address = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (open) address.current?.focus({ preventScroll: true });
  }, [open]);

  const close = () => {
    setUrl("");
    setTitle("");
    onClose();
  };
  const add = (e: FormEvent) => {
    e.preventDefault();
    if (url.trim() === "" || sending) return;
    setSending(true);
    void attempt(api.issueAddWebLink(issueId, url.trim(), title.trim() || null), "Could not add the link").then((outcome) => {
      setSending(false);
      if (!outcome) return;
      useStore.getState().toast("success", outcome.done);
      close();
    });
  };
  const onKeyDown = (e: KeyboardEvent) => {
    // Typing here never reaches the page's key commands.
    e.stopPropagation();
    if (e.key === "Escape") close();
  };

  return (
    <Disclosure open={open}>
      <form className="web-link-form" onSubmit={add} onKeyDown={onKeyDown}>
        <input
          ref={address}
          className="web-link-form__field"
          aria-label="Link address"
          placeholder="https://… (a pull request, a document)"
          value={url}
          disabled={sending}
          onChange={(e) => setUrl(e.target.value)}
        />
        <input
          className="web-link-form__field"
          aria-label="Link title"
          placeholder="Title (optional)"
          value={title}
          disabled={sending}
          onChange={(e) => setTitle(e.target.value)}
        />
        <div className="web-link-form__actions">
          <button type="button" className="btn" onClick={close}>
            Cancel
          </button>
          <button type="submit" className="btn btn-primary" disabled={url.trim() === "" || sending}>
            {sending ? "Adding…" : "Add Link"}
          </button>
        </div>
      </form>
    </Disclosure>
  );
}
