import { Outlet } from "@tanstack/react-router";
import { DesktopShell } from "../components/DesktopShell";

export function Shell() {
  return (
    <DesktopShell>
      <Outlet />
    </DesktopShell>
  );
}
