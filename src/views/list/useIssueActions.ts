import { useMemo } from "react";
import { api, type LinkTarget } from "../../api";
import { browseUrl } from "../../lib/jiraLinks";
import { useConnection } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import type { MenuAt } from "../../components/menu";
import { runWrite } from "../../writes/runWrite";
import { useJiraChange } from "../../writes/useJiraChanges";
import { openStatusMenu } from "../../writes/statusMenu";
import { openAssignMenu } from "../../writes/assignMenu";
import { openPriorityMenu } from "../../writes/priorityMenu";

/** Copies the text and confirms with a toast; subscribes to nothing, so every row can use it. */
export function copyWithToast(text: string, done: string) {
  void attempt(api.copyText(text), "Could not copy").then((copied) => {
    if (copied !== null) useStore.getState().toast("success", done);
  });
}

/** Copies the `taskboard://` link to an issue or a sidebar level. */
export function copyAppLink(target: LinkTarget, done: string) {
  void attempt(api.appLink(target), "Could not make the link").then((link) => {
    if (link !== null) copyWithToast(link, done);
  });
}

/** Per-issue actions; null when they can't run: the URL actions without a Jira site, the writes (`toggleMine`,
 *  `changeStatus`, `assignTo`, `changePriority`) while changes to Jira, or that kind of change, are off. */
export function useIssueActions() {
  const { data: connection } = useConnection();
  const site = connection?.connection.siteUrl;
  const canAssign = useJiraChange("assign");
  const canChangeStatus = useJiraChange("changeStatus");
  const canChangePriority = useJiraChange("changePriority");
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
      copyAppLink: (key: string) => copyAppLink({ kind: "issue", key }, `Copied the Taskboard link to ${key}`),
      openInJira,
      markSeen: (id: string) => void attempt(api.issuesMarkSeen([id]), "Could not mark it read"),
      markUnread: (id: string) => void attempt(api.issueMarkUnread(id), "Could not mark it unread"),
      togglePin: (id: string) => void attempt(api.pinToggleIssue(id), "Could not change the bookmark"),
      toggleMine: canAssign ? (id: string) => runWrite(id, api.issueToggleMine(id), "Could not change the assignee") : null,
      /** Opens the status menu under `at`. */
      changeStatus: canChangeStatus ? (issue: { id: string; key: string }, at: MenuAt) => openStatusMenu(issue, at, openInJira) : null,
      /** Opens the assignee menu under `at`. */
      assignTo: canAssign ? (issue: { id: string; key: string }, at: MenuAt) => openAssignMenu(issue, at) : null,
      /** Opens the priority menu under `at`. */
      changePriority: canChangePriority ? (issue: { id: string; key: string }, at: MenuAt) => openPriorityMenu(issue, at) : null,
    };
  }, [site, canAssign, canChangeStatus, canChangePriority]);
}
