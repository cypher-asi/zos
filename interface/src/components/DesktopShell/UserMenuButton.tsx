import { useMemo } from "react";
import { Button, MenuDropdown, type MenuDropdownItem } from "@cypher-asi/zui";
import { LogOut, User } from "lucide-react";
import { useAuth } from "../../stores/auth-store";

function initialsFor(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return "?";
  const parts = trimmed.split(/\s+/);
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase();
  return (parts[0]![0]! + parts[parts.length - 1]![0]!).toUpperCase();
}

/**
 * Compact user menu rendered into the desktop titlebar. Single source of
 * `useAuthStore.logout()` so the user can actually end their session — the
 * shell otherwise has no exit path. Hides itself when no user is signed in
 * (defensive; shell routes are auth-gated so it normally always has one).
 */
export function UserMenuButton() {
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
    <span className="titlebar-no-drag">
      <MenuDropdown
        items={items}
        align="right"
        trigger={
          <Button
            variant="ghost"
            size="sm"
            rounded="md"
            aria-label={`Account menu for ${label}`}
            title={label}
          >
            {initialsFor(label)}
          </Button>
        }
      />
    </span>
  );
}
