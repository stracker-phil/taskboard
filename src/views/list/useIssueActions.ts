import { useMemo } from "react";
import { api } from "../../api";
import { browseUrl } from "../../lib/jiraLinks";
import { useConnection, useSettings } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import type { MenuAt } from "../../components/menu";
import { runWrite } from "../../writes/runWrite";
import { openStatusMenu } from "../../writes/statusMenu";

/** Copies the text and confirms with a toast; subscribes to nothing, so every row can use it. */
export function copyWithToast(text: string, done: string) {
  void attempt(api.copyText(text), "Could not copy").then((copied) => {
    if (copied !== null) useStore.getState().toast("success", done);
  });
}

/** Per-issue actions; null when they can't run: the URL actions without a Jira site, the writes (`toggleMine`,
 *  `changeStatus`) while changes to Jira are off. */
export function useIssueActions() {
  const { data: connection } = useConnection();
  const { data: settings } = useSettings();
  const site = connection?.connection.siteUrl;
  const jiraChanges = settings?.behaviour.allowJiraChanges ?? false;
  // Stable, so memoized cards don't re-render with their parent.
  return useMemo(() => {
    const urlOf = (key: string) => browseUrl(site, key);
    const openInJira = site ? (key: string) => void attempt(api.openUrl(urlOf(key) ?? ""), "Could not open Jira") : null;
    return {
      copyKey: (key: string) => copyWithToast(key, `Copied ${key}`),
      copyUrl: (key: string) => {
        const url = urlOf(key);
        if (url) copyWithToast(url, `Copied the link to ${key}`);
      },
      openInJira,
      markSeen: (id: string) => void attempt(api.issuesMarkSeen([id]), "Could not mark it read"),
      markUnread: (id: string) => void attempt(api.issueMarkUnread(id), "Could not mark it unread"),
      togglePin: (id: string) => void attempt(api.pinToggleIssue(id), "Could not change the bookmark"),
      toggleMine: jiraChanges ? (id: string) => runWrite(id, api.issueToggleMine(id), "Could not change the assignee") : null,
      /** Opens the status menu under `at`. */
      changeStatus: jiraChanges ? (issue: { id: string; key: string }, at: MenuAt) => openStatusMenu(issue, at, openInJira) : null,
    };
  }, [site, jiraChanges]);
}
