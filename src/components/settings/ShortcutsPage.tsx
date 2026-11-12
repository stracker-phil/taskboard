import { RotateCcw, Search, X } from "lucide-react";
import { type ReactNode, useState } from "react";
import { api, type Keybindings } from "../../api";
import { allCommands, bindingFor, bindingFromEvent, bindingLabel, conflicts, holdersOf, withBinding, type Command } from "../../commands/registry";
import { scopeLabel } from "../../commands/scopes";
import { useKeybindings } from "../../queries/settings";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { Group, Row } from "./controls";
import { ShortcutRecorder } from "./ShortcutRecorder";

/** The app menu's keys: fixed, listed so the page names every key. */
export const MENU_KEYS: { title: string; binding: string }[] = [
  { title: "Settings", binding: "Mod+," },
  { title: "Toggle Sidebar", binding: "Mod+\\" },
  { title: "Refresh", binding: "Mod+R" },
  { title: "Full Sync", binding: "Mod+Shift+R" },
  { title: "Close Tab or Window", binding: "Mod+W" },
  { title: "New Tab", binding: "Mod+Shift+T" },
  { title: "Previous Tab", binding: "Mod+Shift+[" },
  { title: "Next Tab", binding: "Mod+Shift+]" },
  { title: "Reopen Closed Tab", binding: "Mod+Alt+T" },
  { title: "Quit Taskboard", binding: "Mod+Q" },
];

export const shortcutAnchor = (commandId: string) => `shortcut:${commandId}`;
export const menuKeyAnchor = (title: string) => `menu-key:${title}`;

const matches = (command: Command, needle: string) => !needle || command.title.toLowerCase().includes(needle) || command.id.toLowerCase().includes(needle);

/** A small button that keeps its place while hidden: invisible, out of the tab order and the accessibility tree. */
function SlotButton({ shown, label, title, onClick, children }: { shown: boolean; label: string; title: string; onClick: () => void; children: ReactNode }) {
  if (!shown) return <span className="icon-btn tiny shortcut-control__slot" aria-hidden="true" />;
  return (
    <button type="button" className="icon-btn tiny" aria-label={label} title={title} onClick={onClick}>
      {children}
    </button>
  );
}

type Change = (command: Command, binding: string | null) => void;

function ShortcutRow({ command, bindings, change }: { command: Command; bindings: Keybindings; change: Change }) {
  const binding = bindingFor(command.id, bindings);
  const fallback = bindingFor(command.id, {});
  const set = (next: string | null) => change(command, next);
  return (
    <Row label={command.title} anchor={shortcutAnchor(command.id)} description={scopeLabel(command.scope)}>
      {/* Both buttons keep their slot, shown or not, so a click never lands on the other one. */}
      <div className="shortcut-control">
        <SlotButton shown={binding !== fallback} label={`Reset ${command.title}`} title="Reset to default" onClick={() => set(fallback)}>
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

/** Takes a key another command holds only after the owner confirms Reassign, in one save (ADR 0072). */
async function reassign(bindings: Keybindings, command: Command, binding: string | null) {
  const { confirm, toast } = useStore.getState();
  const label = binding ? bindingLabel(binding) : null;
  const menuKey = label ? MENU_KEYS.find((key) => bindingLabel(key.binding) === label) : undefined;
  if (menuKey) {
    toast("error", `${label} belongs to ${menuKey.title} in the menu.`);
    return;
  }
  const holders = binding ? holdersOf(command, binding, bindings) : [];
  if (holders.length > 0) {
    const names = holders.map((holder) => `“${holder.title}”`).join(" and ");
    const confirmed = await confirm({
      title: `${label} is used by ${names}`,
      text: `Reassign it to “${command.title}”? ${names} will have no shortcut.`,
      action: "Reassign",
    });
    if (!confirmed) return;
  }
  await attempt(api.setKeybindings(withBinding(bindings, command, binding, holders)), "Could not save the shortcut");
}

/** Every key of the Taskboard window, rebindable (`keybindings.json`); the menu's keys are listed, fixed. */
export function ShortcutsPage() {
  const { data: bindings = {} } = useKeybindings();
  const [search, setSearch] = useState("");
  const change: Change = (command, binding) => void reassign(bindings, command, binding);
  const needle = search.trim().toLowerCase();
  const list = allCommands().filter((command) => matches(command, needle));
  const clashes = conflicts(bindings);
  return (
    <>
      <label className="settings-search">
        <Search size={14} aria-hidden="true" />
        <input aria-label="Search shortcuts" placeholder="Search commands" value={search} onChange={(e) => setSearch(e.target.value)} />
      </label>
      <Group title="In the Taskboard window" anchor="shortcuts-window">
        {list.length === 0 && <div className="settings-row muted">No matching commands.</div>}
        {list.map((command) => (
          <ShortcutRow key={command.id} command={command} bindings={bindings} change={change} />
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
      <Group title="In the menu" anchor="shortcuts-menu" description="These keys belong to the app menu and can't be changed.">
        {MENU_KEYS.map((key) => (
          <Row key={key.title} label={key.title} anchor={menuKeyAnchor(key.title)}>
            <span className="shortcut-fixed">{bindingLabel(key.binding)}</span>
          </Row>
        ))}
      </Group>
    </>
  );
}
