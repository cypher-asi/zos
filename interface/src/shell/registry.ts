import { ChatApp } from "../apps/chat/ChatApp";
import { ExploreApp } from "../apps/explore";
import { DesktopApp } from "../apps/desktop/DesktopApp";
import type { ShellApp } from "./types";

export const apps: ShellApp[] = [ChatApp, DesktopApp, ExploreApp];
