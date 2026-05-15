import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button, Input, Panel, Spinner, Text } from "@cypher-asi/zui";
import { Plug, Unplug } from "lucide-react";
import { gridApi } from "../../../shared/api/grid";
import { ApiClientError } from "../../../shared/api/core";
import styles from "./NetworkSection.module.css";

const GRID_STATUS_KEY = ["grid", "status"] as const;

/**
 * Default the SDK falls back to when no override is persisted. Mirrors
 * `DEFAULT_CONNECT_TIMEOUT_MS` on the Rust side; surfaced here only to
 * pre-fill the input so the user sees the *actual* effective value.
 */
const DEFAULT_TIMEOUT_MS = 30_000;
const MIN_TIMEOUT_S = 1;
const MAX_TIMEOUT_S = 300;

function effectiveTimeoutSeconds(timeoutMs: number | null | undefined): string {
  const ms = timeoutMs ?? DEFAULT_TIMEOUT_MS;
  return String(Math.round(ms / 1000));
}

export function NetworkSection() {
  const queryClient = useQueryClient();

  const statusQuery = useQuery({
    queryKey: GRID_STATUS_KEY,
    queryFn: gridApi.status,
    refetchInterval: 5000,
  });

  // The drafts are `null` until the user types — until then we display the
  // server value as-is. Once edited, the draft wins so the user's in-flight
  // input is never blown away by a background `refetchInterval` poll.
  const [draftMultiaddr, setDraftMultiaddr] = useState<string | null>(null);
  const [draftTimeout, setDraftTimeout] = useState<string | null>(null);

  const serverMultiaddr = statusQuery.data?.multiaddr ?? "";
  const multiaddr = draftMultiaddr ?? serverMultiaddr;
  const isMultiaddrDirty =
    draftMultiaddr !== null && draftMultiaddr !== serverMultiaddr;

  const serverTimeoutSeconds = useMemo(
    () => effectiveTimeoutSeconds(statusQuery.data?.connect_timeout_ms),
    [statusQuery.data?.connect_timeout_ms],
  );
  const timeoutInput = draftTimeout ?? serverTimeoutSeconds;
  const isTimeoutDirty =
    draftTimeout !== null && draftTimeout !== serverTimeoutSeconds;

  const timeoutValidationError: string | null = (() => {
    if (!isTimeoutDirty) return null;
    const trimmed = timeoutInput.trim();
    if (trimmed.length === 0) return null; // empty == "use default", valid
    if (!/^\d+$/.test(trimmed)) return "Enter a whole number of seconds";
    const n = Number(trimmed);
    if (n < MIN_TIMEOUT_S || n > MAX_TIMEOUT_S) {
      return `Must be between ${MIN_TIMEOUT_S} and ${MAX_TIMEOUT_S} seconds`;
    }
    return null;
  })();

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: GRID_STATUS_KEY });

  const saveMultiaddrMutation = useMutation({
    mutationFn: (next: string) => gridApi.setConfig({ multiaddr: next }),
    onSuccess: () => {
      setDraftMultiaddr(null);
      return invalidate();
    },
  });

  const saveTimeoutMutation = useMutation({
    mutationFn: (timeoutMs: number | null) =>
      gridApi.setTimeout({ timeout_ms: timeoutMs }),
    onSuccess: () => {
      setDraftTimeout(null);
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
    saveMultiaddrMutation.isPending ||
    saveTimeoutMutation.isPending ||
    connectMutation.isPending ||
    disconnectMutation.isPending;
  const mutationError =
    saveMultiaddrMutation.error ??
    saveTimeoutMutation.error ??
    connectMutation.error ??
    disconnectMutation.error;
  const errorMessage =
    mutationError instanceof ApiClientError
      ? mutationError.body.error
      : mutationError instanceof Error
        ? mutationError.message
        : null;

  const handleSaveTimeout = () => {
    const trimmed = timeoutInput.trim();
    // Empty input means "clear the override; use SDK default".
    if (trimmed.length === 0) {
      saveTimeoutMutation.mutate(null);
      return;
    }
    const seconds = Number(trimmed);
    saveTimeoutMutation.mutate(seconds * 1000);
  };

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
            onChange={(e) => setDraftMultiaddr(e.target.value)}
            placeholder="/ip4/127.0.0.1/udp/3690/quic-v1"
            mono
            className={styles.input}
            disabled={busy}
          />
          <Button
            size="sm"
            variant="filled"
            onClick={() => saveMultiaddrMutation.mutate(multiaddr)}
            disabled={busy || multiaddr.trim().length === 0 || !isMultiaddrDirty}
          >
            Save
          </Button>
        </div>
      </div>

      <div className={styles.section}>
        <Text variant="muted" size="sm">
          Connect timeout (seconds)
        </Text>
        <div className={styles.inputRow}>
          <Input
            value={timeoutInput}
            onChange={(e) => setDraftTimeout(e.target.value)}
            placeholder="30"
            mono
            className={styles.timeoutInput}
            disabled={busy}
            inputMode="numeric"
            data-testid="settings-network-timeout-input"
          />
          <Button
            size="sm"
            variant="filled"
            onClick={handleSaveTimeout}
            disabled={
              busy || !isTimeoutDirty || timeoutValidationError !== null
            }
          >
            Save
          </Button>
        </div>
        <Text variant="muted" size="xs">
          {status?.connect_timeout_ms == null
            ? `Using SDK default (${MIN_TIMEOUT_S}–${MAX_TIMEOUT_S}s; clear to reset).`
            : `Custom override active. Clear the field to revert to the SDK default.`}
        </Text>
        {timeoutValidationError && (
          <Text variant="muted" size="xs" className={styles.errorText}>
            {timeoutValidationError}
          </Text>
        )}
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
