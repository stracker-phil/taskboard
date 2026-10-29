import { useState, type MouseEvent } from "react";
import { api } from "../api";
import { openContextMenu } from "../components/menu";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

/** What an attachment's tile and an inline image need: its id, name and the address on the Jira site, if any. */
export interface AttachmentRef {
  id: string;
  filename: string;
  url: string | null;
}

/** Quick Look (which Rust downloads for first) and the context menu: Quick Look, Open in Browser, Copy link. */
export function useAttachmentActions(issueId: string, { id, filename, url }: AttachmentRef) {
  const toast = useStore((s) => s.toast);
  const [loading, setLoading] = useState(false);
  const preview = () => {
    if (loading) return;
    setLoading(true);
    void attempt(api.attachmentPreview(issueId, id), "Could not show the attachment").finally(() => setLoading(false));
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
  return { loading, preview, menu };
}
