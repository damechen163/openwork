# OpenWork Admin Web

Electron admin dashboard for OpenWork: every `openwork` CLI operation is
available through a graphical interface.

## Features

| Page | What it does |
| --- | --- |
| Dashboard | `openwork status`: installation state, platform facts, runtime health |
| Run Task | `openwork run`: workspace + prompt form, live phase output, artifact list with SHA-256 |
| Install | `openwork install`: dry-run plan preview → explicit consent → execution report |
| Doctor | `openwork doctor`: structured diagnostics with PASS/WARN/FAIL/SKIP |
| Runtimes | `openwork runtime list/info`: detection, version, auth, capabilities |

## Architecture

- **Renderer**: React + TypeScript + Vite, plain-CSS dark admin theme
- **Electron main**: spawns the `openwork` binary (PATH lookup, override with
  `OPENWORK_BIN`), exposes one IPC channel per CLI command, streams `run`
  output line by line to the renderer
- **Security**: `contextIsolation: true`, `nodeIntegration: false`, no
  renderer filesystem access; Chromium sandbox disabled only because the GUI
  runs as root under WSL2

## Requirements

- Node.js 20+ (tested with Node 26)
- The `openwork` CLI binary on `PATH` (or set `OPENWORK_BIN`)
- WSL2 + WSLg (or another X/Wayland display) to show the window

## Development

```bash
npm install
npm run dev        # Vite dev server + Electron (hot reload)
npm run typecheck  # tsc --noEmit
npm run build      # typecheck + vite build (dist/, dist-electron/)
```

## Running the built app

```bash
npm run build
npx electron .     # or: ./node_modules/.bin/electron .
```

Root under WSL2: Chromium's sandbox cannot start as root, so the main process
passes `--no-sandbox` automatically.
