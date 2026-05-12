import type { ReactNode } from "react";
import { FolderOpen } from "lucide-react";
import { Explorer, Text, Heading, Spinner, Sidebar, PageEmptyState } from "@cypher-asi/zui";
import { useProjects } from "../queries/projects";
import { useProjectsStore } from "../stores/projects-store";
import type { ShellApp } from "../shell/types";

function ProjectList() {
  const { data: projects = [], isLoading } = useProjects();
  const selectedId = useProjectsStore((s) => s.selectedId);
  const selectProject = useProjectsStore((s) => s.selectProject);

  if (isLoading && projects.length === 0) {
    return (
      <div style={{ display: "flex", justifyContent: "center", padding: "var(--space-4)" }}>
        <Spinner size="sm" />
      </div>
    );
  }

  const data = projects.map((p) => ({
    id: p.id,
    label: p.name,
  }));

  return (
    <Explorer
      data={data}
      onSelect={([id]) => selectProject(id ?? null)}
      defaultSelectedIds={selectedId ? [selectedId] : []}
    />
  );
}

function ProjectMain({ children }: { children?: ReactNode }) {
  const { data: projects = [] } = useProjects();
  const selectedId = useProjectsStore((s) => s.selectedId);
  const project = projects.find((p) => p.id === selectedId);

  return (
    <Sidebar flex>
      <main style={{ padding: "var(--space-4)", overflow: "auto", flex: 1 }}>
        {children ?? (
          project ? (
            <div>
              <Heading level={2}>{project.name}</Heading>
              <Text variant="secondary" style={{ marginTop: "var(--space-2)" }}>
                {project.description}
              </Text>
            </div>
          ) : (
            <PageEmptyState
              icon={<FolderOpen size={32} />}
              title="Select a project from the sidebar"
            />
          )
        )}
      </main>
    </Sidebar>
  );
}

function ProjectSidekick() {
  const { data: projects = [] } = useProjects();
  const selectedId = useProjectsStore((s) => s.selectedId);
  const project = projects.find((p) => p.id === selectedId);

  if (!project) {
    return <PageEmptyState title="No project selected" />;
  }

  return (
    <div style={{ padding: "var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
      <Heading level={4}>{project.name}</Heading>
      <Text variant="secondary" size="sm">{project.description}</Text>
      <div style={{ borderTop: "1px solid var(--color-border)", paddingTop: "var(--space-3)" }}>
        <Text size="xs" variant="muted">Last updated</Text>
        <Text size="sm">{new Date(project.updatedAt).toLocaleDateString()}</Text>
      </div>
    </div>
  );
}

export const ProjectsApp: ShellApp = {
  id: "projects",
  label: "Projects",
  icon: FolderOpen,
  basePath: "/projects",
  LeftPanel: ProjectList,
  MainPanel: ProjectMain,
  SidekickPanel: ProjectSidekick,
};
