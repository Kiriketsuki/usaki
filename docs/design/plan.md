# usaKi: floating AI usage bubble (Tauri v2, Linux and Windows)

## Context

The user has several AI subscriptions: two Claude accounts (account 1 and account 2), ChatGPT/Codex
(account 3) and Gemini (account 4). Each subscription has a 5h window and a 7d window, with its own % used and reset time.
The user wants one floating widget on Hyprland (Linux) and Windows 11. The widget shows
when each window resets and how much of it the user has used. The user can configure the widget.

The approved mockup is at https://claude.ai/artifact/GV4AetCYbFgVsAzGFz6KH2. Its artboards are
Main (prototype), Bubble, Card, States and Settings. The mockup is the visual spec. The
`.dc.html` sources are in `docs/design/mockup/`.

## Settled decisions

- **Framework:** Tauri v2 on both OSes, with one codebase. The user rejected AGS. The mockup HTML and SVG port directly.
- **Shape:** a flat-top hexagon only, with one concentric hex ring per account, outermost first. Chrysaki bans rounded corners. The user dropped the circle option.
- **Mouse:** left-drag moves the widget. A left-click expands the bubble in place into the card, and a click on the card's header hex collapses it.
  A right-click shows the next account in the centre (one step, wraps, never auto). A middle-click returns the centre to auto (earliest 5h reset).
- **Centre:** three lines. The mode (`next 5h` / `5h reset` / `stale 3h`), the countdown, and `<short> · <pct>%`. When the user pins an account, the other rings dim.
- **Colour thresholds:** emerald `#1a8a6a` under 50%, blonde `#FBB13C` from 50%, ruby `#e06a78` from 75%. All three thresholds can be configured.
- **Each machine fetches its own data.** Windows has its own logins. The design has no network sync.
- **The widget never refreshes an OAuth token.** A Claude refresh token works only once. A refresh by the widget logs out the running Claude Code sessions.
- **Data sources, in order of trust:**
  - Claude: the statusline cache file is the primary source and is official. The fallback is a read-only call to `GET https://api.anthropic.com/api/oauth/usage`
    (`anthropic-beta: oauth-2025-04-20`). The widget makes it at most every 5 minutes, and only when the cache is older than 15 minutes. The parser handles both the flat
    `five_hour`/`seven_day` shape and the newer `limits[]` shape. An expired token shows "stale" and the widget waits.
  - Codex: the newest `token_count` event in `~/.codex/sessions/**/*.jsonl`. The parser maps windows by `window_minutes` (300 = 5h, 10080 = 7d), not by name.
    `codex app-server` (`account/rateLimits/read`) is a later option.
  - Gemini: `retrieveUserQuota` is best effort. Google reportedly cut that access for consumer plans in 2026-06, so "stale" is the expected state.
- **Theme:** Chrysaki tokens, Inter and Fira Code, glass bubble, chamfered card, no border-radius.
- **Rabbit identity:** the name comes from usagi, Japanese for rabbit. The user wants a rabbit feel across the whole app (2026-10-23):
  - The bubble has two angular, faceted ears on top of the hex. Both ears stand up normally. One ear folds when any ring is critical. Both ears fold when an account hits its limit or the data is stale.
  - A faceted rabbit-head mark: tri-primary facets and a Blonde eye stud. It is the tray icon, the card header icon (in place of the small hex) and the app icon.
  - The rabbit in the moon (tsuki no usagi): each 7d reset shows a small moon-phase glyph that fills as the reset gets closer.
  - A 220 ms hop when a window resets. This is a deliberate exception to the Chrysaki no-bounce rule. Plain rabbit wording ("burrowed", "hop") was not chosen.
- **Fullscreen:** the bubble must stay above fullscreen windows. The user confirmed this on 2026-10-23.
  A Tauri window on Hyprland is a normal toplevel, so a fullscreen app probably covers it. Only layer-shell stays above.
  The spike tests this first. If the toplevel fails, the Linux build uses layer-shell.
  The mockup's toggle is now off and says "(untested on Linux)".

## Architecture

