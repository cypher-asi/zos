import { Panel, Heading, Text, Button } from "@cypher-asi/zui";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useLoginForm } from "./use-login-form";
import { LoginForm } from "./LoginForm";
import { ResetPasswordForm } from "./ResetPasswordForm";
import { useAuthStore } from "../../stores/auth-store";
import styles from "./LoginView.module.css";

export function LoginView() {
  const f = useLoginForm();
  const bypassLogin = useAuthStore((s) => s.bypassLogin);
  const navigate = useNavigate();
  const { redirect: redirectTo } = useSearch({ from: "/login" });

  function handleBypass() {
    bypassLogin();
    navigate({ to: redirectTo ?? "/", replace: true });
  }

  return (
    <div className={styles.page}>
      <div className={styles.container}>
        <Panel
          variant="solid"
          border="solid"
          borderRadius="lg"
          className={styles.card}
        >
          <div className={styles.header}>
            <Heading level={2}>
              <span className={styles.brand}>ZERO</span>
            </Heading>
            <Text variant="muted" size="sm" align="center" className={styles.subtitle}>
              Zero Identity Authentication
            </Text>
          </div>

          {f.showResetPassword ? (
            <ResetPasswordForm
              resetEmail={f.resetEmail}
              setResetEmail={f.setResetEmail}
              resetStatus={f.resetStatus}
              resetError={f.resetError}
              onSubmit={f.handleResetSubmit}
              onClose={f.closeResetPassword}
            />
          ) : (
            <LoginForm
              activeTab={f.activeTab}
              email={f.email}
              setEmail={f.setEmail}
              password={f.password}
              setPassword={f.setPassword}
              confirmPassword={f.confirmPassword}
              setConfirmPassword={f.setConfirmPassword}
              name={f.name}
              setName={f.setName}
              inviteCode={f.inviteCode}
              setInviteCode={f.setInviteCode}
              error={f.error}
              loading={f.loading}
              onTabChange={f.handleTabChange}
              onSubmit={f.handleSubmit}
              onForgotPassword={f.openResetPassword}
            />
          )}

          {!f.showResetPassword && (
            <div className={styles.bypass}>
              <Text size="xs" variant="muted" align="center">
                Development
              </Text>
              <Button
                type="button"
                variant="ghost"
                onClick={handleBypass}
                className={styles.bypassButton}
              >
                Skip login (dev bypass)
              </Button>
            </div>
          )}
        </Panel>
      </div>
    </div>
  );
}
