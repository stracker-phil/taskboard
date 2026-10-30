import { useConnection } from "../../../queries/settings";
import { ChangesGroup } from "./ChangesGroup";
import { SiteGroup } from "./SiteGroup";
import { TokenGroup } from "./TokenGroup";

/** Settings → Connection: the Jira site, the token's expiry, and whether Taskboard may change Jira. */
export function ConnectionPage() {
  const { data: view } = useConnection();
  if (!view) return null;
  return (
    <>
      <SiteGroup view={view} />
      <TokenGroup view={view} />
      <ChangesGroup />
    </>
  );
}
