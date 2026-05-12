import type { CSSProperties, ReactNode } from "react";
import { Lane } from "../Lane";
import styles from "./ResponsiveMainLane.module.css";

interface ResponsiveMainLaneProps {
  children: ReactNode;
  taskbar?: ReactNode;
  footer?: ReactNode;
  className?: string;
  style?: CSSProperties;
}

/**
 * Persistent middle-lane container so app switches swap inner content
 * without tearing down the visible chrome.
 *
 * The aura-os version branches on a `useAuraCapabilities().isMobileLayout`
 * media-query hook to swap to a mobile taskbar/footer. This shell drops the
 * mobile branch and always renders the desktop variant.
 */
export function ResponsiveMainLane({
  children,
  taskbar,
  footer,
  className,
  style,
}: ResponsiveMainLaneProps) {
  return (
    <Lane
      flex
      className={className}
      style={style}
      taskbar={taskbar}
      footer={footer}
    >
      <main className={styles.mainContent}>{children}</main>
    </Lane>
  );
}
