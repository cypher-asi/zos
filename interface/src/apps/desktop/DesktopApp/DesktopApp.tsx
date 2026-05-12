import { Circle } from "lucide-react";
import type { ReactNode } from "react";
import type { AuraAppModule } from "../../../shell/types";
import { useAppUIStore } from "../../../stores/app-ui-store";
import { useSelectionMarquee } from "./useSelectionMarquee";
import styles from "./DesktopApp.module.css";

function EmptyPanel() {
  return null;
}

function MainPanel({ children }: { children?: ReactNode }) {
  const { rect, handlers } = useSelectionMarquee();
  const openBackgroundModal = useAppUIStore((s) => s.openBackgroundModal);

  const onContextMenu = (event: React.MouseEvent) => {
    if (event.target !== event.currentTarget) return;
    event.preventDefault();
    openBackgroundModal();
  };

  return (
    <div className={styles.surface} {...handlers} onContextMenu={onContextMenu}>
      {rect && (
        <div
          className={styles.marquee}
          style={{
            left: rect.left,
            top: rect.top,
            width: rect.width,
            height: rect.height,
          }}
        />
      )}
      {children}
    </div>
  );
}

export const DesktopApp: AuraAppModule = {
  id: "desktop",
  label: "Desktop",
  icon: Circle,
  basePath: "/desktop",
  LeftPanel: EmptyPanel,
  MainPanel,
};
