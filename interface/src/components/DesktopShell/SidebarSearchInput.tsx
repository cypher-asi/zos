import { useState } from "react";
import { Search } from "@cypher-asi/zui";
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
    <Search
      size="sm"
      placeholder={activeApp.searchPlaceholder ?? "Search"}
      value={draft}
      onChange={(event) => {
        const value = event.target.value;
        setDraft(value);
        setSidebarQuery(activeApp.id, value);
      }}
      showClear
      onClear={() => {
        setDraft("");
        setSidebarQuery(activeApp.id, "");
      }}
    />
  );
}
