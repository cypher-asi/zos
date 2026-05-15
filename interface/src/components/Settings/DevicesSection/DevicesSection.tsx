import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button, Panel, Spinner, Text } from "@cypher-asi/zui";
import { Cpu } from "lucide-react";
import { devicesApi } from "../../../shared/api/devices";
import { ApiClientError } from "../../../shared/api/core";
import { Capability, type DeviceDto } from "../../../shared/types";
import styles from "./DevicesSection.module.css";

const DEVICES_KEY = ["devices"] as const;
const DEFAULT_CAPS = Capability.SEND | Capability.RECEIVE;

const CAPABILITY_LABELS: ReadonlyArray<{ bit: number; label: string }> = [
  { bit: Capability.SEND, label: "send" },
  { bit: Capability.RECEIVE, label: "receive" },
  { bit: Capability.MANAGE_MACHINES, label: "manage machines" },
  { bit: Capability.MANAGE_GROUPS, label: "manage groups" },
  { bit: Capability.ROTATE_EPOCH, label: "rotate epoch" },
];

function formatDate(unixMs: number): string {
  if (!unixMs) return "—";
  return new Date(unixMs).toLocaleDateString();
}

function truncateHex(hex: string): string {
  if (hex.length <= 12) return hex;
  return `${hex.slice(0, 6)}…${hex.slice(-4)}`;
}

function decodeCapabilities(bits: number): string[] {
  return CAPABILITY_LABELS.filter(({ bit }) => (bits & bit) !== 0).map(
    ({ label }) => label,
  );
}

export function DevicesSection() {
  const queryClient = useQueryClient();

  const devicesQuery = useQuery({
    queryKey: DEVICES_KEY,
    queryFn: devicesApi.list,
  });

  const createMutation = useMutation({
    mutationFn: () => devicesApi.create({ capabilities: DEFAULT_CAPS }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: DEVICES_KEY });
    },
  });

  const devices = devicesQuery.data ?? [];
  const error = createMutation.error;
  const isIdentityMissing =
    error instanceof ApiClientError && error.body.code === "identity_missing";
  const errorMessage = isIdentityMissing
    ? "Create a Neural Key first."
    : error instanceof ApiClientError
      ? error.body.error
      : error instanceof Error
        ? error.message
        : null;

  return (
    <Panel
      variant="solid"
      border="solid"
      borderRadius="md"
      className={styles.devicesPanel}
      data-testid="settings-devices-panel"
    >
      <div className={styles.header}>
        <Text weight="semibold" size="sm">
          Devices
        </Text>
        <Button
          size="sm"
          variant="filled"
          icon={<Cpu size={14} />}
          onClick={() => createMutation.mutate()}
          disabled={createMutation.isPending}
        >
          {createMutation.isPending ? "Deriving…" : "Derive Machine Key"}
        </Button>
      </div>

      <Text variant="muted" size="xs">
        Machines registered to this identity. New devices default to{" "}
        <code>SEND + RECEIVE</code>.
      </Text>

      {devicesQuery.isLoading && (
        <div className={styles.loading}>
          <Spinner size="sm" />
        </div>
      )}

      {!devicesQuery.isLoading && devices.length === 0 && (
        <Text variant="muted" size="sm">
          No machine keys yet.
        </Text>
      )}

      {!devicesQuery.isLoading && devices.length > 0 && (
        <div className={styles.tableWrapper}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th className={styles.th}>Machine ID</th>
                <th className={styles.th}>Capabilities</th>
                <th className={styles.th}>Created</th>
              </tr>
            </thead>
            <tbody>
              {devices.map((device: DeviceDto) => {
                const caps = decodeCapabilities(device.capabilities);
                return (
                  <tr key={device.machine_id} className={styles.row}>
                    <td className={styles.td}>
                      <code
                        className={styles.mono}
                        title={device.machine_id}
                      >
                        {truncateHex(device.machine_id)}
                      </code>
                    </td>
                    <td className={styles.td}>
                      <div className={styles.chips}>
                        {caps.length === 0 ? (
                          <span className={styles.chipMuted}>none</span>
                        ) : (
                          caps.map((c) => (
                            <span key={c} className={styles.chip}>
                              {c}
                            </span>
                          ))
                        )}
                      </div>
                    </td>
                    <td className={styles.td}>
                      <Text size="xs">{formatDate(device.created_at)}</Text>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {errorMessage && (
        <Text variant="muted" size="xs" className={styles.errorText}>
          {errorMessage}
        </Text>
      )}
    </Panel>
  );
}
