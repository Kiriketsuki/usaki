# AGENTS.md

## Overview

usaKi is a floating desktop widget for Linux (Hyprland) and Windows 11. It shows the usage of
each AI subscription: the 5h and 7d windows, the % used and the reset time. One account is one
ring on a hexagon bubble. A click opens the bubble into a card with the full detail.

The name comes from usagi, Japanese for rabbit. The app carries a rabbit identity throughout.

## Status

The design is approved. No app code exists yet. Read `.claude/handoffs/` for the latest handoff.

## Design sources

| File | Content |
|---|---|
| `docs/design/plan.md` | Approved design, architecture and milestones |
| `docs/design/mockup/*.dc.html` | Mockup artboards (Main, Bubble, Card, States, Settings) |
| https://claude.ai/artifact/GV4AetCYbFgVsAzGFz6KH2 | The same mockup as a live canvas |

The mockup is the visual spec. Port its HTML, SVG and ring maths. Do not redesign them.

## Stack (planned)

- Tauri v2, one codebase for Linux and Windows
- Rust backend: providers, poller, config, window position
- Vite and vanilla TypeScript frontend, with no UI framework
- TOML config at `dirs::config_dir()/usaki/config.toml`

The commands section comes after the scaffold.

## Rules

- **Never refresh an OAuth token, and never write a credentials file.** A Claude refresh token
  works only once. A refresh by usaKi logs out the running Claude Code sessions. If a token has
  expired, show the account as stale and wait.
- **Claude data:** the statusline cache file `~/.cache/usaki/claude-<acct>.json` is the main
  source. The read-only usage API is a fallback, called at most every 5 minutes.
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
