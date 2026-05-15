import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button, Panel, Spinner, Text } from "@cypher-asi/zui";
import { KeyRound } from "lucide-react";
import { identityApi } from "../../../shared/api/identity";
import { ApiClientError } from "../../../shared/api/core";
import styles from "./IdentitySection.module.css";

const IDENTITY_KEY = ["identity"] as const;

function formatDate(unixMs: number): string {
  if (!unixMs) return "—";
  return new Date(unixMs).toLocaleString();
}

export function IdentitySection() {
  const queryClient = useQueryClient();

  const identityQuery = useQuery({
    queryKey: IDENTITY_KEY,
    queryFn: identityApi.get,
  });

  const createMutation = useMutation({
    mutationFn: identityApi.create,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: IDENTITY_KEY });
    },
  });

  const identity = identityQuery.data;
  const error = createMutation.error;
  const errorMessage =
    error instanceof ApiClientError
      ? error.body.error
      : error instanceof Error
        ? error.message
        : null;

  return (
    <Panel
      variant="solid"
      border="solid"
      borderRadius="md"
      className={styles.identityPanel}
      data-testid="settings-identity-panel"
    >
      <Text weight="semibold" size="sm">
        Identity
      </Text>

      {identityQuery.isLoading && (
        <div className={styles.loading}>
          <Spinner size="sm" />
        </div>
      )}

      {!identityQuery.isLoading && !identity && (
        <div className={styles.empty}>
          <Text variant="muted" size="sm">
            You don't have a Neural Key yet. Create one to participate in
            THE GRID.
          </Text>
          <Button
            size="md"
            variant="filled"
            icon={<KeyRound size={16} />}
            onClick={() => createMutation.mutate()}
            disabled={createMutation.isPending}
          >
            {createMutation.isPending ? "Creating…" : "Create Neural Key"}
          </Button>
        </div>
      )}

      {!identityQuery.isLoading && identity && (
        <div className={styles.details}>
          <div className={styles.field}>
            <Text variant="muted" size="xs">
              Identity ID
            </Text>
            <Text size="sm" className={styles.mono}>
              {identity.identity_id}
            </Text>
          </div>
          <div className={styles.field}>
            <Text variant="muted" size="xs">
              Epoch
            </Text>
            <Text size="sm">{identity.epoch}</Text>
          </div>
          <div className={styles.field}>
            <Text variant="muted" size="xs">
              Created
            </Text>
            <Text size="sm">{formatDate(identity.created_at)}</Text>
          </div>
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
