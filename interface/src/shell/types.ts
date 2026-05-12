import type { LucideIcon } from "lucide-react";
import type { ReactNode, ComponentType } from "react";

export interface ShellApp {
  id: string;
  label: string;
  /** Short phrase agents can use to recognize this app in the UI and prompts. */
  agentDescription?: string;
  /** Searchable keywords that help match changelog themes to this app. */
  agentKeywords?: string[];
  icon: LucideIcon;
  basePath: string;
  LeftPanel: ComponentType;
  /**
   * Wraps the routed `<Outlet />`. The shell renders this inside a persistent
   * `ResponsiveMainLane` (the "middle root panel") so that switching between
   * apps swaps inner content rather than tearing down the visible container.
   * Apps therefore return inner JSX only — no `<Lane>` / `<ResponsiveMainLane>`
   * wrapper of their own. To opt out (e.g. the Desktop wallpaper surface), set
   * `bareMainPanel: true` and the shell will render `MainPanel` directly inside
   * `mainPanelHost` without the persistent lane.
   */
  MainPanel: ComponentType<{ children?: ReactNode }>;
  /**
   * When true, the shell renders this app's `MainPanel` directly inside
   * `mainPanelHost` without wrapping it in the persistent `ResponsiveMainLane`.
   * Used by the Desktop app, whose marquee surface needs a transparent
   * full-bleed area over the wallpaper rather than a Lane chrome.
   */
  bareMainPanel?: boolean;
  SidekickPanel?: ComponentType;
  /** Rendered in the sidekick Lane's `header` slot (e.g. tab bar). */
  SidekickTaskbar?: ComponentType;
  /** Placeholder text shown in the sidebar search input when this app is active. */
  searchPlaceholder?: string;
  /**
   * When true, this app starts in the "Hidden" section of the Apps modal and
   * is omitted from the visible taskbar strip until the user explicitly drags
   * it into the visible section.
   */
  defaultHidden?: boolean;
  /** Called on hover/focus of the nav-rail/taskbar item to warm caches. */
  onPrefetch?: () => void;
}

/** Shape of the lazy-loaded module exported from each `apps/<name>/<Name>App.ts`. */
export type AuraAppModule = Omit<ShellApp, never>;
