import { api, type Behaviour as BehaviourSettings, type IssueOpening, type NotifyAbout, type PinDuration } from "../../api";
import { useSettings } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { Group, Segmented } from "./controls";
import { NumberField } from "./fields";
import { MeModeGroup } from "./MeModeGroup";

const PIN_DURATIONS = ["today", "twoDays", "week", "untilUnpinned"] as const satisfies readonly PinDuration[];
const OPENINGS = ["preview", "full"] as const satisfies readonly IssueOpening[];
const OPENING_LABELS: Record<IssueOpening, string> = { preview: "Preview", full: "Full view" };
const PIN_LABELS: Record<PinDuration, string> = { today: "Today", twoDays: "2 days", week: "1 week", untilUnpinned: "Until removed" };
const NOTIFY_ABOUT = ["nothing", "mentionsAndAssignments", "myIssues"] as const satisfies readonly NotifyAbout[];
const NOTIFY_LABELS: Record<NotifyAbout, string> = { nothing: "Nothing", mentionsAndAssignments: "Mentions and assignments", myIssues: "My issues" };

/** When an open issue counts as seen, and what a new pin starts with. */
function Behaviour() {
  const { data: settings } = useSettings();
  if (!settings) return null;
  const save = (patch: Partial<BehaviourSettings>) =>
    void attempt(api.behaviourSet({ ...settings.behaviour, ...patch }), "Could not save the setting");
  return (
    <Group title="Behavior" anchor="behaviour">
      <NumberField
        label="Count an issue as read after"
        description="How long it stays open in the preview or the Issue view; 0 marks it at once."
        suffix="seconds"
        min={0}
        max={10}
        value={settings.behaviour.seenAfterSeconds}
        onCommit={(seenAfterSeconds) => save({ seenAfterSeconds })}
      />
      <Segmented
        label="Open issues in"
        description="What a click on an issue opens; each List or Table view can choose its own (Columns)."
        options={OPENINGS}
        labels={OPENING_LABELS}
        value={settings.behaviour.openIssuesIn}
        onChange={(openIssuesIn) => save({ openIssuesIn })}
      />
      <Segmented
        label="Bookmark for"
        description="What a new bookmark starts with; the bookmark popover can change it."
        options={PIN_DURATIONS}
        labels={PIN_LABELS}
        value={settings.behaviour.pinDefault}
        onChange={(pinDefault) => save({ pinDefault })}
      />
      <Segmented
        label="Notify me about"
        description="A macOS notification after a sync, while Taskboard isn't in front: mentions of you and issues assigned to you, or also any change of an issue assigned to or reported by you."
        options={NOTIFY_ABOUT}
        labels={NOTIFY_LABELS}
        value={settings.behaviour.notifyAbout}
        onChange={(notifyAbout) => save({ notifyAbout })}
      />
    </Group>
  );
}

function MeModes() {
  const { data: settings } = useSettings();
  return settings ? <MeModeGroup settings={settings} /> : null;
}

export function BehaviorPage() {
  return (
    <>
      <Behaviour />
      <MeModes />
    </>
  );
}
