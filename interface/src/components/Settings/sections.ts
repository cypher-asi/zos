import type { ComponentType } from "react";
import type { LucideIcon } from "lucide-react";
import { Fingerprint, Network, Paintbrush, Smartphone } from "lucide-react";
import { AppearanceSection } from "./AppearanceSection";
import { NetworkSection } from "./NetworkSection";
import { IdentitySection } from "./IdentitySection";
import { DevicesSection } from "./DevicesSection";

export type SettingsSectionId =
  | "appearance"
  | "network"
  | "identity"
  | "devices";

export type SettingsSection = {
  readonly id: SettingsSectionId;
  readonly label: string;
  readonly icon: LucideIcon;
  readonly Pane: ComponentType;
};

export const SETTINGS_SECTIONS: readonly SettingsSection[] = [
  { id: "appearance", label: "Appearance", icon: Paintbrush, Pane: AppearanceSection },
  { id: "network", label: "Network", icon: Network, Pane: NetworkSection },
  { id: "identity", label: "Identity", icon: Fingerprint, Pane: IdentitySection },
  { id: "devices", label: "Devices", icon: Smartphone, Pane: DevicesSection },
];

export const DEFAULT_SETTINGS_SECTION: SettingsSectionId = "appearance";

export function isSettingsSectionId(value: string): value is SettingsSectionId {
  return SETTINGS_SECTIONS.some((s) => s.id === value);
}

export function getSettingsSection(id: SettingsSectionId): SettingsSection {
  const found = SETTINGS_SECTIONS.find((s) => s.id === id);
  if (!found) {
    throw new Error(`Unknown settings section id: ${id}`);
  }
  return found;
}
