import { useState } from "react";
import { PanelSearch } from "../PanelSearch";
import { useActiveApp } from "../../hooks/use-active-app";
import { useAppUIStore } from "../../stores/app-ui-store";

/**
 * Header slot for the left-panel Lane. Each app has its own draft query that
 * persists into `useAppUIStore.sidebarQueries[appId]` so navigating away and
 * back restores the user's input.
 */
export function SidebarSearchInput() {
  const activeApp = useActiveApp();
  const initialQuery = useAppUIStore((s) => s.sidebarQueries[activeApp.id] ?? "");
  const setSidebarQuery = useAppUIStore((s) => s.setSidebarQuery);
  const [draft, setDraft] = useState(initialQuery);

  return (
    <PanelSearch
      placeholder={activeApp.searchPlaceholder ?? "Search"}
      value={draft}
      onChange={(value) => {
        setDraft(value);
        setSidebarQuery(activeApp.id, value);
      }}
    />
  );
}
