import { useMemo, useState, useEffect } from "react";
import {
  Circle,
  ChevronRight,
  ChevronLeft,
  LayoutGrid,
  LogOut,
  Moon,
  Settings,
  Sun,
  User,
} from "lucide-react";
import { MenuDropdown, useTheme, type MenuDropdownItem } from "@cypher-asi/zui";
import { useActiveApp } from "../../hooks/use-active-app";
import { useAppUIStore } from "../../stores/app-ui-store";
import { useAuth } from "../../stores/auth-store";
import { getTaskbarAppsCollapsed, setTaskbarAppsCollapsed } from "../../utils/storage";
import { useNavigate } from "../../lib/router-adapter";
import { AppNavRail, TaskbarIconButton, TASKBAR_ICON_SIZE } from "../AppNavRail";
import { GridStatusPill } from "./GridStatusPill";
import styles from "./BottomTaskbar.module.css";

const TASKBAR_CHEVRON_SIZE = TASKBAR_ICON_SIZE + 1;

const THEME_ICON_BY_KIND = {
  sun: Sun,
  moon: Moon,
} as const;

type ThemeKind = keyof typeof THEME_ICON_BY_KIND;
type ThemeValue = "light" | "dark" | "system";

function resolveThemeIconKind(
  theme: ThemeValue,
  resolved: "light" | "dark",
): ThemeKind {
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
  const kind = resolveThemeIconKind(theme as ThemeValue, resolvedTheme);
  const Icon = THEME_ICON_BY_KIND[kind];

  return (
    <TaskbarIconButton
      icon={<Icon size={TASKBAR_ICON_SIZE} />}
      title={`Theme: ${theme}`}
      aria-label={`Theme: ${theme}`}
      onClick={() => setTheme(nextTheme(theme as ThemeValue))}
    />
  );
}

function UserMenuButton() {
  const { user, isBypass, logout } = useAuth();

  const items: MenuDropdownItem[] = useMemo(
    () => [
      {
        id: "signed-in-as",
        label: user
          ? `Signed in as ${user.display_name || user.primary_zid || "user"}${
              isBypass ? " (dev bypass)" : ""
            }`
          : "Not signed in",
        icon: <User size={14} />,
        disabled: true,
        onClick: () => undefined,
      },
      { id: "divider", label: "", divider: true, onClick: () => undefined },
      {
        id: "logout",
        label: "Sign out",
        icon: <LogOut size={14} />,
        danger: true,
        onClick: () => {
          void logout();
        },
      },
    ],
    [user, isBypass, logout],
  );

  if (!user) return null;

  const label = user.display_name || user.primary_zid || "Account";

  return (
    <MenuDropdown
      items={items}
      align="right"
      trigger={
        <TaskbarIconButton
          icon={<User size={TASKBAR_ICON_SIZE} />}
          title={label}
          aria-label={`Account menu for ${label}`}
        />
      }
    />
  );
}

function useClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 30_000);
    return () => clearInterval(id);
  }, []);
  return now.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

export function BottomTaskbar() {
  const openAppsModal = useAppUIStore((s) => s.openAppsModal);
  const openSettingsModal = useAppUIStore((s) => s.openSettingsModal);
  const activeApp = useActiveApp();
  const time = useClock();
  const navigate = useNavigate();
  const previousPath = useAppUIStore((s) => s.previousPath);
  const [collapsed, setCollapsed] = useState(() => getTaskbarAppsCollapsed());

  const toggleAppsCollapsed = () => {
    setCollapsed((current) => {
      const next = !current;
      setTaskbarAppsCollapsed(next);
      return next;
    });
  };

  return (
    <div
      className={styles.bar}
      data-agent-surface="desktop-shell-bottom-taskbar"
    >
      <div className={styles.left}>
        <TaskbarIconButton
          selected={activeApp.id === "desktop"}
          icon={<Circle size={TASKBAR_ICON_SIZE} />}
          title="Desktop"
          aria-label="Desktop"
          onClick={() => {
            if (activeApp.id === "desktop") {
              if (previousPath) navigate(previousPath);
            } else {
              navigate("/desktop");
            }
          }}
        />
      </div>

      <div className={styles.center}>
        <AppNavRail
          layout="taskbar"
          allowReorder
          {...(collapsed && { includeIds: ["projects"] })}
        />
        <TaskbarIconButton
          icon={<LayoutGrid size={TASKBAR_ICON_SIZE} />}
          title="Apps"
          aria-label="Apps"
          onClick={openAppsModal}
        />
        <TaskbarIconButton
          icon={
            collapsed ? (
              <ChevronRight size={TASKBAR_CHEVRON_SIZE} />
            ) : (
              <ChevronLeft size={TASKBAR_CHEVRON_SIZE} />
            )
          }
          onClick={toggleAppsCollapsed}
          aria-label={collapsed ? "Expand apps" : "Collapse apps"}
        />
      </div>

      <div className={styles.right}>
        <GridStatusPill />
        <ThemeToggleButton />
        <UserMenuButton />
        <TaskbarIconButton
          icon={<Settings size={TASKBAR_ICON_SIZE} />}
          title="Settings"
          aria-label="Settings"
          onClick={openSettingsModal}
        />
        <span className={styles.clock}>{time}</span>
      </div>
    </div>
  );
}
