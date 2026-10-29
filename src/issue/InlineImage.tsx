import { type CSSProperties, useRef } from "react";
import { useAttachmentImage } from "../queries/attachmentImages";
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
 * An image attached to the issue, inside rich text (ADR 0039). It is asked from Rust as a `data:` URL only once
 * it is on screen; until then a neutral box of its size holds its place, and when Rust has none it stays the
 * "Attachment · name" chip. A click shows it in Quick Look; its context menu is the attachment tile's.
 */
export function InlineImage({ issueId, attachmentId, name, width, height }: InlineImageProps) {
  const box = useRef<HTMLSpanElement>(null);
  const visible = useVisible(box);
  const { data: src, isError, isSuccess } = useAttachmentImage(issueId, attachmentId, visible);
  const { data: issue } = useIssue(issueId);
  const url = issue?.attachments.find((attachment) => attachment.id === attachmentId)?.url ?? null;
  const { preview, menu } = useAttachmentActions(issueId, { id: attachmentId, filename: name, url });

  if (isError || (isSuccess && !src)) return <span className="jira-chip">Attachment · {name}</span>;
  if (!src) return <span ref={box} className="jira-image jira-image--loading" role="img" aria-label={name} aria-busy="true" style={boxStyle(width, height)} />;
  return (
    <button
      type="button"
      className="jira-image-button"
      title={`Quick Look ${name}`}
      onClick={(e) => {
        e.stopPropagation();
        preview();
      }}
      onContextMenu={menu}
    >
      <img className="jira-image" src={src} alt={name} width={width ?? undefined} height={height ?? undefined} />
    </button>
  );
}

function boxStyle(width: number | null, height: number | null): CSSProperties {
  const size = width && height ? { width, height } : DEFAULT_SIZE;
  return { width: size.width, aspectRatio: `${size.width} / ${size.height}` };
}
