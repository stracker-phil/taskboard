import { useConnection } from "../../../queries/settings";
import { SiteGroup } from "./SiteGroup";
import { TokenGroup } from "./TokenGroup";

/** Settings → Connection: the Jira site, and the token's expiry. */
export function ConnectionPage() {
  const { data: view } = useConnection();
  if (!view) return null;
  return (
    <>
      <SiteGroup view={view} />
      <TokenGroup view={view} />
    </>
  );
}
