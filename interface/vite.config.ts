import path from "node:path";
import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const envDir = path.resolve(__dirname, "..");
  const env = loadEnv(mode, envDir, "");
  const serverPort = env.ZERO_SERVER_PORT || "3100";
  const apiTarget = `http://localhost:${serverPort}`;

  const vendoredZuiEntry = path.resolve(__dirname, "node_modules/@cypher-asi/zui/src/index.ts");
  const vendoredZuiStyles = path.resolve(__dirname, "node_modules/@cypher-asi/zui/src/styles/index.css");

  return {
    plugins: [react()],
    resolve: {
      dedupe: ["react", "react-dom"],
      preserveSymlinks: true,
      alias: [
        { find: "@cypher-asi/zui/styles", replacement: vendoredZuiStyles },
        { find: "@cypher-asi/zui", replacement: vendoredZuiEntry },
        { find: "react-dom", replacement: path.resolve(__dirname, "node_modules/react-dom") },
        { find: "react", replacement: path.resolve(__dirname, "node_modules/react") },
      ],
    },
    server: {
      port: 5173,
      proxy: {
        "/api": { target: apiTarget },
      },
    },
  };
});
