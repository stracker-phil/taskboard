import { useConnection } from "../../../queries/settings";
import { ProjectsGroup } from "./ProjectsGroup";
import { SiteGroup } from "./SiteGroup";
import { TokenGroup } from "./TokenGroup";

/** Settings → Connection: the Jira site, the token's expiry and the tracked projects. */
export function ConnectionPage() {
  const { data: view } = useConnection();
  if (!view) return null;
  return (
    <>
      <SiteGroup view={view} />
      <TokenGroup view={view} />
      <ProjectsGroup connected={!!view.connection.accountId} />
    </>
  );
}
