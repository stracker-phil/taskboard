import { api } from "../../api";
import { useAppInfo } from "../../queries/useAppInfo";
import { fail } from "../../state/store";
import { Group, Row } from "./controls";
import { ShowDataFolder } from "./privacy/ShowDataFolder";

/** Settings → About: the version, and the folders a bug report or a privacy check looks at. */
export function AboutPage() {
  const { data } = useAppInfo();
  return (
    <Group title="About" anchor="about">
      <Row label={data?.name ?? "Taskboard"}>
        <span className="settings-value">{data ? `Version ${data.version}` : ""}</span>
      </Row>
      <Row label="Data" description="Your setup and, unless the Jira data stays in memory, the cache.">
        <ShowDataFolder />
      </Row>
      <Row label="Logs" description="What Taskboard did and what went wrong, for a bug report.">
        <button type="button" className="btn" onClick={() => void api.openLogFolder().catch((e) => fail(e, "Could not open the log folder"))}>
          Open Log Folder
        </button>
      </Row>
    </Group>
  );
}
