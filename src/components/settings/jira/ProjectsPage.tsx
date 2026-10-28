import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, type ProjectRow } from "../../../api";
import { SETTINGS_KEYS, useConnection, useProjects } from "../../../queries/settings";
import { fail } from "../../../state/store";
import { Group, Row, SwitchInput } from "../controls";
import { useBusyAction } from "../useBusyAction";

/** The Jira projects, each with a Track switch. */
export function ProjectsPage() {
  const client = useQueryClient();
  const { data: view } = useConnection();
  const { data: projects = [] } = useProjects();
  const [query, setQuery] = useState("");
  const [busy, run] = useBusyAction();
  const connected = !!view?.connection.accountId;
  const refreshScope = () => void client.invalidateQueries({ queryKey: SETTINGS_KEYS.scope });

  const refresh = () =>
    void run(
      async () => {
        client.setQueryData(SETTINGS_KEYS.projects, await api.projectsRefresh());
        refreshScope();
      },
      (err) => fail(err, "Could not load the projects"),
    );

  const track = (project: ProjectRow, tracked: boolean) => {
    client.setQueryData<ProjectRow[]>(SETTINGS_KEYS.projects, (rows) => rows?.map((row) => (row.id === project.id ? { ...row, tracked } : row)));
    api.projectTrack(project.id, tracked).then(refreshScope, (e: unknown) => {
      fail(e, `Could not change ${project.name}`);
      void client.invalidateQueries({ queryKey: SETTINGS_KEYS.projects });
    });
  };

  const tracked = projects.filter((p) => p.tracked).length;
  const shown = matching(projects, query);

  return (
    <Group title="Projects" anchor="projects">
      <Row
        label="Jira projects"
        description={connected ? `${tracked} of ${projects.length} tracked. Tracked projects make the default scope.` : "Test the connection first."}
      >
        <button type="button" className="btn" disabled={!connected || busy} onClick={refresh}>
          Refresh
        </button>
      </Row>
      {projects.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Refresh to load your projects.</span>
        </div>
      ) : (
        <>
          <div className="settings-row">
            <input
              className="input settings-filter"
              type="search"
              aria-label="Filter projects"
              placeholder="Filter by name or key"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          {shown.length === 0 && (
            <div className="settings-row">
              <span className="settings-empty">No projects match “{query.trim()}”.</span>
            </div>
          )}
          {shown.map((project) => (
            <Row key={project.id} label={project.name} description={<span className="mono">{project.key}</span>}>
              <SwitchInput aria-label={`Track ${project.name}`} checked={project.tracked} onChange={(on) => track(project, on)} />
            </Row>
          ))}
        </>
      )}
    </Group>
  );
}

/** Every word of the query is in the project's name or key, ignoring case. */
function matching(projects: ProjectRow[], query: string): ProjectRow[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return projects.filter((p) => {
    const text = `${p.name} ${p.key}`.toLowerCase();
    return words.every((word) => text.includes(word));
  });
}
