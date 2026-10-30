import { RotateCcw, Search, X } from "lucide-react";
import { type ReactNode, useState } from "react";
import { api, type Keybindings } from "../../api";
import { allCommands, bindingFor, bindingFromEvent, bindingLabel, conflicts, type Command } from "../../commands/registry";
import { scopeLabel } from "../../commands/scopes";
import { useKeybindings } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { Group, Row } from "./controls";
import { ShortcutRecorder } from "./ShortcutRecorder";

/** The app menu's keys: fixed, listed so the page names every key. */
const MENU_KEYS: { title: string; binding: string }[] = [
  { title: "Settings", binding: "Mod+," },
  { title: "Toggle Sidebar", binding: "Mod+\\" },
  { title: "Refresh", binding: "Mod+R" },
  { title: "Full Sync", binding: "Mod+Shift+R" },
  { title: "Close Window", binding: "Mod+W" },
  { title: "Quit Taskboard", binding: "Mod+Q" },
];

const matches = (command: Command, needle: string) => !needle || command.title.toLowerCase().includes(needle) || command.id.toLowerCase().includes(needle);

function withoutBinding(bindings: Keybindings, id: string): Keybindings {
  const rest = { ...bindings };
  delete rest[id];
  return rest;
}

/** A small button that keeps its place while hidden: invisible, out of the tab order and the accessibility tree. */
function SlotButton({ shown, label, title, onClick, children }: { shown: boolean; label: string; title: string; onClick: () => void; children: ReactNode }) {
  if (!shown) return <span className="icon-btn tiny shortcut-control__slot" aria-hidden="true" />;
  return (
    <button type="button" className="icon-btn tiny" aria-label={label} title={title} onClick={onClick}>
      {children}
    </button>
  );
}

function ShortcutRow({ command, bindings, save }: { command: Command; bindings: Keybindings; save: (next: Keybindings) => void }) {
  const overridden = Object.hasOwn(bindings, command.id);
  const binding = bindingFor(command.id, bindings);
  const set = (next: string | null) => save({ ...bindings, [command.id]: next });
  return (
    <Row label={command.title} description={scopeLabel(command.scope)}>
      {/* Both buttons keep their slot, shown or not, so a click never lands on the other one. */}
      <div className="shortcut-control">
        <SlotButton shown={overridden} label={`Reset ${command.title}`} title="Reset to default" onClick={() => save(withoutBinding(bindings, command.id))}>
          <RotateCcw size={11} />
        </SlotButton>
        <SlotButton shown={binding !== null} label={`Remove shortcut for ${command.title}`} title="Remove shortcut" onClick={() => set(null)}>
          <X size={11} />
        </SlotButton>
        <ShortcutRecorder value={binding} parse={bindingFromEvent} label={bindingLabel} onChange={set} name={command.title} />
      </div>
    </Row>
  );
}

/** Every key of the Taskboard window, rebindable (`keybindings.json`); the menu's keys are listed, fixed. */
export function ShortcutsPage() {
  const { data: bindings = {} } = useKeybindings();
  const [search, setSearch] = useState("");
  const save = (next: Keybindings) => void attempt(api.setKeybindings(next), "Could not save the shortcut");
  const needle = search.trim().toLowerCase();
  const list = allCommands().filter((command) => matches(command, needle));
  const clashes = conflicts(bindings);
  return (
    <>
      <label className="settings-search">
        <Search size={14} aria-hidden="true" />
        <input aria-label="Search shortcuts" placeholder="Search commands" value={search} onChange={(e) => setSearch(e.target.value)} />
      </label>
      <Group title="In the Taskboard window">
        {list.length === 0 && <div className="settings-row muted">No matching commands.</div>}
        {list.map((command) => (
          <ShortcutRow key={command.id} command={command} bindings={bindings} save={save} />
        ))}
      </Group>
      {clashes.length > 0 && (
        <div className="shortcut-conflicts" role="alert">
          {clashes.map((clash) => (
            <div key={`${clash.a.id}:${clash.b.id}`}>
              {bindingLabel(clash.binding)} is used by both “{clash.a.title}” and “{clash.b.title}”.
            </div>
          ))}
        </div>
      )}
      <Group title="In the menu" description="These keys belong to the app menu and can't be changed.">
        {MENU_KEYS.map((key) => (
          <Row key={key.title} label={key.title}>
            <span className="shortcut-fixed">{bindingLabel(key.binding)}</span>
          </Row>
        ))}
      </Group>
    </>
  );
}
