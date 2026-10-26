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
- **Shape:** flat-top hexagons only. Each account is one hex cell, 100 px corner to corner, and the cells tile edge to edge into one cluster (issue #11).
  The fill order gives 2 cells on a diagonal, 3 as a triangle, 4 as a diamond and 5 as a kite. More cells fill rings around the middle.
  Each cell has two 3 px rings: the 7d ring outside and the 5h ring inside it. Chrysaki bans rounded corners. The user dropped the circle option.
- **Mouse:** left-drag moves the widget. A left-click expands the bubble in place into the card, and a click on the card's header hex collapses it.
  A right-click shows the next account in the centre (one step, wraps, never auto). A middle-click returns the centre to auto (earliest 5h reset).
- **Cell text:** three rows on the cell centre. The 5h window is always on the left and the 7d window on the right.
  The top row shows the two percents. The middle row shows the account name. The bottom row shows the two countdowns
  (`33m`, `1h12m` or `2d04h`). A window with no reset time, such as an unused 5h window, shows 0% and `ready`.
  A name longer than 12 characters ends in an ellipsis.
  The auto account (earliest 5h reset) or the pinned account gets a bright outline. A stale cell dims.
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
  - The bubble has two long, angular, faceted ears on the top cell. They tilt out a little, with a flat-cut tip and a light amethyst inner panel.
    Both ears stand up normally. The right ear folds when a live 5h window is at 75% or more. The left ear folds when a live 7d window is at 75% or more.
    Both ears fold when a live account hits 100% in either window, or when no live account remains.
    A fold bends the top third of the ear over at a kink, and the hanging flap is the darkest facet.
  - A faceted rabbit-head mark: tri-primary facets and a Blonde eye stud. It is the tray icon, the card header icon (in place of the small hex) and the app icon.
  - The rabbit in the moon (tsuki no usagi): each 7d reset shows a small moon-phase glyph that fills as the reset gets closer.
  - A 220 ms hop when a window resets. This is a deliberate exception to the Chrysaki no-bounce rule. Plain rabbit wording ("burrowed", "hop") was not chosen.
- **Fullscreen:** the bubble must stay above fullscreen windows. The user confirmed this on 2026-10-23.
  The spike (`docs/spikes/2026-10-23-hyprland-window.md`) showed that a toplevel stays above fullscreen when Hyprland pins it from the map.
  The user chose the pinned toplevel on 2026-10-26. A layer-shell overlay also stays above fullscreen, but its drag loses the pointer and it cannot cross monitors.
  The app never toggles pin itself. If a window loses its pin during a fullscreen change, Hyprland hides it.
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
                     `hyprctl dispatch movewindowpixel exact X Y,class:^usaki$` after the map, because Tauri
                     set_position and set_size do nothing there. Size goes through GTK set_size_request, and
                     a move after each resize keeps the top-left fixed. Elsewhere it uses set_position.
ui/ (Vite + vanilla TS, no framework, to keep the webview light)
  bubble.ts          hex cell SVG: port hexPath() and the ring maths from Bubble.dc.html, one cell per account
  layout.ts          honeycomb cell layout and the window size for the cell count
  card.ts            expanded card: port of Card.dc.html
  centre.ts          pure centre-selection logic: auto, cycle, middle-click reset
  interactions.ts    pointer threshold of 4 px → getCurrentWindow().startDragging(); contextmenu → cycle; auxclick button 1 → auto
  settings/          second window: port of Settings.dc.html, writes config.toml
```

Window flags: `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`,
`resizable: false`. The bubble window size follows the cell count, from `layout()` in `ui/src/layout.ts`:
104×149 for one cell, 179×192 for two, 179×236 for three, 254×236 for four and 254×322 for five.
The card is 420×470. Every resize keeps the top-left corner in place. On Hyprland the app moves the window back by PID after a resize, because Hyprland keeps the centre.

## Changes outside the new repo

1. `~/dev/obKidian/000-System/Agents/Claude/statusline/statusline-command.sh` (the `~/.claude/statusline` symlink target):
   after the relay block, and **before** the `CHRYSAKI_STATUSLINE_MOD` early `exit 0` at about line 24, write
   `{account, five_hour, seven_day, written_at}` to `${XDG_CACHE_HOME:-$HOME/.cache}/usaki/claude-<acct>.json` through a temp file and `mv`.
   The account comes from the same rule as lines 140 to 148: `CLAUDE_CONFIG_DIR`, else `.workspace.current_dir` matching the account 2 work directory.
   This block must stay above the early exit, because the mod path exits there.
2. `~/dots/hypr/.config/hypr/hyprland.conf`: add the `usaki-bubble` windowrule from the spike report. It matches `class:^usaki$` and sets float, pin, no border, no shadow, no rounding, no focus on map, no dim and an opacity override.

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
