import { useLayoutEffect, useState, type RefObject } from "react";
import { createPortal } from "react-dom";
import { PeekCard, place } from "../issue/hoverPeek/PeekCard";
import { usePreviewedAfter, useSeenAfter } from "../issue/useSeenAfter";

interface Props {
  id: string;
  unread: boolean;
  /** The inbox panel it sits beside. */
  beside: RefObject<HTMLElement | null>;
}

/** The selected issue's peek card left of the inbox; previewed for the read delay, it counts as read and as
 *  used, like the main preview. */
export function InboxPreview({ id, unread, beside }: Props) {
  const [rect, setRect] = useState<DOMRect | null>(null);
  useLayoutEffect(() => setRect(beside.current?.getBoundingClientRect() ?? null), [beside]);
  useSeenAfter(id, unread);
  usePreviewedAfter(id);
  if (!rect) return null;
  return createPortal(<PeekCard id={id} style={place(rect, "left")} interactive />, document.body);
}
