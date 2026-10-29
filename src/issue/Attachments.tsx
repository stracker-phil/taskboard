import { useState, type MouseEvent } from "react";
import { api, type Attachment } from "../api";
import { openContextMenu } from "../components/menu";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

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
  const toast = useStore((s) => s.toast);
  const [loading, setLoading] = useState(false);
  const { url, filename } = attachment;
  const meta = [attachment.size, attachment.authorName].filter(Boolean).join(" · ");
  const preview = () => {
    if (loading) return;
    setLoading(true);
    void attempt(api.attachmentPreview(issueId, attachment.id), "Could not show the attachment").finally(() => setLoading(false));
  };
  const openInBrowser = () => {
    if (url) void attempt(api.openUrl(url), "Could not open the attachment");
  };
  const copyLink = () => {
    if (!url) return;
    void attempt(api.copyText(url), "Could not copy").then((copied) => {
      if (copied !== null) toast("success", `Copied the link to ${filename}`);
    });
  };
  const menu = (e: MouseEvent) =>
    openContextMenu(
      e,
      [
        { text: "Quick Look", run: preview },
        { text: "Open in Browser", enabled: url !== null, run: openInBrowser },
        { text: "Copy link", enabled: url !== null, run: copyLink },
      ],
      filename,
    );
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
