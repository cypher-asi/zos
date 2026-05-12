import { ChatApp } from "../apps/chat/ChatApp";
import { ZeroApp } from "../apps/zero";
import { ProjectsApp } from "../apps/projects";
import { ExploreApp } from "../apps/explore";
import { DesktopApp } from "../apps/desktop/DesktopApp";
import type { ShellApp } from "./types";

export const apps: ShellApp[] = [
  ChatApp,
  ZeroApp,
  DesktopApp,
  ProjectsApp,
  ExploreApp,
];
