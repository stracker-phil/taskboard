import { api } from "../../api";
import { useAppInfo } from "../../queries/useAppInfo";
import { fail } from "../../state/store";
import { Group, Row } from "./controls";
import { ShowDataFolder } from "./privacy/ShowDataFolder";

/** Settings → About: the version, and the folders a bug report or a privacy check looks at. */
export function AboutPage() {
  const { data } = useAppInfo();
  return (
    <Group>
      <Row label="Version" anchor="about-version">
        <span className="settings-value">{data ? `${data.name} ${data.version}` : ""}</span>
      </Row>
      <Row label="Data" anchor="about-data" description="Your setup and, unless the Jira data stays in memory, the cache.">
        <ShowDataFolder />
      </Row>
      <Row label="Logs" anchor="about-logs" description="What Taskboard did and what went wrong, for a bug report.">
        <button type="button" className="btn" onClick={() => void api.openLogFolder().catch((e) => fail(e, "Could not open the log folder"))}>
          Open Log Folder
        </button>
      </Row>
    </Group>
  );
}
