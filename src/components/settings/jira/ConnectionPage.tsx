import { useConnection } from "../../../queries/settings";
import { ChangesGroup, ReadsGroup } from "./ChangesGroup";
import { SiteGroup } from "./SiteGroup";
import { TokenGroup } from "./TokenGroup";
import { TokenScopesGroup } from "./TokenScopesGroup";

/** Settings → Connection: the Jira site, the token, every scope it needs, what Taskboard reads and whether it may
 *  change Jira. */
export function ConnectionPage() {
  const { data: view } = useConnection();
  if (!view) return null;
  return (
    <>
      <SiteGroup view={view} />
      <TokenGroup view={view} />
      <TokenScopesGroup tokenKind={view.connection.tokenKind} />
      <ReadsGroup />
      <ChangesGroup />
    </>
  );
}
