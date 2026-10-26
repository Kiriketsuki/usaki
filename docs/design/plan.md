# usaKi: floating AI usage bubble (Tauri v2, Linux and Windows)

## Context

The user has several AI subscriptions: two Claude accounts (account 1 and account 2), ChatGPT/Codex
(account 3) and Gemini (account 4). Each subscription has a 5h window and a 7d window, with its own % used and reset time.
The user wants one floating widget on Hyprland (Linux) and Windows 11. The widget shows
when each window resets and how much of it the user has used.

This file is the current design. The code leads where the two differ. The bubble shape lives in
`ui/src/layout.ts`, `ui/src/geometry.ts`, `ui/src/ears.ts`, `ui/src/pads.ts` and `ui/src/plate.ts`.
The mockup in `docs/design/mockup/` holds the approved design of 2026-10-23. It is historical. See
the [Decisions log](#decisions-log) for the changes since then.

## Design

- **Framework:** Tauri v2 on both OSes, with one codebase. The user rejected AGS. The mockup HTML and SVG ported directly.
- **Window:** one pinned toplevel window with the app_id `usaki`, the `bubble` window in `src-tauri/tauri.conf.json`.
  Its flags are `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`,
  `resizable: false`, `shadow: false` and `visible: false`. The app shows the window after the first resize.
  On Hyprland the `usaki-bubble` rule floats and pins the window from the map (see **Fullscreen**).
- **Shape:** flat-top hexagons only, laid out as a sitting rabbit seen from the front (issues #11 and #21). Each account is one hex cell.
  The auto account is the head: the biggest cell, 120 px corner to corner, with the ears on it. The other accounts are body cells,
  100 px corner to corner, below the head. When the auto account changes, the new auto account becomes the head.
  Two accounts give a head on a chest. Three give a head between two feet. Four give a head on a chest with two haunches.
  Five add a cell under the chest. Six put the head between two shoulders, with a cell under the head and two haunches below.
  Six accounts then stay shorter than five. More cells extend that body as rows of three. Cells that touch keep a 3 px gap.
  Chrysaki bans rounded corners. The user dropped the circle option.
- **Rings:** each cell has two rings of the same 3 px stroke (scaled with the cell). The 7d ring is outside and the 5h ring
  inside it. Each ring grows clockwise from 12 o'clock, where a thin notch marks the start. The notch also breaks a full ring,
  so a 100% ring never reads as the cell outline.
- **Foot pads:** small decor hexes with no ring and no text (issue #21). A head alone gets two
  front paws under it. A body gets a hind foot pad beside the bottom of each outermost cell. A head between two feet also gets
  two front paws under it. A pad keeps at least 3 px from every cell, so it never covers a ring. A pad hex has a 7.5 px corner
  radius and a 0.5 px gap, in the light surface `#4a4f68`.
- **Backplate:** one plate joins the whole rabbit, so it reads as one silhouette on a busy wallpaper (issue #21, round 2). The plate is
  the union of a copy of every cell, ear hex and pad, grown by 3 px at each edge. The cell glass fills it, and a 1 px line of
  `#e0e2ea` at 20% runs along its outside. Cells, rings, text, ears and pads draw on top of it.
- **Window size:** the bubble window size follows the cell count, from `layout()` in `ui/src/layout.ts`.
  It holds the plate of every fold state and its edge line, with a 2 px pad, so every border pixel stays clear. A fold never changes it.
  The sizes are 139×246 for one account, 155×307 for two and 310×270 for three.
  Four accounts take 310×351, five take 310×396 and six take 310×360.
  The card is 420 px wide. Its height is the bar, the header and one block per account (`cardSize()` in `ui/src/card.ts`).
- **Cell text:** three rows on the cell centre. The 5h window is always on the left and the 7d window on the right.
  The top row shows the two percents. The middle row shows the account name. The bottom row shows the two countdowns
  (`33m`, `1h12m` or `2d04h`). A window with no reset time, such as an unused 5h window, shows 0% and `ready`.
  A name of up to 12 characters fits whole after the provider mark (8 px, 1 px gap). A long name steps the name font
  down from 9.5 px to 9 px, then to 8.5 px, before it loses characters. A name longer than 12 characters ends in an ellipsis.
  The head gets a bright outline when it is the auto account or a pinned account. A pinned head gets its own outline tone and a stud.
  A stale cell dims. When no account is live, the whole bubble dims.
- **Provider marks (issue #19):** each cell shows a small filled polygon before the name. Claude has a starburst in its
  brand terracotta `#D97757` and Codex a teal `#20969c` chevron. `PROVIDER_COLOUR` in `ui/src/marks.ts` holds both accents.
  The card title shows the same mark. The shape names the provider without colour. The rings keep the usage tones.
  The account id is `<provider>:<name>`, so a Claude account and a Codex account with the same name never merge. Pins use this id.
  The body lists the Claude accounts first, then the Codex accounts, each by name.
- **7d-only accounts (issue #19):** Codex on the Pro plan reports only the 7d window. Its cell draws the 5h track as an empty
  track with no arc and no notch, and its 5h slots show `--`. An empty track is a row of small flat-top hex dots in `#363a4f`,
  evenly spaced from 12 o'clock with a dot on each corner (`trackDots` in `ui/src/geometry.ts`). It ranks after every live account that has a 5h window,
  by its 7d reset. So it becomes the auto head only when no live account has a 5h window. A pin can still make it the head.
  It counts for the left (7d) ear only. Its card block has no 5h row, shows the credits balance, and its head tag is `next 7d`.
  An account with no 5h and no 7d window keeps its cell. The cell shows the mark, the name, `no data` and two empty tracks.
- **Colour thresholds:** emerald `#1a8a6a` under 50%, blonde `#FBB13C` from 50%, ruby `#e06a78` from 75%.
  `DEFAULT_THRESHOLDS` in `ui/src/colour.ts` holds the values. The config file in milestone 3 makes them configurable.
- **Card:** a left-click expands the bubble in place into the card. The card has one block per account, in account order.
  Each block shows the mark and the name, a tag on the head (`next 5h`, `next 7d` or `pinned`), and a stale age when stale.
  Each window row has a bar, the percent, the countdown and the clock time of the reset. A 7d row adds a moon.
  A Codex block adds a credits row. The header holds the rabbit mark (close), `updated 2m ago` and a refresh button.
- **Controls:** left-drag on the bubble or the card header moves the window. The drag starts past a 4 px threshold, so a drag
  never counts as a click. A left-click, or Enter or Space on the focused bubble, opens the card. The rabbit mark or Escape closes it.
  A right-click pins the next account into the head (one step, wraps, never auto). A middle-click returns the head to auto (earliest 5h reset).
  Right-click and middle-click do nothing while the card is open.
- **Position:** the app saves the top-left corner and the monitor name in `dirs::state_dir()/usaki/position.json` when a drag
  settles and on exit. It restores them on start. On Hyprland the app reads and moves its window through `hyprctl`, matched by
  process id, because Tauri `set_position` does nothing there. GTK `set_size_request` sets the size. After each resize the
  app moves the window back, because Hyprland keeps the centre and the app keeps the top-left corner.
- **Stick to the home screen:** Hyprland can move a pinned window to another monitor when that monitor switches workspace.
  `src-tauri/src/stick.rs` listens to the Hyprland event socket. After a burst of workspace events it moves the window back
  to its home monitor and corner with `movetoworkspacesilent` and then `movewindowpixel`. That order keeps the pin.
- **Each machine fetches its own data.** Windows has its own logins. The design has no network sync.
- **The widget never refreshes an OAuth token.** A Claude refresh token works only once. A refresh by the widget logs out the running Claude Code sessions.
- **Theme:** Chrysaki tokens, Inter and Fira Code, glass bubble, chamfered card, no border-radius.
  The bubble has no drop shadow. The window holds the plate with a 2 px pad, and the window edge cuts any wider shadow into a faint rectangle.
- **Rabbit identity:** the name comes from usagi, Japanese for rabbit. The user wants a rabbit feel across the whole app (2026-10-23):
  - The head carries two long, angular ears (issue #11). Faceted polygons give the source shape: a flat-cut tip and a light
    amethyst inner panel. Since issue #21 flat-top hexes fill each ear, with a 6 px corner radius and a 0.5 px gap. The ear
    then reads as one solid shape. Each hex takes the tone of the facet that holds its centre. Each facet tone sits one surface step
    above the plate glass. The ears are about 100 px tall and lean out from each other.
  - Both ears stand up normally. The right ear folds when a live 5h window is at 75% or more. The left ear folds when a live 7d window is at 75% or more.
    Both ears fold when a live account hits 100% in either window, or when no live account remains.
    A fold bends the top third of the ear over at a kink, and the hanging flap is the darkest facet. The top row of the
    standing part of a folded ear takes a crease tone below every facet, so the fold reads at 1x.
  - A faceted rabbit-head mark: tri-primary facets and a Blonde eye stud. It is the card header icon (the close button) today.
    It is also planned as the tray icon and the app icon.
  - The rabbit in the moon (tsuki no usagi): each 7d row of the card shows a small moon-phase glyph. The moon fills as the reset gets closer.
  - A 220 ms hop when a window resets. The user approved it, but the app does not draw it yet. It is a deliberate exception to the Chrysaki no-bounce rule.
    Plain rabbit wording ("burrowed", "hop") was not chosen.
- **Fullscreen:** the bubble must stay above fullscreen windows. The user confirmed this on 2026-10-23.
  The spike (`docs/spikes/2026-10-23-hyprland-window.md`) showed that a toplevel stays above fullscreen when Hyprland pins it from the map.
  The user chose the pinned toplevel on 2026-10-26. A layer-shell overlay also stays above fullscreen, but its drag loses the pointer and it cannot cross monitors.
  The app never toggles pin itself. If a window loses its pin during a fullscreen change, Hyprland hides it.

## Data sources

In order of trust:

- **Claude statusline cache:** the primary source, and official. The Claude Code statusline writes
  `${XDG_CACHE_HOME:-$HOME/.cache}/usaki/claude-<acct>.json`. Each cache file is one account. The parser handles both the flat
  `five_hour`/`seven_day` shape and the newer `limits[]` shape. A file older than 15 minutes is stale. See `claude-cache.md`.
- **Claude watcher logins:** a Claude Code login in `<data dir>/usaki/watch/<acct>`, made once with
  `CLAUDE_CONFIG_DIR=<dir> claude auth login`. The app reads the token read-only and calls
  `GET https://api.anthropic.com/api/oauth/usage` (`anthropic-beta: oauth-2025-04-20`) at most every 5 minutes per account.
  Only the official `claude` binary refreshes the token, in the watcher dir. A fresh cache file wins over the watcher. See `watcher-logins.md`.
- **Codex session logs:** the newest `token_count` event in `${CODEX_HOME:-$HOME/.codex}/sessions/**/*.jsonl`. The parser maps
  windows by `window_minutes` (300 = 5h, 10080 = 7d), not by name. The app never reads `auth.json`. See `codex-logs.md`.
  `codex app-server` (`account/rateLimits/read`) is a later option.
- **Gemini:** `retrieveUserQuota` is best effort and not built. Google reportedly cut that access for consumer plans in 2026-06, so "stale" is the expected state.

The poller reads every source each 30 s and sends a `usage://update` event to the webview. The watcher loop runs on its own
thread, because a usage call or a token refresh can take minutes.

## Architecture

The repo is `~/dev/Personal/usaki` (personal, public GitHub `Kiriketsuki/usaki`, made from the `Kiriketsuki/kTemplate` template). It is not part of `~/dots`.
The repo holds no account data. Real config, caches, credentials and session handoffs stay out of git.

```
crates/usaki-core/src/
  model.rs           Snapshot, Window, WindowKind, Credits: the usage model of every provider
  error.rs           error type. Messages follow strict STE form
  providers/
    mod.rs           trait Provider { fn fetch(&self, acct: &Account) -> Result<Snapshot> }
    claude_cache.rs  reads ~/.cache/usaki/claude-<acct>.json
    claude_watch/    watcher logins: read-only credentials, usage call, claude CLI refresh, call schedule
    codex_logs/      scans the newest session jsonl backwards for token_count
src-tauri/src/
  main.rs            app setup, commands (get_usage, set_bubble_size, track_position, debug_ui)
  window.rs          window size, WEBKIT_DISABLE_DMABUF_RENDERER and the app_id on Linux
  accounts.rs        Claude account discovery from the cache files
  codex.rs           Codex account discovery from CODEX_HOME
  poller.rs          30 s loop. Merges the sources and emits `usage://update`
  watch.rs           the watcher loop on its own thread
  view.rs            the usage data that the backend sends to the webview
  position.rs        the saved position file and its validity check
  placement.rs       reads, saves and restores the placement on each platform
  hyprland.rs        hyprctl reads and moves, matched by process id
  stick.rs           keeps the bubble on its home screen on Hyprland
ui/src/ (Vite + vanilla TS, no framework, to keep the webview light)
  main.ts            wiring: events, resize, pointer and key handlers
  model.ts           turns the backend data into the bubble view
  layout.ts          the rabbit layout and the window size for the cell count
  geometry.ts        hex paths, cell sizes, the ring pair, the notch and the empty track
  ears.ts            the ear hexes for each fold state, filled from the faceted ear polygons
  pads.ts            the foot pad hexes at the base of the rabbit
  plate.ts           the backplate: the union of padded copies of every hex
  fit.ts             fits the cell text inside the 5h ring
  marks.ts           the provider marks and accents
  bubble.ts          draws the bubble SVG
  card.ts            the card view. cardRender.ts draws it
  centre.ts          pure head logic: auto, pin, cycle, middle-click reset
  drag.ts            the 4 px drag threshold and the click and drag gesture
  colour.ts          ring tones and thresholds
  countdown.ts       countdown and clock text
  moon.ts            the moon phase of a 7d reset
```

## Changes outside the repo

1. **Statusline:** the user's Claude Code statusline command writes `{account, five_hour, seven_day, written_at}` to
   `${XDG_CACHE_HOME:-$HOME/.cache}/usaki/claude-<acct>.json` through a temp file and `mv`. The write must run before any
   early exit in the script. `claude-cache.md` gives the contract.
2. **Hyprland:** `hyprland.conf` holds the `usaki-bubble` windowrule from the spike report. It matches `class:^usaki$` and sets float, pin,
   no initial focus, no border, no shadow, no rounding, no blur, no dim, no animation and an opacity override.

## Milestones

| # | Scope | Status |
|---|---|---|
| 1 | Scaffold the Tauri app, the model, the Claude cache provider and the statusline write. The bubble renders live Claude data. | Done |
| 2 | Add the card, all mouse controls, position persistence and the Hyprland rule. Also done: ring pairs, the rabbit layout and watcher logins. | Done |
| 3 | Add the Codex provider, the settings window, config hot reload, the tray and start on login. | In progress. Codex is done. Settings, config, tray and start on login are open. |
| 4 | Build and test on Windows. Add Gemini. | Open |

Onboarding for install and first run is issue #14.

## Verification

- Unit tests (Rust): Claude parser against fixtures for the flat shape and the `limits[]` shape. Codex parser against a real
  `token_count` line with `primary` 10080 and `secondary` null. Stale detection. The watcher reads the credentials file with
  `read_to_string` only. A guard test for that rule is still open.
- Unit tests (vitest): the head logic, countdown text, the layout sizes, the ear fold rule, the pads, the plate and the card sizes.
  The first right-click from auto goes to the account after the shown one. The cycle wraps, and a middle-click returns to auto.
- Manual on Hyprland: drag, restart, and confirm the restored position. Confirm that the bubble stays pinned across workspaces and on its
  own screen. Run `claude` in the account 1 directory and in the account 2 directory. Confirm that both cells update.
- RAM: measure PSS with `smem -P usaki` after 10 minutes idle. The target is under 150 MB on Linux.
- Windows: frameless, transparent, always on top, tray, and start on login. Statusline under Git Bash writes `%USERPROFILE%\.cache\usaki`.

## Decisions log

- **2026-10-23:** the user approved the design and the mockup. The name "usaKi" is final. The bubble must stay above fullscreen windows.
- **2026-10-23:** the rabbit identity: ears, the rabbit-head mark, the moon and the hop.
- **2026-10-26:** the pinned toplevel replaced the layer-shell overlay on Linux (issue #2). The layer drag lost the pointer.
- **2026-10-26:** tiled cells replaced the single hex. One account is one hex cell (issue #11).
- **2026-10-26:** each cell has a ring pair, 7d outside and 5h inside, in place of one ring per account (issue #11).
- **2026-10-26:** watcher logins became the second Claude source (issue #13).
- **2026-10-26:** Codex session logs became the Codex source, and provider marks name each provider (issue #19).
- **2026-10-26:** rabbit layout D: a front-view rabbit with a head cell, body cells, mosaic ears, foot pads and a backplate (issue #21).
- **2026-10-26:** the work on issue #19 removed the bubble drop shadow. The window edge cut it into a faint rectangle.
