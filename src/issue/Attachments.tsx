import type { MouseEvent } from "react";
import { api, type Attachment } from "../api";
import { openContextMenu } from "../components/menu";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

/** The issue's files as tiles; hidden when there are none. A tile opens the file on the Jira site in the default
 *  browser (Rust only hands out addresses on the connected site); the app never downloads or previews it. */
export function Attachments({ attachments }: { attachments: Attachment[] }) {
  if (attachments.length === 0) return null;
  return (
    <section className="attachments" aria-label="Attachments">
      <h2>Attachments</h2>
      <div className="attachments__grid">
        {attachments.map((attachment) => (
          <AttachmentTile key={attachment.id} attachment={attachment} />
        ))}
      </div>
    </section>
  );
}

function AttachmentTile({ attachment }: { attachment: Attachment }) {
  const toast = useStore((s) => s.toast);
  const { url, filename } = attachment;
  const meta = [attachment.size, attachment.authorName].filter(Boolean).join(" · ");
  const content = (
    <>
      <span className="attachment__badge" aria-hidden="true">
        {attachment.badge}
      </span>
      <span className="attachment__text">
        <span className="attachment__name">{filename}</span>
        {meta && <span className="attachment__meta">{meta}</span>}
      </span>
    </>
  );
  if (!url) {
    return (
      <div className="attachment attachment--inert" title={filename}>
        {content}
      </div>
    );
  }
  const open = () => void attempt(api.openUrl(url), "Could not open the attachment");
  const copy = () =>
    void attempt(api.copyText(url), "Could not copy").then((copied) => {
      if (copied !== null) toast("success", `Copied the link to ${filename}`);
    });
  const menu = (e: MouseEvent) =>
    openContextMenu(
      e,
      [
        { text: "Open", run: open },
        { text: "Copy link", run: copy },
      ],
      filename,
    );
  return (
    <button type="button" className="attachment" title={filename} onClick={open} onContextMenu={menu}>
      {content}
    </button>
  );
}
