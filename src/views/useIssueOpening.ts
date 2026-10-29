import type { IssueOpening } from "../api";
import { useSettings } from "../queries/settings";

/** What a click on an issue opens: the view's choice, else Settings → General (the preview until it loads). */
export function useIssueOpening(openWith: IssueOpening | null): IssueOpening {
  const { data: settings } = useSettings();
  return openWith ?? settings?.behaviour.openIssuesIn ?? "preview";
}
