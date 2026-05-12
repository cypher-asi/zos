import { useState, type ReactNode } from "react";
import { Settings } from "lucide-react";
import { Text, Heading, Label, Toggle, Navigator, Sidebar } from "@cypher-asi/zui";
import type { ShellApp } from "../shell/types";

const SECTIONS = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "shortcuts", label: "Keyboard Shortcuts" },
  { id: "about", label: "About" },
];

function SettingsSidebar() {
  const [active, setActive] = useState("general");

  return (
    <div style={{ padding: "var(--space-3)", display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
      <Label size="xs" uppercase>Settings</Label>
      <Navigator items={SECTIONS} value={active} onChange={setActive} />
    </div>
  );
}

function SettingsMain({ children }: { children?: ReactNode }) {
  return (
    <Sidebar flex>
      <main style={{ padding: "var(--space-4)", overflow: "auto", flex: 1, maxWidth: 640 }}>
        {children ?? (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
            <Heading level={2}>General</Heading>
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <Text size="sm">Auto-save</Text>
                  <Text size="xs" variant="muted">Automatically save changes</Text>
                </div>
                <Toggle defaultChecked />
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <Text size="sm">Notifications</Text>
                  <Text size="xs" variant="muted">Show desktop notifications</Text>
                </div>
                <Toggle />
              </div>
            </div>
          </div>
        )}
      </main>
    </Sidebar>
  );
}

export const SettingsApp: ShellApp = {
  id: "settings",
  label: "Settings",
  icon: Settings,
  basePath: "/settings",
  LeftPanel: SettingsSidebar,
  MainPanel: SettingsMain,
};
