import type { SyntheticEvent } from "react";
import { copyWithToast } from "../../views/list/useIssueActions";

const stop = (e: SyntheticEvent) => e.stopPropagation();

/** An issue key that copies itself when clicked ("Copied NW-1042"). The click and double-click stop here, so
 *  the row, card or button around it neither selects nor opens the issue. */
export function CopyKey({ issueKey, className }: { issueKey: string; className?: string }) {
  return (
    <span
      className={className ? `${className} copy-key` : "copy-key"}
      title="Click to copy"
      onClick={(e) => {
        stop(e);
        copyWithToast(issueKey, `Copied ${issueKey}`);
      }}
      onDoubleClick={stop}
    >
      {issueKey}
    </span>
  );
}
