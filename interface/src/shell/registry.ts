import { ZeroApp } from "../apps/zero";
import { ProjectsApp } from "../apps/projects";
import { ExploreApp } from "../apps/explore";
import { SettingsApp } from "../apps/settings";
import { DesktopApp } from "../apps/desktop/DesktopApp";
import type { ShellApp } from "./types";

export const apps: ShellApp[] = [
  ZeroApp,
  DesktopApp,
  ProjectsApp,
  ExploreApp,
  SettingsApp,
];
