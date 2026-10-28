import { useQueryClient } from "@tanstack/react-query";
import { api, type ProjectRow } from "../../../api";
import { SETTINGS_KEYS, useProjects } from "../../../queries/settings";
import { fail } from "../../../state/store";
import { Group, Row, SwitchInput } from "../controls";
import { useBusyAction } from "../useBusyAction";

/** Connection → Projects: the projects Jira lists, each tracked or not; tracked ones make the default scope. */
export function ProjectsGroup({ connected }: { connected: boolean }) {
  const client = useQueryClient();
  const { data: projects = [] } = useProjects();
  const [busy, run] = useBusyAction();
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

  return (
    <Group title="Projects" anchor="projects">
      <Row label="Jira projects" description={connected ? "Tracked projects make the default scope." : "Test the connection first."}>
        <button type="button" className="btn" disabled={!connected || busy} onClick={refresh}>
          Refresh
        </button>
      </Row>
      {projects.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Refresh to load your projects.</span>
        </div>
      ) : (
        projects.map((project) => (
          <Row key={project.id} label={project.name} description={<span className="mono">{project.key}</span>}>
            <SwitchInput aria-label={`Track ${project.name}`} checked={project.tracked} onChange={(on) => track(project, on)} />
          </Row>
        ))
      )}
    </Group>
  );
}
