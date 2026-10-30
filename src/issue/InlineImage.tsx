import { type CSSProperties, useRef, useState } from "react";
import { api } from "../api";
import { useIssue } from "../queries/issues";
import { useAttachmentActions } from "./useAttachmentActions";
import { useVisible } from "./useVisible";

/** The loading box's size when the document names none. */
const DEFAULT_SIZE = { width: 320, height: 180 };

export interface InlineImageProps {
  issueId: string;
  attachmentId: string;
  name: string;
  width: number | null;
  height: number | null;
}

/**
 * An image attached to the issue, inside rich text (ADR 0039, ADR 0042). The webview loads it from Rust's
 * `inline-image:` scheme only once it is on screen; until it has loaded a neutral box of its size holds its
 * place, and when Rust has none it stays the "Attachment · name" chip. A click shows it in Quick Look; its
 * context menu is the attachment tile's.
 */
export function InlineImage({ issueId, attachmentId, name, width, height }: InlineImageProps) {
  const box = useRef<HTMLSpanElement>(null);
  const visible = useVisible(box);
  const [load, setLoad] = useState<"loading" | "loaded" | "failed">("loading");
  const { data: issue } = useIssue(issueId);
  const url = issue?.attachments.find((attachment) => attachment.id === attachmentId)?.url ?? null;
  const { preview, menu } = useAttachmentActions(issueId, { id: attachmentId, filename: name, url });

  if (load === "failed") return <span className="jira-chip">Attachment · {name}</span>;
  if (!visible) return <span ref={box} className="jira-image jira-image--loading" role="img" aria-label={name} aria-busy="true" style={boxStyle(width, height)} />;
  const loaded = load === "loaded";
  return (
    <button
      type="button"
      className="jira-image-button"
      title={`Quick Look ${name}`}
      disabled={!loaded}
      onClick={(e) => {
        e.stopPropagation();
        preview();
      }}
      onContextMenu={menu}
    >
      <img
        className={loaded ? "jira-image" : "jira-image jira-image--loading"}
        style={loaded ? undefined : boxStyle(width, height)}
        aria-busy={loaded ? undefined : true}
        src={api.inlineImageUrl(issueId, attachmentId)}
        alt={name}
        width={width ?? undefined}
        height={height ?? undefined}
        onLoad={() => setLoad("loaded")}
        onError={() => setLoad("failed")}
      />
    </button>
  );
}

function boxStyle(width: number | null, height: number | null): CSSProperties {
  const size = width && height ? { width, height } : DEFAULT_SIZE;
  return { width: size.width, aspectRatio: `${size.width} / ${size.height}` };
}
