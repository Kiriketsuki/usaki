# AGENTS.md

## Overview

usaKi is a floating desktop widget for Linux (Hyprland). A Windows 11 build is on the roadmap but
has no test yet. The widget shows the usage of each AI subscription: the 5h and 7d windows, the %
used and the reset time.

One account is one flat-top hex cell, and the cells form a front-view rabbit. The head cell is
the biggest and carries the ears. It shows the account whose 5h window resets next. The other
accounts are smaller body cells. Each cell has a 7d ring outside a 5h ring. The cell shows both
percents, the name with a provider mark and both countdowns, 5h on the left and 7d on the right.
A glass backplate joins the cells, the ears and the foot pads. A click opens the bubble into a
card with the full detail.

The name comes from usagi, Japanese for rabbit. The app carries a rabbit identity throughout.

## Status

The MVP is done. These parts exist:

- The Tauri app and a pinned toplevel bubble with the `usaki-bubble` Hyprland rule
- The Claude statusline cache provider and the Claude watcher logins
- The Codex session log provider, with provider marks on the cells
- The rabbit layout: head and body cells, mosaic ears that fold, foot pads and the backplate
- The card, the mouse and keyboard controls, and the saved position and screen

Not built yet: `config.toml`, the settings window, the tray, start on login, onboarding (#14),
Gemini, a Codex watcher through `codex app-server`, and the Windows build. Read
`.claude/handoffs/` for the latest handoff.

## Design sources

| File | Content |
|---|---|
| `docs/design/plan.md` | Current design, decisions log and milestones |
| `ui/src/layout.ts` | The rabbit layout and the window size for each cell count |
| `ui/src/geometry.ts` | Hex paths, cell sizes, the ring pair and the empty track |
| `ui/src/ears.ts` | The mosaic ears and the fold rule |
| `ui/src/plate.ts` | The backplate |
| `docs/design/claude-cache.md` | The statusline cache file contract |
| `docs/design/watcher-logins.md` | The Claude watcher logins |
| `docs/design/codex-logs.md` | The Codex session log reader |
| `docs/design/mockup/*.dc.html` | The approved mockup of 2026-10-23. It is historical. |

The code leads. The mockup shows the approved design of 2026-10-23. Since then the code has
added tiled cells, ring pairs, the rabbit layout and the provider marks. Change the code and
`plan.md`, not the mockup.

## Stack

- Tauri v2, one codebase for Linux and Windows. The app crate is `src-tauri/`.
- Rust backend: providers, poller, watcher loop, window position. The shared model and the
  providers are in `crates/usaki-core/`.
- Vite and vanilla TypeScript frontend in `ui/`, with no UI framework. bun manages the
  packages and runs the scripts. Never use npm, pnpm or npx.
- TOML config at `dirs::config_dir()/usaki/config.toml` (planned, milestone 3)

`package.json` stays in `ui/`. A root `package.json` turns on the npm publish and version
checks of the template workflows.

## Commands

Run the bun commands in `ui/` and the cargo commands at the repo root.

| Task | Command |
|---|---|
| Install the frontend packages | `bun install` |
| Run the app with hot reload | `bun run tauri dev` |
| Build a debug binary | `bun run tauri build --no-bundle --debug` |
| Build a release binary | `bun run tauri build --no-bundle` |
| Frontend tests (vitest) | `bun run test` |
| Rust tests | `cargo test --workspace` |
| Format check | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |

- `bun run test` runs vitest. Do not use `bun test`, which starts the bun test runner.
- A plain `cargo build` gives a dev binary that loads the Vite dev server. Use
  `bun run tauri build` for a binary with the page inside it.
- On Linux the app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself when the variable is unset
  (`src-tauri/src/window.rs`). It keeps a value that the user sets.
- The window position lives in `dirs::state_dir()/usaki/position.json` (the data folder on
  Windows). `USAKI_STATE_DIR` replaces that folder, so a test run never writes the real file.
- A debug build reads `USAKI_DEBUG_UI`, a comma-separated list of `right`, `middle`, `open` and
  `close`. The page runs one action every 3 s, for live checks without a mouse.
- On Hyprland the bubble needs the `usaki-bubble` window rule from
  `docs/spikes/2026-10-23-hyprland-window.md`. The rule floats and pins the window.

## Rules

- **Never refresh an OAuth token, and never write a credentials file.** A Claude refresh token
  works only once. A refresh by usaKi logs out the running Claude Code sessions. If a token has
  expired, show the account as stale and wait. In a watcher dir only, usaKi may ask the official
  `claude` binary to refresh the token. usaKi itself still never does it.
- **Claude data:** two sources. The main source is the statusline cache file
  `~/.cache/usaki/claude-<acct>.json`. A watcher login at `<data dir>/usaki/watch/<acct>` is the
  second source. For a watcher login, usaKi reads the token read-only and calls the read-only
  usage API, at most every 5 minutes per account. A fresh cache file wins. See
  `docs/design/watcher-logins.md`.
- **Codex data:** read `$CODEX_HOME/sessions` logs (`~/.codex/sessions` by default). Map windows
  by `window_minutes`, not by name. Never read `~/.codex/auth.json`.
- **Hexagon only.** Every shape is a flat-top hex: the cells, the ear mosaic, the foot pads and
  the backplate. No circle variant.
- **Mouse:** a left-drag moves the widget. A left-click expands it into the card. A right-click
  pins the next account into the head and wraps, and it never goes to auto. A middle-click
  returns the head to auto.
- **Theme:** Chrysaki. Zero border-radius, Inter and Fira Code, no emoji. The bubble has no drop
  shadow, because the window edge cuts it into a visible rectangle. A 220 ms hop on reset is the
  one approved exception to the Chrysaki no-bounce rule.
- **Provider colours:** the mark accent comes from `PROVIDER_COLOUR` in `ui/src/marks.ts`. The
  rings keep the usage tones.

  | Provider | Mark | Accent |
  |---|---|---|
  | Claude | Starburst | `#D97757` terracotta |
  | Codex | Chevron | `#20969c` teal |

## Conventions

- Versioning: the `VERSION` file uses `YY.Major.Minor.Patch[Suffix]`. CI bumps it on PR merge (see `.github/CI-CD-Guide.md`).
- Branch: `<type>/<issue#>-<kebab-description>`. Commit and PR title: `<type>[(scope)]: <description>`.
- Write prose and code comments in plain STE form: no em dash, no semicolon, no contraction.
- Commit or push only when the user asks.
