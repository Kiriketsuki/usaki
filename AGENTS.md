# AGENTS.md

## Overview

usaKi is a floating desktop widget for Linux (Hyprland) and Windows 11. It shows the usage of
each AI subscription: the 5h and 7d windows, the % used and the reset time. One account is one
flat-top hex cell, and the cells form a rabbit. The account that resets next is the big head cell
with the ears. The others are smaller body cells. Each cell has a 7d ring outside a 5h ring. The cell
shows both percents and both countdowns, 5h on the left and 7d on the right. A click opens the
bubble into a card with the full detail.

The name comes from usagi, Japanese for rabbit. The app carries a rabbit identity throughout.

## Status

The user approved the design. Milestone 1 has the Tauri scaffold and a bubble that shows live Claude
cache data. Read `.claude/handoffs/` for the latest handoff.

## Design sources

| File | Content |
|---|---|
| `docs/design/plan.md` | Approved design, architecture and milestones |
| `docs/design/mockup/*.dc.html` | Mockup artboards (Main, Bubble, Card, States, Settings) |
| https://claude.ai/artifact/GV4AetCYbFgVsAzGFz6KH2 | The same mockup as a live canvas |

The mockup is the visual spec. Port its HTML, SVG and ring maths. Do not redesign them. One
exception: issue #11 replaced the single hex with a honeycomb of cells and redrew the ears. For
the bubble, `ui/src/layout.ts`, `ui/src/geometry.ts` and `ui/src/ears.ts` lead until the mockup
catches up.

## Stack

- Tauri v2, one codebase for Linux and Windows. The app crate is `src-tauri/`.
- Rust backend: providers, poller, config, window position. The shared model and the
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
| Build the app binary | `bun run tauri build --no-bundle` |
| Frontend tests (vitest) | `bun run test` |
| Rust tests | `cargo test --workspace` |
| Format check | `cargo fmt --all --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |

- `bun run test` runs vitest. Do not use `bun test`, which starts the bun test runner.
- A plain `cargo build` gives a dev binary that loads the Vite dev server. Use
  `bun run tauri build` for a binary with the page inside it.
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
- **Codex data:** read `~/.codex/sessions` logs. Map windows by `window_minutes`, not by name.
- **Hexagon only.** No circle variant.
- **Mouse:** a left-drag moves the widget. A left-click expands it. A right-click shows the next
  account in the centre and wraps, and it never goes to auto. A middle-click returns to auto.
- **Theme:** Chrysaki. Zero border-radius, Inter and Fira Code, no emoji. A 220 ms hop on reset
  is the one approved exception to the Chrysaki no-bounce rule.

## Conventions

- Versioning: the `VERSION` file uses `YY.Major.Minor.Patch[Suffix]`. CI bumps it on PR merge (see `.github/CI-CD-Guide.md`).
- Branch: `<type>/<issue#>-<kebab-description>`. Commit and PR title: `<type>[(scope)]: <description>`.
- Write prose and code comments in plain STE form: no em dash, no semicolon, no contraction.
- Commit or push only when the user asks.
