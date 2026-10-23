# Spike: Tauri v2 window on Hyprland 0.54.3

Issue: #2. Date: 2026-10-23. Code: `spikes/hypr-window/`.

## Summary

- A Tauri v2 toplevel stays above a fullscreen window only when Hyprland pins it from
  the moment it maps. Hyprland hides a window that loses its pin during a fullscreen change.
  It recovers when fullscreen ends, if it stays unpinned. A window pinned again while
  hidden stays invisible after fullscreen ends and after it gets focus.
- A gtk-layer-shell overlay surface stays above fullscreen in both fullscreen modes. It
  needs no hyprland.conf rule, and it gets the exact size and position the app asks for.
- **Recommendation:** use a layer-shell overlay surface for the bubble and the card on
  Hyprland. Keep the toplevel path as the fallback for compositors without layer-shell.
- On this machine WebKitGTK draws nothing unless `WEBKIT_DISABLE_DMABUF_RENDERER=1` is set.
- On Hyprland, the Tauri `set_position`, `set_size` and `outer_position` calls do not work.
  The app must use hyprctl (toplevel) or layer margins (layer-shell) for position, and
  GTK `set_size_request` for size.

## Environment

| Item | Value |
|---|---|
| Compositor | Hyprland 0.54.3 (commit 521ece46) |
| Monitors | eDP-1 at 0,0, DP-2 at 1920,0 (1920x1080, test monitor), DP-3 at 3840,0. Scale 1.00 on all. |
| GPU | NVIDIA (PCI id 10de:28e0). The log shows `MESA-EGL: warning: egl: failed to create dri2 screen`. |
| Toolchain | rustc 1.91, cargo 1.91.1, bun 1.3.10, @tauri-apps/cli 2.12.4 |
| Crates | tauri 2.12.5, tao 0.37.1, wry 0.57.0, gtk 0.18.2, gtk-layer-shell 0.8.2 (feature `v0_6`) |
| System libs | webkit2gtk-4.1, gtk-layer-shell 0.10.1 |

## How to run the spike

```sh
cd spikes/hypr-window
bun install
CARGO_TARGET_DIR=$PWD/target bunx tauri build --no-bundle --debug
WEBKIT_DISABLE_DMABUF_RENDERER=1 ./target/debug/usaki-spike-hypr-window                         # toplevel
WEBKIT_DISABLE_DMABUF_RENDERER=1 USAKI_SPIKE_MODE=layer ./target/debug/usaki-spike-hypr-window  # layer-shell
```

Set `USAKI_SPIKE_CMD_FILE=<path>` to drive the window while it runs. The backend runs each
new line of that file once: `geom`, `pos X Y`, `size W H`, `gtk-size W H`, `resizable 0|1`,
`autotest`, `layer-move DX DY`, `gtk-reshow`, `quit`. The header of
`src-tauri/src/main.rs` lists all the variables.

## Checks

