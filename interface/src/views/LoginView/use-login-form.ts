import { useEffect, useState, type FormEvent } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useAuth } from "../../stores/auth-store";
import { authApi } from "../../shared/api/auth";
import { ApiClientError } from "../../shared/api/core";

export type AuthTab = "signin" | "register";

export const AUTH_TABS = [
  { id: "signin", label: "Sign In" },
  { id: "register", label: "Create Account" },
];

function getAuthErrorMessage(err: unknown): string {
  if (err instanceof ApiClientError) {
    return err.body?.error || err.message || "Authentication failed";
  }
  if (err instanceof Error) return err.message;
  return "An unexpected error occurred";
}

export function useLoginForm() {
  const { login, register, isAuthenticated, isLoading } = useAuth();
  const navigate = useNavigate();
  const { redirect: redirectTo } = useSearch({ from: "/login" });
  const from = redirectTo ?? "/";

  // If a session is restored asynchronously while the user sits on /login
  // (e.g. desktop-side restoreSession completes after the form has mounted,
  // or a background tab logs in), get the user off the login page without
  // requiring a form submit.
  useEffect(() => {
    if (isLoading || !isAuthenticated) return;
    navigate({ to: from, replace: true });
  }, [from, isAuthenticated, isLoading, navigate]);

  const [activeTab, setActiveTab] = useState<AuthTab>("signin");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [name, setName] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [showResetPassword, setShowResetPassword] = useState(false);
  const [resetEmail, setResetEmail] = useState("");
  const [resetStatus, setResetStatus] = useState<
    "input" | "sending" | "sent" | "error"
  >("input");
  const [resetError, setResetError] = useState("");

  function resetForm(): void {
    setEmail("");
    setPassword("");
    setConfirmPassword("");
    setName("");
    setInviteCode("");
    setError(null);
  }

  function openResetPassword(): void {
    setResetEmail(email);
    setResetStatus("input");
    setResetError("");
    setShowResetPassword(true);
  }

  function closeResetPassword(): void {
    setShowResetPassword(false);
  }

  async function handleResetSubmit(): Promise<void> {
    if (!resetEmail.trim()) return;
    setResetStatus("sending");
    try {
      await authApi.requestPasswordReset(resetEmail.trim());
      setResetStatus("sent");
    } catch (err) {
      setResetError(
        err instanceof Error ? err.message : "Failed to send reset email"
      );
      setResetStatus("error");
    }
  }

  function handleTabChange(id: string): void {
    setActiveTab(id as AuthTab);
    resetForm();
  }

  async function handleSubmit(e: FormEvent): Promise<void> {
    e.preventDefault();
    setError(null);

    if (!email.trim() || !password.trim()) {
      setError("Email and password are required");
      return;
    }

    if (activeTab === "register") {
      if (password !== confirmPassword) {
        setError("Passwords do not match");
        return;
      }
      if (!name.trim()) {
        setError("Name is required");
        return;
      }
      if (!inviteCode.trim()) {
        setError("Invite code is required");
        return;
      }
    }

    setLoading(true);
    try {
      if (activeTab === "signin") {
        await login(email, password);
      } else {
        await register(email, password, name.trim(), inviteCode.trim());
      }
      navigate({ to: from, replace: true });
    } catch (err) {
      setError(getAuthErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return {
    activeTab,
    email,
    setEmail,
    password,
    setPassword,
    confirmPassword,
    setConfirmPassword,
    name,
    setName,
    inviteCode,
    setInviteCode,
    error,
    loading,
    showResetPassword,
    resetEmail,
    setResetEmail,
    resetStatus,
    resetError,
    handleTabChange,
    handleSubmit,
    openResetPassword,
    closeResetPassword,
    handleResetSubmit,
  };
}
