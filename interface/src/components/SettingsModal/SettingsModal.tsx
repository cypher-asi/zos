import { Modal } from "@cypher-asi/zui";
import { AppearanceSection } from "../Settings/AppearanceSection";
import styles from "./SettingsModal.module.css";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Top-level Settings modal mounted from `DesktopShell` and opened from the
 * gear button in `BottomTaskbar`. Currently scoped to the Appearance section
 * (theme + accent + custom token editor + presets); future settings (e.g.
 * Notifications, Keyboard) can be added by replacing the body with a
 * two-column layout + Navigator.
 */
export function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
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
        <AppearanceSection />
      </div>
    </Modal>
  );
}
