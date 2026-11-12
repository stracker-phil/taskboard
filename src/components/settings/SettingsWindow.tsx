import { useMemo, useState } from "react";
import { resolveSettingsPage, searchSettings, settingsGroups, targetOn, useSettingsPagesVersion, type SettingsMatch, type SettingsPage } from "../../registry/settingsPages";
import { ScrollTarget, type RevealTarget } from "./controls";

/** What Settings is asked to show: a page, row or Space id. A new object per request, so asking again reveals
 *  again. */
export interface SettingsRequest {
  id?: string;
}

const targetOf = (match: SettingsMatch) => match.row?.id ?? match.page.id;

function NavButton({ page, row, current, onOpen }: SettingsMatch & { current: string; onOpen: (id: string) => void }) {
  const { icon: Icon } = page;
  return (
    <button
      type="button"
      aria-label={row ? `${row.title}, ${page.label}` : undefined}
      aria-current={!row && current === page.id ? "page" : undefined}
      onClick={() => onOpen(targetOf({ page, row }))}
    >
      <Icon size={14} aria-hidden="true" />
      <span className="settings-nav-text">
        <span className="settings-nav-title">{page.label}</span>
        {row && <span className="settings-nav-setting">{row.title}</span>}
      </span>
    </button>
  );
}

function SearchResults({ results, current, onOpen }: { results: SettingsMatch[]; current: string; onOpen: (id: string) => void }) {
  if (results.length === 0) return <p className="settings-nav-empty">No Results</p>;
  return (
    <div className="settings-nav-group" role="group" aria-label="Results">
      {results.map((match) => (
        <NavButton key={targetOf(match)} {...match} current={current} onOpen={onOpen} />
      ))}
    </div>
  );
}

function PageGroups({ current, onOpen }: { current: string; onOpen: (id: string) => void }) {
  return settingsGroups().map(({ group, pages }) => (
    <div key={group} className="settings-nav-group" role="group" aria-label={group}>
      <div className="settings-nav-heading" aria-hidden="true">
        {group}
      </div>
      {pages.map((page) => (
        <NavButton key={page.id} page={page} current={current} onOpen={onOpen} />
      ))}
    </div>
  ));
}

/** The page list under its group headings; a search replaces it with every matching page and row, best first. */
function SettingsNav({ current, onOpen }: { current: string; onOpen: (id: string) => void }) {
  const [query, setQuery] = useState("");
  const searching = query.trim() !== "";
  const results = searching ? searchSettings(query) : [];
  return (
    <nav className="settings-nav" aria-label="Settings pages">
      <div className="settings-titlebar" data-tauri-drag-region />
      <input
        className="input settings-nav-search"
        type="search"
        aria-label="Search Settings"
        placeholder="Search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && results[0]) onOpen(targetOf(results[0]));
        }}
      />
      <div className="settings-nav-list scroller">
        {searching ? <SearchResults results={results} current={current} onOpen={onOpen} /> : <PageGroups current={current} onOpen={onOpen} />}
      </div>
    </nav>
  );
}

/** The row to reveal: kept while the request and the page stay the same, so a re-render doesn't reveal it again. */
function useRevealTarget(request: SettingsRequest, page: SettingsPage | undefined): RevealTarget | undefined {
  return useMemo(() => (page ? targetOn(page, request.id) : undefined), [request, page]);
}

/** `request.id` may name a row: its page opens scrolled to it, washed in the accent once. */
export function SettingsWindow({ request, onOpen }: { request: SettingsRequest; onOpen: (id: string) => void }) {
  // Re-renders when a module registers a page after the window opened.
  useSettingsPagesVersion();
  const page = resolveSettingsPage(request.id);
  const target = useRevealTarget(request, page);
  if (!page) return null;
  const { Body } = page;

  return (
    <div className="settings-window">
      <SettingsNav current={page.id} onOpen={onOpen} />
      <div className="settings-pane">
        <header data-tauri-drag-region>
          <h2 data-tauri-drag-region>{page.label}</h2>
        </header>
        <div className="settings-body scroller" key={page.id}>
          {page.intro && <p className="settings-intro">{page.intro}</p>}
          <ScrollTarget.Provider value={target}>
            <Body />
          </ScrollTarget.Provider>
        </div>
      </div>
    </div>
  );
}
