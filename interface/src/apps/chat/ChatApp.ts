import { MessageSquare } from "lucide-react";
import { ChatList } from "./ChatList";
import { ChatMainPanel } from "./ChatMainPanel";
import type { ShellApp } from "../../shell/types";

export const ChatApp: ShellApp = {
  id: "chat",
  label: "Chat",
  icon: MessageSquare,
  basePath: "/chat",
  LeftPanel: ChatList,
  MainPanel: ChatMainPanel,
  searchPlaceholder: "Search",
};
