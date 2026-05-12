# ZERO

Desktop application: **Rust (tao + wry)** desktop shell, **Axum** API server, **React 19 + TypeScript + ZUI** frontend, **Zustand** data layer.

## Quick Start

### Development (two terminals)

```bash
# Terminal 1 – API server
cargo run -p zero-server

# Terminal 2 – Vite dev server (hot reload)
cd interface && npm install && npm run dev
```

Open **http://localhost:5173** in a browser. The Vite dev server proxies `/api` requests to the Axum server on port 3100.

### Desktop

```bash
cargo run -p zero-desktop
```

Builds the interface, starts the Axum server in-process, and opens a frameless native window (tao + wry WebView).

## Architecture

```
zos/
  apps/
    zero-desktop/     Rust binary – tao window + wry webview + embedded server
    zero-server/      Rust lib+binary – Axum API server
  crates/
    zos-auth/         Auth client crate
  interface/          Vite + React 19 + TypeScript + ZUI + Zustand
    src/
      shell/          Shell infrastructure (types, store, platform detection)
      api/            Typed fetch client + endpoint wrappers
      stores/         Zustand domain stores (projects, UI state)
      layout/         Layout primitives (Shell, Lane, NavRail, WindowControls)
      apps/           Self-contained app modules (one file per app)
                      Includes the headlining `zero.tsx` app.
```

## Key Technologies

- **Window**: tao (cross-platform windowing) + wry (WebView2 / WKWebView)
- **Server**: Axum + Tokio + tower-http
- **Interface**: Vite 8, React 19, TypeScript 5.9
- **UI Library**: @cypher-asi/zui (design system)
- **State**: Zustand 5
- **Icons**: lucide-react

## Environment

- `ZERO_SERVER_PORT` (default `3100`)
- `ZERO_SERVER_HOST` (default `127.0.0.1`)
