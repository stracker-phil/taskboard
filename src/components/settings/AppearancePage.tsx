import { api, type Appearance } from "../../api";
import { useSettings } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { Group, Segmented } from "./controls";

const OPTIONS = ["system", "light", "dark"] as const satisfies readonly Appearance[];

/** Light, dark or following macOS, for every Taskboard window. */
export function AppearancePage() {
  const { data: settings } = useSettings();
  return (
    <Group title="Appearance" anchor="appearance">
      <Segmented
        label="Appearance"
        description="Automatic follows macOS."
        options={OPTIONS}
        labels={{ system: "Automatic" }}
        value={settings?.appearance ?? null}
        onChange={(appearance) => void attempt(api.appearanceSet(appearance), "Could not change the appearance")}
      />
    </Group>
  );
}
