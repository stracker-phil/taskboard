import { type MouseEvent, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api } from "../api";
import { sanitizeJiraHtml } from "../lib/jiraHtml";
import { useConnection } from "../queries/settings";
import { attempt } from "../state/requests";
import { InlineImage, type InlineImageProps } from "./InlineImage";

const SLOT = "jira-image-slot";

type Slot = Omit<InlineImageProps, "issueId"> & { host: HTMLElement };

/** Each inline image placeholder becomes an empty slot an `InlineImage` renders into; the slot keeps the
 *  placeholder's id, name and size, so finding the slots again after a re-run finds the same ones. */
function withSlots(clean: string): string {
  if (!clean.includes("data-attachment-id")) return clean;
  const template = document.createElement("template");
  template.innerHTML = clean;
  for (const image of Array.from(template.content.querySelectorAll("img[data-attachment-id]"))) {
    const slot = document.createElement("span");
    slot.className = SLOT;
    for (const [from, to] of [["data-attachment-id", "data-attachment-id"], ["alt", "data-name"], ["width", "data-width"], ["height", "data-height"]]) {
      const value = image.getAttribute(from);
      if (value !== null) slot.setAttribute(to, value);
    }
    image.replaceWith(slot);
  }
  return template.innerHTML;
}

function slotsIn(root: HTMLElement): Slot[] {
  const size = (value: string | undefined) => (value ? Number(value) : null);
  return Array.from(root.querySelectorAll<HTMLElement>(`.${SLOT}`)).map((host) => ({
    host,
    attachmentId: host.dataset.attachmentId ?? "",
    name: host.dataset.name || "image",
    width: size(host.dataset.width),
    height: size(host.dataset.height),
  }));
}

/** Jira's HTML (a description, a comment) of one issue, sanitized; its links open in the default browser and its
 *  attached images show inline (ADR 0039). */
export function JiraHtml({ html, issueId, className }: { html: string; issueId: string; className?: string }) {
  const { data: connection } = useConnection();
  const site = connection?.connection.siteUrl;
  const clean = useMemo(() => withSlots(sanitizeJiraHtml(html, site)), [html, site]);
  const root = useRef<HTMLDivElement>(null);
  const [slots, setSlots] = useState<Slot[]>([]);
  useLayoutEffect(() => {
    setSlots(root.current ? slotsIn(root.current) : []);
  }, [clean]);
  const openLink = (e: MouseEvent) => {
    const link = e.target instanceof Element ? e.target.closest("a") : null;
    if (!link) return;
    e.preventDefault();
    const href = link.getAttribute("href");
    if (href) void attempt(api.openUrl(href), "Could not open the link");
  };
  return (
    <>
      <div ref={root} className={["jira-html", className].filter(Boolean).join(" ")} onClick={openLink} dangerouslySetInnerHTML={{ __html: clean }} />
      {slots.map(({ host, ...image }, i) => createPortal(<InlineImage issueId={issueId} {...image} />, host, `${i}-${image.attachmentId}`))}
    </>
  );
}
