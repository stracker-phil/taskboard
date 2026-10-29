import type { Attachment } from "../api";
import { useAttachmentActions } from "./useAttachmentActions";

/** The issue's files as tiles; hidden when there are none. A click (or Space, as on any button) shows the file in
 *  Quick Look, which Rust downloads from the Jira site first; the browser opens only addresses on the connected site. */
export function Attachments({ issueId, attachments }: { issueId: string; attachments: Attachment[] }) {
  if (attachments.length === 0) return null;
  return (
    <section className="attachments" aria-label="Attachments">
      <h2>Attachments</h2>
      <div className="attachments__grid">
        {attachments.map((attachment) => (
          <AttachmentTile key={attachment.id} issueId={issueId} attachment={attachment} />
        ))}
      </div>
    </section>
  );
}

function AttachmentTile({ issueId, attachment }: { issueId: string; attachment: Attachment }) {
  const { loading, preview, menu } = useAttachmentActions(issueId, attachment);
  const { filename } = attachment;
  const meta = [attachment.size, attachment.authorName].filter(Boolean).join(" · ");
  return (
    <button
      type="button"
      className="attachment"
      title={filename}
      aria-busy={loading}
      onClick={preview}
      onContextMenu={menu}
    >
      <span className="attachment__badge" aria-hidden="true">
        {loading ? <span className="attachment__spinner" /> : attachment.badge}
      </span>
      <span className="attachment__text">
        <span className="attachment__name">{filename}</span>
        {meta && <span className="attachment__meta">{loading ? "Loading…" : meta}</span>}
      </span>
    </button>
  );
}
