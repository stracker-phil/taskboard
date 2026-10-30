import { useMemo } from "react";
import { api } from "../../api";
import { browseUrl } from "../../lib/jiraLinks";
import { useConnection } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";

/** Copies the text and confirms with a toast; subscribes to nothing, so every row can use it. */
export function copyWithToast(text: string, done: string) {
  void attempt(api.copyText(text), "Could not copy").then((copied) => {
    if (copied !== null) useStore.getState().toast("success", done);
  });
}

/** Copy key, copy URL, Open in Jira, the read toggle and the pin toggle; the URL ones are null without a Jira site. */
export function useIssueActions() {
  const { data: connection } = useConnection();
  const site = connection?.connection.siteUrl;
  // Stable, so memoized cards don't re-render with their parent.
  return useMemo(() => {
    const urlOf = (key: string) => browseUrl(site, key);
    return {
      copyKey: (key: string) => copyWithToast(key, `Copied ${key}`),
      copyUrl: (key: string) => {
        const url = urlOf(key);
        if (url) copyWithToast(url, `Copied the link to ${key}`);
      },
      openInJira: site ? (key: string) => void attempt(api.openUrl(urlOf(key) ?? ""), "Could not open Jira") : null,
      markSeen: (id: string) => void attempt(api.issuesMarkSeen([id]), "Could not mark it read"),
      markUnread: (id: string) => void attempt(api.issueMarkUnread(id), "Could not mark it unread"),
      togglePin: (id: string) => void attempt(api.pinToggleIssue(id), "Could not change the bookmark"),
    };
  }, [site]);
}
