import { Button, useTheme } from "@cypher-asi/zui";
import { Sun, Moon } from "lucide-react";
import { ShellTitlebar } from "../ShellTitlebar";
import { WindowControls } from "../WindowControls";
import { UserMenuButton } from "./UserMenuButton";
import styles from "./DesktopShell.module.css";

interface DesktopTitlebarProps {
  sidekickCollapsed: boolean;
  onToggleSidekick: () => void;
}

const ICON_BY_KIND = {
  sun: Sun,
  moon: Moon,
} as const;

type ThemeKind = keyof typeof ICON_BY_KIND;
type ThemeValue = "light" | "dark" | "system";

function resolveIconKind(theme: ThemeValue, resolved: "light" | "dark"): ThemeKind {
  if (theme === "system") return resolved === "light" ? "sun" : "moon";
  return theme === "light" ? "sun" : "moon";
}

function nextTheme(theme: ThemeValue): ThemeValue {
  if (theme === "light") return "dark";
  if (theme === "dark") return "system";
  return "light";
}

function ThemeToggleButton() {
  const { theme, resolvedTheme, setTheme } = useTheme();
  const kind = resolveIconKind(theme as ThemeValue, resolvedTheme);
  const Icon = ICON_BY_KIND[kind];

  return (
    <span className="titlebar-no-drag">
      <Button
        variant="ghost"
        size="sm"
        rounded="md"
        iconOnly
        aria-label={`Theme: ${theme}`}
        onClick={() => setTheme(nextTheme(theme as ThemeValue))}
      >
        <Icon size={14} strokeWidth={2} />
      </Button>
    </span>
  );
}

export function DesktopTitlebar({
  sidekickCollapsed,
  onToggleSidekick,
}: DesktopTitlebarProps) {
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
        <span className={`titlebar-center ${styles.titleCenter}`}>Shell</span>
      }
      actions={
        <div
          className={styles.titleActions}
          onDoubleClick={(e) => e.stopPropagation()}
        >
          <UserMenuButton />
          <ThemeToggleButton />
          <WindowControls
            sidekickCollapsed={sidekickCollapsed}
            onToggleSidekick={onToggleSidekick}
          />
        </div>
      }
    />
  );
}