New repo `~/dev/Personal/usaki` (personal, public GitHub `Kiriketsuki/usaki`, made from the `Kiriketsuki/kTemplate` template). It is not part of `~/dots`.
The repo holds no account data. Real config, caches, credentials and session handoffs stay out of git.

```
src-tauri/src/
  main.rs            app setup, windows (bubble, settings), tray, autostart plugin
  model.rs           Snapshot { account_id, windows: [Window{kind, used_pct, resets_at}], fetched_at, source, stale }
  config.rs          TOML load/save + notify watcher for hot reload; path from dirs::config_dir()/usaki/config.toml
  poller.rs          tokio loop, per-provider minimum interval, emits `usage://update` to the webview
  providers/
    mod.rs           trait Provider { fn fetch(&self, acct: &Account) -> Result<Snapshot> }
    claude_cache.rs  reads ~/.cache/usaki/claude-<acct>.json
    claude_api.rs    read-only usage call, never writes the credentials file
    codex_logs.rs    scans the newest session jsonl backwards for token_count
    gemini.rs        best effort
  position.rs        saves and restores x and y. On Hyprland it detects HYPRLAND_INSTANCE_SIGNATURE and runs
                     `hyprctl dispatch movewindowpixel exact X Y,title:^usaki$` after the map. Elsewhere it uses set_position.
ui/ (Vite + vanilla TS, no framework, to keep the webview light)
  bubble.ts          hex ring SVG: port hexPath() and the ring maths from Bubble.dc.html
  card.ts            expanded card: port of Card.dc.html
  centre.ts          pure centre-selection logic: auto, cycle, middle-click reset
  interactions.ts    pointer threshold of 4 px → getCurrentWindow().startDragging(); contextmenu → cycle; auxclick button 1 → auto
  settings/          second window: port of Settings.dc.html, writes config.toml
```

Window flags: `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`,
`resizable: false`. The window resizes between 152×188 (bubble: the hex plus the ears) and 420×470 (card), anchored at the top-left.

## Changes outside the new repo

1. `~/dev/obKidian/000-System/Agents/Claude/statusline/statusline-command.sh` (the `~/.claude/statusline` symlink target):
   after the relay block, and **before** the `CHRYSAKI_STATUSLINE_MOD` early `exit 0` at about line 24, write
   `{account, five_hour, seven_day, written_at}` to `${XDG_CACHE_HOME:-$HOME/.cache}/usaki/claude-<acct>.json` through a temp file and `mv`.
   The account comes from the same rule as lines 140 to 148: `CLAUDE_CONFIG_DIR`, else `.workspace.current_dir` matching the account 2 work directory.
   This block must stay above the early exit, because the mod path exits there.
2. `~/dots/hypr/.config/hypr/hyprland.conf`: add windowrules for `title:^usaki$`: float, pin, no border, no shadow, no rounding, no focus on map.

## Milestones

1. Scaffold the Tauri app, `model.rs`, `claude_cache.rs` and the statusline write. The bubble renders live Claude data for both accounts.
2. Add the card, all mouse interactions, position persistence and the Hyprland rules.
3. Add the Codex provider, the settings window, config hot reload, the tray and start on login.
4. Build and test on Windows. Add the Claude API fallback and Gemini.

## Verification

- Unit tests (Rust): Claude parser against fixtures for the flat shape and the `limits[]` shape. Codex parser against a real
  `token_count` line with `primary` 10080 and `secondary` null. Stale detection. A guard test that `claude_api.rs` never opens the credentials file for writing.
- Unit tests (vitest): `centre.ts`: the first right-click from auto goes to the account after the shown one, it wraps, and a middle-click returns to auto. Countdown formatting.
- Manual on Hyprland: drag, restart, and confirm the restored position. Confirm that the bubble stays pinned across workspaces. Run `claude` in the account 1 directory and in the account 2 directory, then confirm that both rings update.
- RAM: measure PSS with `smem -P usaki` after 10 min idle. The target is under 150 MB on Linux.
- Windows: frameless, transparent, always on top, tray, and start on login. Statusline under Git Bash writes `%USERPROFILE%\.cache\usaki`.

## Resolved questions (2026-10-23)

- "usaKi" is the final name.
- The bubble must stay above fullscreen windows. See **Fullscreen** above.
