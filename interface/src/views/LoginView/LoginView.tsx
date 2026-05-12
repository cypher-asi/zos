import { Panel, Heading, Text, Button } from "@cypher-asi/zui";
import { useLoginForm } from "./use-login-form";
import { LoginForm } from "./LoginForm";
import { ResetPasswordForm } from "./ResetPasswordForm";
import {
  isDevBypassAvailable,
  useAuthStore,
} from "../../stores/auth-store";
import { ShellTitlebar } from "../../components/ShellTitlebar";
import { WindowControls } from "../../components/WindowControls";
import { useShellCapabilities } from "../../hooks/use-shell-capabilities";
import styles from "./LoginView.module.css";

export function LoginView() {
  const f = useLoginForm();
  const bypassLogin = useAuthStore((s) => s.bypassLogin);
  const { hasDesktopBridge } = useShellCapabilities();
  const showDevBypass = isDevBypassAvailable() && !f.showResetPassword;

  return (
    <div className={styles.page}>
      {hasDesktopBridge ? (
        <ShellTitlebar
          icon={<span className={styles.titlebarLeading} aria-hidden="true" />}
          title={
            <span className="titlebar-center">
              <span className={styles.titlebarBrand}>ZERO</span>
            </span>
          }
          actions={<WindowControls />}
        />
      ) : null}

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

          <Text align="center" className={styles.cardTitle}>
            Login to ZERO
          </Text>

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

          {showDevBypass && (
            <div className={styles.bypass}>
              <Text size="xs" variant="muted" align="center">
                Development build
              </Text>
              <Button
                type="button"
                variant="ghost"
                onClick={bypassLogin}
                className={styles.bypassButton}
              >
                Skip login (dev bypass — no real session)
              </Button>
            </div>
          )}
        </Panel>
      </div>
    </div>
  );
}
