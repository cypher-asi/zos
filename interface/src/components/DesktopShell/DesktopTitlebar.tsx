import { ShellTitlebar } from "../ShellTitlebar";
import { WindowControls } from "../WindowControls";
import { useActiveApp } from "../../hooks/use-active-app";
import styles from "./DesktopShell.module.css";

interface DesktopTitlebarProps {
  sidekickCollapsed: boolean;
  onToggleSidekick: () => void;
}

export function DesktopTitlebar({
  sidekickCollapsed,
  onToggleSidekick,
}: DesktopTitlebarProps) {
  const activeApp = useActiveApp();
  const inApp = activeApp.id !== "desktop";
  const titleLabel = inApp ? "ZERO" : "Shell";
  return (
    <ShellTitlebar
      icon={
        <span className={`${styles.titleLeading} titlebar-no-drag`}>
          {/* Reserved leading slot — keep empty so ShellTitlebar's first-child
              alignment rule still aligns the next pill (BottomTaskbar.left)
              with the bottom-left Desktop button. */}
        </span>
      }
      title={
        <span className={`titlebar-center ${styles.titleCenter}`}>
          {titleLabel}
        </span>
      }
      actions={
        <div
          className={styles.titleActions}
          onDoubleClick={(e) => e.stopPropagation()}
        >
          <WindowControls
            sidekickCollapsed={sidekickCollapsed}
            onToggleSidekick={onToggleSidekick}
          />
        </div>
      }
    />
  );
}
