import { api, type Behaviour as BehaviourSettings, type PinDuration } from "../../api";
import { useSettings } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { useAppInfo } from "../../queries/useAppInfo";
import { fail } from "../../state/store";
import { Group, Row, Segmented } from "./controls";
import { NumberField } from "./fields";

function About() {
  const { data } = useAppInfo();
  return (
    <Group title="About" anchor="about">
      <Row label={data?.name ?? "Taskboard"}>
        <span className="settings-value">{data ? `Version ${data.version}` : ""}</span>
      </Row>
      <Row label="Logs" description="What Taskboard did and what went wrong, for a bug report.">
        <button type="button" className="btn" onClick={() => void api.openLogFolder().catch((e) => fail(e, "Could not open the log folder"))}>
          Open Log Folder
        </button>
      </Row>
    </Group>
  );
}

const PIN_DURATIONS = ["today", "twoDays", "week", "untilUnpinned"] as const satisfies readonly PinDuration[];
const PIN_LABELS: Record<PinDuration, string> = { today: "Today", twoDays: "2 days", week: "1 week", untilUnpinned: "Until unpinned" };

/** When an open issue counts as seen, and what a new pin starts with. */
function Behaviour() {
  const { data: settings } = useSettings();
  if (!settings) return null;
  const save = (patch: Partial<BehaviourSettings>) =>
    void attempt(api.behaviourSet({ ...settings.behaviour, ...patch }), "Could not save the setting");
  return (
    <Group title="Behaviour" anchor="behaviour">
      <NumberField
        label="Count an issue as seen after"
        description="How long it stays open in the peek or the Issue view; 0 marks it at once."
        suffix="seconds"
        min={0}
        max={10}
        value={settings.behaviour.seenAfterSeconds}
        onCommit={(seenAfterSeconds) => save({ seenAfterSeconds })}
      />
      <Segmented
        label="Pin for"
        description="What a new pin starts with; the pin popover can change it."
        options={PIN_DURATIONS}
        labels={PIN_LABELS}
        value={settings.behaviour.pinDefault}
        onChange={(pinDefault) => save({ pinDefault })}
      />
    </Group>
  );
}

export function GeneralPage() {
  return (
    <>
      <Behaviour />
      <About />
    </>
  );
}
