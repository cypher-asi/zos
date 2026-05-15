import { useMemo } from "react";
import { Modal, Navigator } from "@cypher-asi/zui";
import type { NavigatorItemProps } from "@cypher-asi/zui";
import {
  SETTINGS_SECTIONS,
  getSettingsSection,
  isSettingsSectionId,
  DEFAULT_SETTINGS_SECTION,
} from "../Settings";
import { useAppUIStore } from "../../stores/app-ui-store";
import styles from "./SettingsModal.module.css";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Top-level Settings modal mounted from `DesktopShell` and opened from the
 * gear button in `BottomTaskbar`. Renders a two-column layout: a left
 * `Navigator` for the section list (Appearance, Network, Identity, Devices)
 * and a right pane that mounts the active section's component. The active
 * section persists in `useAppUIStore` so reopening returns to the last view.
 */
export function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
  const activeSection = useAppUIStore((s) => s.settingsActiveSection);
  const setActiveSection = useAppUIStore((s) => s.setSettingsActiveSection);

  const navItems = useMemo<NavigatorItemProps[]>(
    () =>
      SETTINGS_SECTIONS.map((s) => {
        const Icon = s.icon;
        return { id: s.id, label: s.label, icon: <Icon size={14} /> };
      }),
    [],
  );

  const safeId = isSettingsSectionId(activeSection)
    ? activeSection
    : DEFAULT_SETTINGS_SECTION;
  const { Pane } = getSettingsSection(safeId);

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Settings"
      size="lg"
      fullHeight
      noPadding
    >
      <div className={styles.body}>
        <aside className={styles.nav}>
          <Navigator
            items={navItems}
            value={safeId}
            onChange={(id) => {
              if (isSettingsSectionId(id)) {
                setActiveSection(id);
              }
            }}
          />
        </aside>
        <section className={styles.content}>
          <Pane />
        </section>
      </div>
    </Modal>
  );
}
