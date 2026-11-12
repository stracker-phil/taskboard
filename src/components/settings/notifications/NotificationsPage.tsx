import { useSettings } from "../../../queries/settings";
import { EventsGroup } from "./EventsGroup";
import { WhichIssuesGroup } from "./WhichIssuesGroup";

/** Settings → Notifications: which changes make an issue unread and which a macOS notification announces. */
export function NotificationsPage() {
  const { data: settings } = useSettings();
  if (!settings) return null;
  return (
    <>
      <WhichIssuesGroup notifications={settings.notifications} />
      <EventsGroup notifications={settings.notifications} />
    </>
  );
}
