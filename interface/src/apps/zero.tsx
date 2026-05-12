import type { ReactNode } from "react";
import { CircleDot } from "lucide-react";
import { Text, Heading, Label, Sidebar } from "@cypher-asi/zui";
import type { ShellApp } from "../shell/types";

function ZeroSidebar() {
  return (
    <div style={{ padding: "var(--space-3)", display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
      <Label size="xs" uppercase>ZERO</Label>
      <Text size="sm" variant="muted">
        Welcome to ZERO.
      </Text>
    </div>
  );
}

function ZeroMain({ children }: { children?: ReactNode }) {
  return (
    <Sidebar flex>
      <main style={{ padding: "var(--space-4)", overflow: "auto", flex: 1 }}>
        {children ?? (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
            <Heading level={2}>ZERO</Heading>
            <Text variant="secondary">
              Start from zero.
            </Text>
          </div>
        )}
      </main>
    </Sidebar>
  );
}

export const ZeroApp: ShellApp = {
  id: "zero",
  label: "ZERO",
  icon: CircleDot,
  basePath: "/zero",
  LeftPanel: ZeroSidebar,
  MainPanel: ZeroMain,
};
