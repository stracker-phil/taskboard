import { useMemo } from "react";
import { api } from "../../api";
import { browseUrl } from "../../lib/jiraLinks";
import { useConnection } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";

/** Copy key, copy URL, Open in Jira, the read toggle and the pin toggle; the URL ones are null without a Jira site. */
export function useIssueActions() {
  const { data: connection } = useConnection();
  const toast = useStore((s) => s.toast);
  const site = connection?.connection.siteUrl;
  // Stable, so memoised cards don't re-render with their parent.
  return useMemo(() => {
    const copy = (text: string, done: string) =>
      void attempt(api.copyText(text), "Could not copy").then((copied) => {
        if (copied !== null) toast("success", done);
      });
    const urlOf = (key: string) => browseUrl(site, key);
    return {
      copyKey: (key: string) => copy(key, `Copied ${key}`),
      copyUrl: (key: string) => {
        const url = urlOf(key);
        if (url) copy(url, `Copied the link to ${key}`);
      },
      openInJira: site ? (key: string) => void attempt(api.openUrl(urlOf(key) ?? ""), "Could not open Jira") : null,
      markSeen: (id: string) => void attempt(api.issuesMarkSeen([id]), "Could not mark it read"),
      markUnread: (id: string) => void attempt(api.issueMarkUnread(id), "Could not mark it unread"),
      togglePin: (id: string) => void attempt(api.pinToggleIssue(id), "Could not change the bookmark"),
    };
  }, [site, toast]);
}
