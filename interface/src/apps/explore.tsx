import { useState, type ReactNode } from "react";
import { Compass } from "lucide-react";
import { Text, Heading, Label, Navigator, Sidebar, PageEmptyState } from "@cypher-asi/zui";
import type { ShellApp } from "../shell/types";

const CATEGORIES = [
  { id: "templates", label: "Templates" },
  { id: "plugins", label: "Plugins" },
  { id: "themes", label: "Themes" },
  { id: "community", label: "Community" },
];

function ExploreSidebar() {
  const [active, setActive] = useState("templates");

  return (
    <div style={{ padding: "var(--space-3)", display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
      <Label size="xs" uppercase>Categories</Label>
      <Navigator items={CATEGORIES} value={active} onChange={setActive} />
    </div>
  );
}

function ExploreMain({ children }: { children?: ReactNode }) {
  return (
    <Sidebar flex>
      <main style={{ padding: "var(--space-4)", overflow: "auto", flex: 1 }}>
        {children ?? (
          <div>
            <Heading level={2}>Explore</Heading>
            <Text variant="secondary" style={{ marginTop: "var(--space-2)" }}>
              Discover templates, plugins, and community resources.
            </Text>
          </div>
        )}
      </main>
    </Sidebar>
  );
}

function ExploreSidekick() {
  return <PageEmptyState title="Select an item to see details" />;
}

export const ExploreApp: ShellApp = {
  id: "explore",
  label: "Explore",
  icon: Compass,
  basePath: "/explore",
  LeftPanel: ExploreSidebar,
  MainPanel: ExploreMain,
  SidekickPanel: ExploreSidekick,
};
