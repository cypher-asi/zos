export const DEFAULT_SIDEKICK_WIDTH = 320;
export const SIDEKICK_MIN_WIDTH = 200;
export const SIDEKICK_MAX_WIDTH = 1200;

/**
 * Single shared storage key for the sidekick width. The sidekick is the same
 * pixel width across every app — switching apps does not change its size.
 */
export const SHARED_SIDEKICK_STORAGE_KEY = "shell-sidekick-width";

// Legacy key kept for one-time read-through migration so users keep the width
// they previously chose under the older shared scheme.
export const LEGACY_SHARED_SIDEKICK_STORAGE_KEY = "shell-sidekick-v2";

function clampSidekickWidth(width: number) {
  return Math.min(SIDEKICK_MAX_WIDTH, Math.max(SIDEKICK_MIN_WIDTH, width));
}

function parseStoredWidth(rawValue: string | null): number | null {
  if (rawValue == null) return null;
  const parsedValue = Number.parseInt(rawValue, 10);
  if (!Number.isFinite(parsedValue)) return null;
  return clampSidekickWidth(parsedValue);
}

/**
 * Read the stored sidekick width. Falls back to the legacy shared key so
 * existing users don't lose their preference on first load after the upgrade.
 * Returns DEFAULT_SIDEKICK_WIDTH when nothing is stored.
 */
export function readStoredSidekickWidth(): number {
  if (typeof window === "undefined") return DEFAULT_SIDEKICK_WIDTH;
  try {
    const sharedValue = parseStoredWidth(
      localStorage.getItem(SHARED_SIDEKICK_STORAGE_KEY),
    );
    if (sharedValue != null) return sharedValue;

    const legacyValue = parseStoredWidth(
      localStorage.getItem(LEGACY_SHARED_SIDEKICK_STORAGE_KEY),
    );
    if (legacyValue != null) return legacyValue;

    return DEFAULT_SIDEKICK_WIDTH;
  } catch {
    return DEFAULT_SIDEKICK_WIDTH;
  }
}

export function persistSidekickWidth(width: number): void {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(
      SHARED_SIDEKICK_STORAGE_KEY,
      String(clampSidekickWidth(width)),
    );
  } catch {
    // Ignore storage failures.
  }
}
