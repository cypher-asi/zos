import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { ThemeProvider } from "@cypher-asi/zui";
import { queryClient } from "./lib/query-client";
import { router } from "./router";
import { useAuthStore } from "./stores/auth-store";
import "@fontsource-variable/inter";
import "@cypher-asi/zui/styles";
import "./index.css";

useAuthStore.getState().restoreSession();

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("Missing #root element");

createRoot(rootEl).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme="dark" defaultAccent="purple">
        <RouterProvider router={router} />
      </ThemeProvider>
    </QueryClientProvider>
  </StrictMode>,
);
