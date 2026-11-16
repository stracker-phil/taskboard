import { useRef, useState, type MouseEvent } from "react";
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

/** One attachment's actions and context menu; `loading` covers Rust's download, and a request while one runs is ignored. */
export function useAttachmentActions(issueId: string, { id, filename, url }: AttachmentRef) {
  const toast = useStore((s) => s.toast);
  const [loading, setLoading] = useState(false);
  const busy = useRef(false);
  const whileLoading = <T>(request: () => Promise<T>, failure: string) => {
    if (busy.current) return Promise.resolve(null);
    busy.current = true;
    setLoading(true);
    return attempt(request(), failure).finally(() => {
      busy.current = false;
      setLoading(false);
    });
  };
  const preview = () => void whileLoading(() => api.attachmentPreview(issueId, id), "Could not show the attachment");
  const download = () =>
    void whileLoading(() => api.attachmentDownload(issueId, id), "Could not download the attachment");
  const saveAs = () =>
    void whileLoading(() => api.attachmentSaveAs(issueId, id), "Could not save the attachment").then((saved) => {
      if (saved) toast("success", `Saved ${filename}`);
    });
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
        { text: "Download", run: download },
        { text: "Save As…", run: saveAs },
        "separator",
        { text: "Open in Browser", enabled: url !== null, run: openInBrowser },
        { text: "Copy link", enabled: url !== null, run: copyLink },
      ],
      filename,
    );
  return { loading, preview, download, menu };
}
