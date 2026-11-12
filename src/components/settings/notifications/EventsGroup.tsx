import type { Notifications, NotifyEvent } from "../../../api";
import { Group } from "../controls";
import { type MatrixRow, SettingsMatrix } from "../SettingsMatrix";
import { useSaveNotifications } from "./useSaveNotifications";

type Channel = "inApp" | "mac";

const COLUMNS: { key: Channel; header: string; title: string; cellLabel: string }[] = [
  { key: "inApp", header: "In app", title: "The issue turns unread and shows in the inbox", cellLabel: "in app" },
  { key: "mac", header: "macOS", title: "A notification after a sync, while Taskboard isn't in front", cellLabel: "on macOS" },
];

const SECTIONS: { title: string; events: { event: NotifyEvent; label: string }[] }[] = [
  {
    title: "About you",
    events: [
      { event: "mentionsMe", label: "Mentions me" },
      { event: "assignsMe", label: "Assigns me" },
      { event: "unassignsMe", label: "Unassigns me" },
      { event: "addsMeToPeopleField", label: "Adds me to a people field" },
    ],
  },
  {
    title: "On these issues",
    events: [
      { event: "newComment", label: "New comment" },
      { event: "movedToDone", label: "Moved to Done" },
      { event: "reopened", label: "Reopened (back to To Do)" },
      { event: "otherChange", label: "Other status change" },
      { event: "dueOrPriority", label: "Due date or priority changed" },
      { event: "created", label: "Issue created" },
    ],
  },
];

const ORDER = SECTIONS.flatMap((section) => section.events.map(({ event }) => event));

/** Rows never move, so a click only flips its box. */
export function EventsGroup({ notifications }: { notifications: Notifications }) {
  const save = useSaveNotifications();
  const toggle = (event: string, channel: string, on: boolean) => {
    const key = channel as Channel;
    const events = notifications[key];
    save({ [key]: on ? ORDER.filter((e) => e === event || events.includes(e)) : events.filter((e) => e !== event) });
  };
  const rows: MatrixRow[] = SECTIONS.flatMap(({ title, events }) => [
    { key: title, header: <span className="settings-matrix__section">{title}</span>, cells: [] },
    ...events.map(({ event, label }) => ({
      key: event,
      header: label,
      cells: COLUMNS.map(({ key, cellLabel }) => ({ checked: notifications[key].includes(event), label: `${label} ${cellLabel}` })),
    })),
  ]);

  return (
    <Group
      title="Notify me about"
      anchor="notify-about"
      description="In app turns the issue unread; macOS notifies after a sync. Changes about you always pass the filters above."
    >
      <SettingsMatrix corner="Change" columns={COLUMNS} rows={rows} onToggle={toggle} />
    </Group>
  );
}
