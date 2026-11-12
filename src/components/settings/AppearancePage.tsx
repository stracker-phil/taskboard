import { api, type Appearance } from "../../api";
import { useSettings } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { AccentPicker } from "./AccentPicker";
import { Group, Row, Segmented } from "./controls";

const OPTIONS = ["system", "light", "dark"] as const satisfies readonly Appearance[];

/** Light, dark or following macOS, and the accent color, for every Taskboard window. */
export function AppearancePage() {
  const { data: settings } = useSettings();
  return (
    <Group>
      <Segmented
        label="Mode"
        anchor="mode"
        description="Automatic follows macOS."
        options={OPTIONS}
        labels={{ system: "Automatic" }}
        value={settings?.appearance ?? null}
        onChange={(appearance) => void attempt(api.appearanceSet(appearance), "Could not change the appearance")}
      />
      <Row label="Accent color" anchor="accent" description="Selection, links and focus. A Space can choose its own.">
        <AccentPicker label="Accent color" value={settings?.accent ?? null} onChange={(accent) => accent && void attempt(api.accentSet(accent), "Could not change the accent color")} />
      </Row>
    </Group>
  );
}