The screenshot files are outside the repo, in a session scratch directory outside the repo.
The table names them by file name only. They can show private desktop content, so the
repo holds no copy.

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | Native Wayland | Pass | `hyprctl clients -j` for title `usaki`: `"xwayland": false`, `"class": "usaki-spike-hypr-window"`. |
| 2 | Float, pin, no border, no shadow, no rounding, no focus on map | Pass, with two extra rules | See [Check 2](#check-2-float-and-pin). Hyprland also needs `opacity ... override` and `no_dim`. The `size` rule did not give the requested size. |
| 3 | Transparency | Pass, with `WEBKIT_DISABLE_DMABUF_RENDERER=1` | See [Check 3](#check-3-transparency). |
| 4 | Position restore | `movewindowpixel` passes. Tauri `set_position` fails. | See [Check 4](#check-4-position). |
| 5 | `startDragging` IPC | Pass (IPC only). A real drag needs a manual check. | See [Check 5](#check-5-startdragging). |
| 6 | Above fullscreen, toplevel | Pass only when pinned from map | See [Check 6](#check-6-fullscreen-toplevel). |
| 7 | Layer-shell variant | Pass, with a first-map workaround | See [Check 7](#check-7-layer-shell). |

### Check 2: float and pin

The spike did not edit hyprland.conf and did not run `hyprctl reload`. The user config has
`exec = python3 ~/.config/hypr/scripts/setup_monitors.py`, and a reload runs it again.
The spike used two runtime paths instead.

**Path A, dispatchers on a mapped window.** All calls returned `ok`:

```sh
hyprctl dispatch setfloating address:0x55bf757810e0
hyprctl dispatch pin address:0x55bf757810e0
hyprctl dispatch setprop address:0x55bf757810e0 border_size 0   # ok
hyprctl dispatch setprop address:0x55bf757810e0 no_shadow 1     # ok
hyprctl dispatch setprop address:0x55bf757810e0 rounding 0      # ok
hyprctl dispatch setprop address:0x55bf757810e0 no_blur 1       # ok
```

Result: `{"floating":true,"pinned":true,"xwayland":false}`. `pin` is a toggle.

**Path B, a named rule at runtime.** Hyprland 0.54 lets hyprctl change and disable named
rules (`windowrule[name]:enable false`). The spike made the rule `usaki-spike` with
`hyprctl keyword 'windowrule[usaki-spike]:<effect>' '<value>'` for `match:title ^usaki$`,
`float on`, `pin on`, `border_size 0`, `no_shadow on`, `rounding 0`, `no_initial_focus on`,
`no_blur on`, `no_dim on` and `opacity 1.0 override 1.0 override 1.0 override`. Each call
returned `ok`. A new window then mapped as:

```
{"at":[2780,440],"size":[200,200],"floating":true,"pinned":true,"xwayland":false,"overFullscreen":true}
active window before and after the map: the user terminal (focus did not change)
getprop border_size: 0   no_shadow: true   rounding: 0   no_blur: true
```

At the end the spike ran `hyprctl keyword 'windowrule[usaki-spike]:enable' false` (`ok`).

Findings:

- The user config sets `active_opacity = 0.9`, `inactive_opacity = 0.85` and
  `dim_inactive = true`. Without an opacity override, the hex fill sampled as
  `srgb(25,121,95)` and terminal text showed through it. With
  `opacity_override 1` and `opacity_inactive_override 1`, the fill sampled as
  `srgb(26,138,106)`, which is exactly `#1a8a6a`.
- The `size` effect did not apply. With `size 152 188` and later `size 300 260`, the window
  mapped at 200x200 both times. The app must set its own size (see [Sizing](#sizing-on-hyprland)).
- The rules were checked for syntax with `Hyprland --verify-config -c <temp file>`:
  `config ok`. A control file with a false effect name gave
  `config option <windowrule:no_borderz> does not exist.`

### Check 3: transparency

| Run | Env | Result | Screenshot |
|---|---|---|---|
| 1 | none | The window mapped (`hyprctl` lists it), but nothing drew. The region shows only the desktop. | `01-toplevel-initial.png` |
| 2 | `WEBKIT_DISABLE_DMABUF_RENDERER=1` | The hex drew. Hyprland blurred the area behind the window and drew a border, because no rule was set yet. | `02-toplevel-nodmabuf.png` |
| 3 | same, plus the runtime rule and opacity override | The hex drew at full colour. Terminal text around the hex, inside the window rectangle, is crisp, with no blur and no dim. | `05-toplevel-opacity-override.png` |
| 4 | layer-shell, same env | The hex drew. A pixel inside the surface but outside the hex sampled as the colour of the window below (`srgb(128,32,64)`). | `23-layer-over-fs0.png` |

Command: `grim -g "2760,420 240x240" <file>`, then `magick <file> -format "%[pixel:p{x,y}]" info:`.

### Check 4: position

```
hyprctl dispatch movewindowpixel exact 3500 700,title:^usaki$   -> ok, "at":[3500,700]
hyprctl dispatch movewindowpixel exact 2000 100,title:^usaki$   -> ok, "at":[2000,100]
Tauri set_position(3300,600) -> Ok(())                           -> "at":[2000,100] (no move)
Tauri set_position(1380,500) -> Ok(())                           -> "at":[2000,100] (no move)
Tauri outer_position()       -> PhysicalPosition { x: 0, y: 0 } at every position
```

Wayland does not let a client set or read its global position. So on the toplevel path the
app must read `at` from `hyprctl clients -j` to save the position, and must restore it
with `movewindowpixel`. The layer-shell path does not need hyprctl, because the app owns
the margins.

### Check 5: startDragging

The backend ran JS in the page that calls `getCurrentWindow().startDragging()`, then a
control call that the capability does not grant:

```
js: autotest startDragging resolved: no permission error
js: autotest setPosition rejected (control call, expected): window.set_position not allowed. Permissions associated with this command: core:window:allow-set-position
```

So `core:window:allow-start-dragging` reaches the backend, and a missing permission shows
as a clear rejection. The window did not move, because no mouse button was down. The tools to
simulate a pointer (wtype, ydotool) are not installed. A real drag needs the manual check
in [Manual checks](#manual-checks).

### Check 6: fullscreen, toplevel

Setup: a test kitty (`kitty --class usaki-spike-fs`, background `#802040`) on workspace 11,
usaki at 2000,100 on the same workspace. Region: `grim -g "1980,80 240x240"`. The pixel at
the hex centre is `srgb(26,138,106)` when usaki is visible, and the kitty colour when kitty
covers it.

| Case | kitty state | usaki | Result | Screenshot |
|---|---|---|---|---|
| a | `fullscreen 0` (fullscreen: 2) | pinned from map | Visible | `06-fs0-pinned.png`, `15-pinned-from-map-fs0.png` |
| b | `fullscreen 1` (fullscreen: 1) | pinned from map | Visible | `07-fs1-pinned.png`, `14-pinned-from-map-fs1.png` |
| c | already fullscreen, then usaki starts | pinned from map | Visible. Focus stays on kitty. | `13-launched-over-fs0.png` |
| d | `fullscreen 1` set after an unpin | same workspace, not pinned | Hidden | `08-fs1-unpinned.png` |
| e | `fullscreen 0` set after an unpin | same workspace, not pinned | Hidden | `09-fs0-unpinned.png` |
| f | kitty stays fullscreen, usaki pinned again | pinned again | Still hidden (`overFullscreen: false`) | `10-fs0-repinned.png` |
| g | as f, then `alterzorder top` | pinned | Still hidden (`overFullscreen: true`, fade alpha stays 0) | `12-fs0-after-alterzorder.png` |

The Hyprland v0.54.3 source explains the result:

- `src/Compositor.cpp:2226-2230`. On a fullscreen change, Hyprland sets
  `m_createdOverFullscreen = false` for each window on the workspace, but it skips pinned
  windows.
- `src/managers/animation/DesktopAnimationManager.cpp:466-475`. The fade loop skips pinned
  windows. It sets alpha 0 for other windows that were not created over the fullscreen
  window.
- `src/render/Renderer.cpp:366`. The over-fullscreen pass draws a floating window when it
  is `m_createdOverFullscreen` or `m_pinned`.
- `src/desktop/view/Window.cpp:1974`. A floating window gets `m_createdOverFullscreen = true` at map.

**Decision:** the toplevel is visible above fullscreen only when it floats and Hyprland
pins it from the map onward. That holds with the `pin = on` rule. Suppose a user unpins it, for
example with a pin keybind, and a fullscreen change then happens. Hyprland then hides
the window. Two cases follow:

- If it stays unpinned, it shows again when fullscreen ends.
- If the user pins it again while Hyprland hides it, it stays invisible after fullscreen ends,
  after `alterzorder top`, and after a focus change. Live test: the hex centre stayed
  `srgb(117,31,60)` (the kitty colour) 5 s after `fullscreen 0`, and after
  `focuswindow usaki`. The cause is in
  `src/managers/animation/DesktopAnimationManager.cpp:466-470`. The loop skips a window when
  `if (w->m_fadingOut || w->m_pinned || w->isFullscreen()) continue;` matches. Only after
  that check does it reset `*w->m_alpha = 1.F`. On fullscreen exit it skips the pinned window, so alpha stays at zero.
  The code in `FocusState.cpp:39-40` runs only while the workspace has a fullscreen window, and it
  sets the floating fade, not `m_alpha`. Focus therefore does not repair it.

The app must never toggle pin at runtime on the toplevel path. A user who toggles pin by
hand can lose the window until it is closed and mapped again.

### Check 7: layer-shell

The spike creates the window with `visible: false`, then calls
`init_layer_shell()`, `set_namespace("usaki")`, `set_layer(Layer::Overlay)`, anchors top
and left, sets the margins, `set_keyboard_mode(KeyboardMode::None)` and
`set_exclusive_zone(-1)`, and then shows the window through Tauri. GTK reported
`realized=false visible=false` before the init, so the spike did not need an unrealize.

```
hyprctl layers -j  -> DP-2, level 3 (overlay): {"x":2000,"y":100,"w":152,"h":188,"namespace":"usaki"}
```

| Item | Result | Evidence |
|---|---|---|
| First map | Blank. Hyprland lists the surface, but no pixel draws. | `16-layer-over-fs0.png`, `17-layer-no-fs.png`. `WAYLAND_DEBUG=1` shows `zwlr_layer_surface_v1#50.configure(558912, 152, 188)` and `ack_configure(558912)`, then no `attach` or `commit` on that surface. |
| Ruled out | `WEBKIT_DISABLE_COMPOSITING_MODE=1`, a bare gtk `show_all()`, `LD_PRELOAD` of libgtk-layer-shell, and an opaque GTK background. All stayed blank. | `18-` to `21-*.png`. `ldd` already lists libgtk-layer-shell before libwayland-client. |
| Workaround | One `hide()` and `show_all()` on the GTK window after the map. The surface then draws. The spike now does this 300 ms after setup. | `22-layer-reshow.png`. With the automatic workaround, 5 of 6 launches drew. One early launch did not appear in `hyprctl layers`, and the next 5 did (`trial-1.png` to `trial-4.png`, `26-layer-auto-reshow-desktop.png`). |
| Above fullscreen 0 | Visible | `23-layer-over-fs0.png`: hex `srgb(26,138,106)`, outside the hex `srgb(128,32,64)` |
| Above fullscreen 1 | Visible | `24-layer-over-fs1.png` |
| Transparency | Pass. No blur, no dim, no opacity rule needed. | as above |
| Move | `layer-move 300 150` -> `{"x":2300,"y":250}`. `layer-move -300 -150` -> `{"x":2000,"y":100}`. | command log and `hyprctl layers -j` |
| Resize | Tauri `set_size(420,470)` had no effect. GTK `set_size_request` plus `resize` gave `{"w":420,"h":470}`, then `{"w":152,"h":188}`, anchored at the top-left. | `25-layer-after-resize.png` |

A layer surface cannot use `startDragging`. The spike page implements the drag in
`src/index.html`. On `pointerdown` it stores the client point and captures the pointer. On
each `pointermove` it sends the delta from that point to the `layer_move_by` command, which
adds it to the margins. The point stays fixed, because the surface moves under the
pointer. The spike tested the command path. The pointer path needs the manual check below.

## Sizing on Hyprland

| Case | Result |
|---|---|
| No float rule | The window mapped floating at 955x1070, about the size of a tiled slot next to the open terminal. Tao calls `set_resizable(false)` after the first configure (`tao-0.37.1/src/platform_impl/linux/window.rs:117-123`). The likely cause is that this call locks the size of the first configure. This cause is not confirmed. `resizewindowpixel exact 152 188` and the `max_size` prop then had no effect. |
| Float rule | The window mapped at 200x200, whatever the `size` rule said. |
| Tauri `set_size` | `Ok(())`, no change, on both the toplevel and the layer surface. |
| GTK `set_size_request` plus `resize` | Works. On the toplevel, Hyprland keeps the window centre: 200x200 at 2780,440 became 152x188 at 2804,446. The app must move the window after a resize to keep the top-left fixed. On the layer surface, the top-left stays fixed. |

## Recommended Linux window approach

**Layer-shell overlay for the bubble and the card.** Reasons:

1. It meets the hard requirement. The overlay layer is above fullscreen windows in both
   modes, and no window state (pin, focus, workspace) can hide it.
2. It needs no hyprland.conf rule and no `~/dots` change. The window rules in the plan
   (section "Changes outside the new repo", item 2) are then not needed for the bubble.
3. The app owns position and size. Margins give an exact top-left position that the app
   can save and restore without hyprctl. GTK resizes keep the top-left anchor that the
   plan asks for.
4. Sway and river also implement `zwlr_layer_shell_v1`. The spike did not test them.

Costs to plan for:

- The first-map blank surface needs the hide and show workaround, and a check that the
  surface really drew. It failed once in 6 launches.
- The drag is custom code (margin deltas), not `startDragging`. The spike page sends
  `client - start` on every `pointermove` without waiting for the margin change. Several
  events can arrive before the compositor moves the surface, so one delta can apply more
  than once and the drag can overshoot or jitter. The real code needs a guard: one move
  in flight at a time, or subtract the sum of the deltas already sent. The manual drag
  check stays a blocker for the decision.
- The app must pick the monitor with `set_monitor`. Without it, the compositor uses the
  focused monitor. Margins are relative to that monitor.
- `KeyboardMode::None` blocks key input. The card has no text input, so this is fine. The
  settings window should stay a normal toplevel.
- `position.rs` in the plan must change: no hyprctl on Linux, save the margins and the
  monitor name instead.

**Fallback:** on a Wayland compositor without `zwlr_layer_shell_v1` (for example GNOME),
use the toplevel with the rules below. Check for layer-shell support with
`gtk_layer_shell::is_supported()` at start.

## hyprland.conf rules

Layer-shell path (recommended). No rule is required. This one turns off the open animation:

```
layerrule {
  name = usaki-layer
  match:namespace = ^usaki$
  no_anim = on
}
```

Toplevel path (fallback). The rule matches the Wayland app_id (class), not the title. The
spike app uses the class `usaki-spike-hypr-window`. The real app must set its app_id to
`usaki`. The settings window must use another app_id so it does not match. A title match
would also catch any other window with the title `usaki`. `Hyprland --verify-config`
accepts this block:

```
windowrule {
  name = usaki-bubble
  match:class = ^usaki$
  float = on
  pin = on
  no_initial_focus = on
  border_size = 0
  no_shadow = on
  rounding = 0
  no_blur = on
  no_dim = on
  no_anim = on
  opacity = 1.0 override 1.0 override 1.0 override
}
```

Do not rely on `size` in this rule. The app sets its size through GTK. The `pin = on`
effect is what keeps the window above fullscreen, so it must stay in the rule.

## Environment variables

| Variable | Needed | Why |
|---|---|---|
| `WEBKIT_DISABLE_DMABUF_RENDERER=1` | Yes, on this NVIDIA machine | Without it the webview draws nothing (check 3, run 1). The app should set it itself on Linux, before the Tauri builder runs, when the user has not set it. |
| `WEBKIT_DISABLE_COMPOSITING_MODE=1` | No | It did not fix the blank layer surface. |
| `LD_PRELOAD` of libgtk-layer-shell | No | The link order is already correct. |

## Open risks for Windows 11

The spike tested none of these. The spike ran on Linux only.

- **Fullscreen:** `alwaysOnTop` puts the window in the topmost band. A borderless
  fullscreen app usually stays below it. An exclusive fullscreen game (DirectX flip model)
  covers it, and another topmost app can go above it. Test with a borderless game and an
  exclusive one.
- **Corners and border:** Windows 11 DWM can round the corners of an undecorated window
  and draw a 1 px border. Chrysaki bans rounded corners. `shadow: false` may not be
  enough. The app may need
  `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND)`.
- **Transparency:** WebView2 needs a transparent background colour for the page. Check that
  no white flash shows before the first paint.
- **Click-through:** on both OSes, the transparent part of the window still takes clicks.
  On Linux this also needs a fix (a GDK input region). On Windows, hit testing or
  `set_ignore_cursor_events` on the transparent area.
- **DPI:** monitors with mixed scale factors change physical and logical coordinates when
  the window crosses them. Save the position in one unit only.
- **Focus:** `show()` can take focus from the active app. Test that the bubble does not
  take focus on start.

## Manual checks

1. Toplevel drag: run the toplevel command from [How to run](#how-to-run-the-spike).
   Left-drag the hex. Confirm that the window follows the pointer and stays where you
   release it.
2. Layer drag: run the layer-shell command, left-drag the hex, and confirm that it follows
   the pointer and `hyprctl layers -j` shows the new `x` and `y`.

## Session clean-up

- All spike windows, the test kitty and every process the spike started are closed.
  `ps` shows no `usaki` or test `kitty` process.
- The runtime rule `usaki-spike` is disabled (`enable false`). It stays in Hyprland
  memory as an inert rule until the next config reload. No other runtime keyword changed.
- The spike did not edit `~/dots` or any hyprland.conf.
