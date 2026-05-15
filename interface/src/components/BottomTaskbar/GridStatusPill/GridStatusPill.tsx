import { useQuery } from "@tanstack/react-query";
import { Spinner } from "@cypher-asi/zui";
import { gridApi } from "../../../shared/api/grid";
import { useAppUIStore } from "../../../stores/app-ui-store";
import styles from "./GridStatusPill.module.css";

const GRID_STATUS_KEY = ["grid", "status"] as const;

/**
 * Compact glanceable Grid connection indicator for the BottomTaskbar.
 * Shares its query cache with `Settings/NetworkSection` via the
 * identical `["grid","status"]` key so both surfaces stay in sync
 * without redundant fetches. Click opens Settings → Network.
 */
export function GridStatusPill() {
  const openSettingsModal = useAppUIStore((s) => s.openSettingsModal);
  const setSettingsActiveSection = useAppUIStore(
    (s) => s.setSettingsActiveSection,
  );

  const statusQuery = useQuery({
    queryKey: GRID_STATUS_KEY,
    queryFn: gridApi.status,
    refetchInterval: 5000,
  });

  const status = statusQuery.data;
  const showInitialSpinner = statusQuery.isPending && !status;
  const connected = status?.connected ?? false;

  const handleClick = (): void => {
    setSettingsActiveSection("network");
    openSettingsModal();
  };

  let label: string;
  let dotClass: string;
  if (showInitialSpinner) {
    label = "Grid…";
    dotClass = styles.dotPending;
  } else if (connected) {
    label = "Connected";
    dotClass = styles.dotConnected;
  } else {
    label = "Disconnected";
    dotClass = styles.dotDisconnected;
  }

  const tooltipParts: string[] = [];
  if (status?.multiaddr) tooltipParts.push(`Grid: ${status.multiaddr}`);
  if (status?.identity_id) tooltipParts.push(`Identity: ${status.identity_id}`);
  if (status?.last_error) tooltipParts.push(`Last error: ${status.last_error}`);
  const tooltip =
    tooltipParts.length > 0
      ? tooltipParts.join("\n")
      : showInitialSpinner
        ? "Checking Grid status…"
        : connected
          ? "Grid connected"
          : "Grid disconnected — click to configure";

  return (
    <button
      type="button"
      className={styles.pill}
      onClick={handleClick}
      title={tooltip}
      aria-label={`Grid status: ${label}. Open network settings.`}
      data-testid="grid-status-pill"
    >
      {showInitialSpinner ? (
        <span className={styles.spinner} aria-hidden="true">
          <Spinner size="sm" />
        </span>
      ) : (
        <span className={`${styles.dot} ${dotClass}`} aria-hidden="true" />
      )}
      <span className={styles.label}>{label}</span>
    </button>
  );
}
