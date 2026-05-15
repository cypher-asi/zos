import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button, Input, Panel, Spinner, Text } from "@cypher-asi/zui";
import { Plug, Unplug } from "lucide-react";
import { gridApi } from "../../../shared/api/grid";
import { ApiClientError } from "../../../shared/api/core";
import styles from "./NetworkSection.module.css";

const GRID_STATUS_KEY = ["grid", "status"] as const;

export function NetworkSection() {
  const queryClient = useQueryClient();

  const statusQuery = useQuery({
    queryKey: GRID_STATUS_KEY,
    queryFn: gridApi.status,
    refetchInterval: 5000,
  });

  // The draft is `null` until the user types — until then we display the
  // server value as-is. Once edited, `draft` wins so the user's in-flight
  // input is never blown away by a background `refetchInterval` poll.
  const [draft, setDraft] = useState<string | null>(null);
  const serverMultiaddr = statusQuery.data?.multiaddr ?? "";
  const multiaddr = draft ?? serverMultiaddr;
  const isDirty = draft !== null && draft !== serverMultiaddr;

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: GRID_STATUS_KEY });

  const saveMutation = useMutation({
    mutationFn: (next: string) => gridApi.setConfig({ multiaddr: next }),
    onSuccess: () => {
      setDraft(null);
      return invalidate();
    },
  });

  const connectMutation = useMutation({
    mutationFn: gridApi.connect,
    onSuccess: invalidate,
  });

  const disconnectMutation = useMutation({
    mutationFn: gridApi.disconnect,
    onSuccess: invalidate,
  });

  const status = statusQuery.data;
  const connected = status?.connected ?? false;
  const busy =
    saveMutation.isPending ||
    connectMutation.isPending ||
    disconnectMutation.isPending;
  const mutationError =
    saveMutation.error ?? connectMutation.error ?? disconnectMutation.error;
  const errorMessage =
    mutationError instanceof ApiClientError
      ? mutationError.body.error
      : mutationError instanceof Error
        ? mutationError.message
        : null;

  return (
    <Panel
      variant="solid"
      border="solid"
      borderRadius="md"
      className={styles.networkPanel}
      data-testid="settings-network-panel"
    >
      <Text weight="semibold" size="sm">
        Network
      </Text>

      <div className={styles.section}>
        <Text variant="muted" size="sm">
          Status
        </Text>
        <div className={styles.statusRow}>
          <span
            className={`${styles.pill} ${connected ? styles.pillConnected : styles.pillDisconnected}`}
          >
            {connected ? "Connected" : "Disconnected"}
          </span>
          {statusQuery.isLoading && <Spinner size="sm" />}
        </div>
        {status?.identity_id && (
          <Text variant="muted" size="xs" className={styles.mono}>
            Identity: {status.identity_id}
          </Text>
        )}
        {status?.last_error && (
          <Text variant="muted" size="xs">
            Last error: {status.last_error}
          </Text>
        )}
      </div>

      <div className={styles.section}>
        <Text variant="muted" size="sm">
          GRID multiaddr
        </Text>
        <div className={styles.inputRow}>
          <Input
            value={multiaddr}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="/ip4/127.0.0.1/udp/3690/quic-v1"
            mono
            className={styles.input}
            disabled={busy}
          />
          <Button
            size="sm"
            variant="filled"
            onClick={() => saveMutation.mutate(multiaddr)}
            disabled={busy || multiaddr.trim().length === 0 || !isDirty}
          >
            Save
          </Button>
        </div>
      </div>

      <div className={styles.section}>
        <Text variant="muted" size="sm">
          Connection
        </Text>
        <div className={styles.buttonRow}>
          <Button
            size="sm"
            variant="filled"
            icon={<Plug size={14} />}
            onClick={() => connectMutation.mutate()}
            disabled={busy}
          >
            Connect
          </Button>
          <Button
            size="sm"
            variant="ghost"
            icon={<Unplug size={14} />}
            onClick={() => disconnectMutation.mutate()}
            disabled={busy}
          >
            Disconnect
          </Button>
        </div>
      </div>

      {errorMessage && (
        <Text variant="muted" size="xs" className={styles.errorText}>
          {errorMessage}
        </Text>
      )}
    </Panel>
  );
}
