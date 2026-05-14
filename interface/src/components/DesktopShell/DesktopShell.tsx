import {
  Suspense,
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { Lane, type LaneResizeControls } from "../Lane";
import { ResponsiveMainLane } from "../ResponsiveMainLane";
import { BottomTaskbar } from "../BottomTaskbar";
import { useActiveApp } from "../../hooks/use-active-app";
import { useAppUIStore } from "../../stores/app-ui-store";
import { useDesktopBackgroundStore } from "../../stores/desktop-background-store";
import { AppsModal } from "../AppsModal";
import { BackgroundModal } from "../../apps/desktop/BackgroundModal";
import { SettingsModal } from "../SettingsModal";
import {
  persistSidekickWidth,
  readStoredSidekickWidth,
} from "./desktop-shell-sidekick";
import { BackgroundLayer } from "./BackgroundLayer";
import { DesktopTitlebar } from "./DesktopTitlebar";
import { PersistentSidekickLane } from "./PersistentSidekickLane";
import { SidebarSearchInput } from "./SidebarSearchInput";
import { SidekickPortalBridge } from "./SidekickPortalBridge";
import { useLeftPanelWidthCssVar } from "./desktop-shell-effects";
import styles from "./DesktopShell.module.css";

interface DesktopShellProps {
  /**
   * Routed content for the active app, e.g. the TanStack Router `<Outlet />`.
   * The shell wraps it in the active app's `MainPanel` so each app keeps
   * control of its own main-panel chrome.
   */
  children?: ReactNode;
}

export function DesktopShell({ children }: DesktopShellProps) {
  const activeApp = useActiveApp();
  const sidekickCollapsed = useAppUIStore((s) => s.sidekickCollapsed);
  const toggleSidekick = useAppUIStore((s) => s.toggleSidekick);
  const appsModalOpen = useAppUIStore((s) => s.appsModalOpen);
  const closeAppsModal = useAppUIStore((s) => s.closeAppsModal);
  const backgroundModalOpen = useAppUIStore((s) => s.backgroundModalOpen);
  const closeBackgroundModal = useAppUIStore((s) => s.closeBackgroundModal);
  const settingsModalOpen = useAppUIStore((s) => s.settingsModalOpen);
  const closeSettingsModal = useAppUIStore((s) => s.closeSettingsModal);

  const leftPanelRef = useRef<HTMLDivElement>(null);
  const sidekickResizeControlsRef = useRef<LaneResizeControls | null>(null);
  const [sidekickInitialWidth] = useState(() => readStoredSidekickWidth());
  const [sidekickHeaderTarget, setSidekickHeaderTarget] =
    useState<HTMLDivElement | null>(null);
  const [sidekickPanelTarget, setSidekickPanelTarget] =
    useState<HTMLDivElement | null>(null);
  const backgroundHydrated = useDesktopBackgroundStore((s) => s.hydrated);
  const { MainPanel } = activeApp;
  const isDesktop = activeApp.id === "desktop";
  const desktopModeActive = isDesktop && backgroundHydrated;
  const hasActiveSidekick = Boolean(activeApp.SidekickPanel) && !isDesktop;
  const sidekickHostCollapsed = sidekickCollapsed || !hasActiveSidekick;
  const showSidekickHeader = hasActiveSidekick && Boolean(activeApp.SidekickTaskbar);

  const markAppVisited = useAppUIStore((s) => s.markAppVisited);
  const setPreviousPath = useAppUIStore((s) => s.setPreviousPath);
  useEffect(() => {
    markAppVisited(activeApp.id);
    if (activeApp.id !== "desktop") {
      setPreviousPath(activeApp.basePath);
    }
  }, [activeApp.id, activeApp.basePath, markAppVisited, setPreviousPath]);

  const handleSidekickHeaderTargetChange = useCallback(
    (node: HTMLDivElement | null) => {
      setSidekickHeaderTarget((current) => (current === node ? current : node));
    },
    [],
  );

  const handleSidekickPanelTargetChange = useCallback(
    (node: HTMLDivElement | null) => {
      setSidekickPanelTarget((current) => (current === node ? current : node));
    },
    [],
  );

  const handleSidekickResizeEnd = useCallback((size: number) => {
    persistSidekickWidth(size);
  }, []);

  useLeftPanelWidthCssVar({
    leftPanelRef,
    isDesktop,
    activeAppId: activeApp.id,
  });

  return (
    <>
      <div
        className={styles.desktopShell}
        data-desktop-mode={desktopModeActive || undefined}
        data-agent-context="desktop-shell"
      >
        <BackgroundLayer />
        <DesktopTitlebar
          sidekickCollapsed={sidekickCollapsed}
          onToggleSidekick={toggleSidekick}
        />

        <div className={styles.desktopContent}>
          <div ref={leftPanelRef} className={styles.desktopSidebar}>
            <div className={styles.desktopSidebarBody}>
              <Lane
                resizable
                resizePosition="right"
                defaultWidth={200}
                maxWidth={600}
                storageKey="shell-sidebar"
                collapsible
                collapsed={isDesktop}
                animateCollapse={false}
                header={<SidebarSearchInput />}
              >
                <div
                  className={styles.panelActive}
                  data-agent-surface="left-panel"
                  data-agent-active-app-id={activeApp.id}
                  data-agent-active-app-label={activeApp.label}
                  aria-label={`${activeApp.label} navigation panel`}
                >
                  <activeApp.LeftPanel />
                </div>
              </Lane>
            </div>
          </div>

          <div
            className={
              sidekickHostCollapsed
                ? `${styles.mainPanelHost} ${styles.mainPanelHostNoSidekick}`
                : styles.mainPanelHost
            }
            data-agent-surface="main-panel"
            data-agent-active-app-id={activeApp.id}
            data-agent-active-app-label={activeApp.label}
            aria-label={`${activeApp.label} main panel`}
          >
            {activeApp.bareMainPanel ? (
              <MainPanel>{children}</MainPanel>
            ) : (
              <ResponsiveMainLane>
                <MainPanel>{children}</MainPanel>
              </ResponsiveMainLane>
            )}
          </div>
          {hasActiveSidekick && (
            <SidekickPortalBridge
              headerTarget={sidekickHeaderTarget}
              panelTarget={sidekickPanelTarget}
            />
          )}
          <PersistentSidekickLane
            resizeControlsRef={sidekickResizeControlsRef}
            collapsed={sidekickHostCollapsed}
            defaultWidth={sidekickInitialWidth}
            showHeaderSlot={showSidekickHeader}
            onResizeEnd={handleSidekickResizeEnd}
            onHeaderTargetChange={handleSidekickHeaderTargetChange}
            onPanelTargetChange={handleSidekickPanelTargetChange}
          />
        </div>
        <BottomTaskbar />
      </div>

      {appsModalOpen ? (
        <Suspense fallback={null}>
          <AppsModal isOpen={appsModalOpen} onClose={closeAppsModal} />
        </Suspense>
      ) : null}
      {backgroundModalOpen ? (
        <Suspense fallback={null}>
          <BackgroundModal
            isOpen={backgroundModalOpen}
            onClose={closeBackgroundModal}
          />
        </Suspense>
      ) : null}
      {settingsModalOpen ? (
        <SettingsModal
          isOpen={settingsModalOpen}
          onClose={closeSettingsModal}
        />
      ) : null}
    </>
  );
}
