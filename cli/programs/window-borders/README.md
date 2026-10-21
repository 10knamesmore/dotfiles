# window-borders

Event-driven macOS window borders written in Rust. Each drawn border uses four
narrow windows instead of one transparent window covering its entire area.

## Installation and startup

The dotfiles `dots install` manifest installs this binary to
`~/.local/bin/window-borders` on macOS only. The managed yabai configuration
starts it with the dotfiles palette, 2pt borders, automatic display scaling,
and segmented stack indicators.

```sh
cd ~/dotfiles
./dots.sh install
```

This macOS-only program is an independent Cargo package under `cli/programs/`.
It keeps its own lockfile and build profile. The `cli/dots/` workspace does not
build it; `dots install` selects it only on macOS.

## Manual run

```sh
cd ~/dotfiles/cli/programs/window-borders
cargo build --release
./target/release/window-borders
```

The default process runs until Ctrl-C or SIGTERM. It manages existing and newly
created document windows and floating modal windows. Menus, attached sheets,
desktop elements, and border windows are excluded.

Defaults match the current dotfiles palette:

```sh
./target/release/window-borders \
  --width 3 \
  --active-color 0xffcba6f7 \
  --inactive-color 0xff45475a
```

The executable does not stop an existing border process. Stop the yabai-launched
instance before a manual run.

## Behavior

- **Focus:** the foreground application's front eligible window receives the
  active color. Color changes repaint existing surfaces without replacing them.
- **Movement:** the four windows move and reorder in one SkyLight transaction,
  below the target at its level and sub-level. Borders ignore mouse input.
- **Resize:** new fixed-size surfaces replace the previous group. The old group
  is ordered out in the same transaction, then released.
- **Visibility:** hidden, minimized, off-Space, and fullscreen windows retain
  their tracked identity but have no allocated border surfaces.
- **Spaces:** entering a visible desktop recreates the group. Sticky state is
  mirrored onto border windows. Native fullscreen Spaces have no borders.
- **Displays:** the display containing the largest visible portion of a target
  determines its backing scale. Display configuration changes refresh the
  display list; a scale change replaces the affected surfaces.
- **Lifetime:** target close releases its group. Closing one application does
  not stop the all-window manager.
- **Stack indicator:** with `--yabai-stack`, the focused stack's top border has
  one segment per member, highlighting the current member. Other windows have
  an uninterrupted border.

The run loop sleeps between WindowServer/display notifications. There is no
geometry polling or animation integration. Direct private-API tag or sub-level
writes that emit no notification are observed at the next window/Space update.

## Options

| Option | Meaning | Default |
| --- | --- | --- |
| `--window ID` | Restrict management to one target; exit when it closes | All eligible windows |
| `--width POINTS` | Border width | `3` |
| `--radius POINTS` | Target corner radius | `9` |
| `--active-color ARGB` | Focused color, eight hex digits | `0xffcba6f7` |
| `--inactive-color ARGB` | Unfocused color, eight hex digits | `0xff45475a` |
| `--scale 1\|2` | Override automatic display scale | Automatic |
| `--seconds N` | Limit the run duration | No limit |
| `--verbose` | Log focus, display, lifecycle, and surface changes | Off |
| `--layout strips\|full\|empty` | Surface-allocation comparison | `strips` |
| `--yabai-stack` | Divide the focused stack's top border into member segments | Off; enabled by yabairc |

`full` uses one full-size surface. `empty` tracks windows without creating
surfaces. Both drawing layouts use the same ring path and one-point
antialiasing margin; there is no unused glow/shadow padding.

For a bounded single-window comparison:

```sh
window_id="$(yabai -m query --windows --window | jq -r .id)"
./target/release/window-borders \
  --window "$window_id" --layout strips --seconds 60 --verbose
```

`jq` is only used by this example to select the window. The running manager
requires `yabai` when stack indicators are enabled; ordinary borders do not require
it. Neither mode uses an Accessibility observer or screen capture.

## Stack indicators

`yabairc` registers named signals for focus, window creation/destruction,
movement, resize, Space changes, and display changes. Each sends `SIGUSR1` to
`window-borders` after yabai has updated its state.

With `--yabai-stack`, an asynchronous worker reads:

```sh
yabai -m query --windows id,frame,stack-index,has-focus --space
```

Indexed windows sharing the focused window's frame form the stack snapshot;
yabai's query output has no stack-group ID. Members are sorted by `stack-index`.
The WindowServer focus determines which member is highlighted.

The worker coalesces refresh requests and publishes the latest result without
blocking drawing. There is no polling. A failed yabai command is reported as an
error rather than silently keeping a stale indicator.

The straight top edge is divided into equal-width slots. The current slot uses
the active border color; the others use the inactive color. Gaps are 6pt, capped
at one third of a slot for narrow windows. Corners and the other three edges
keep the border color.

Segments reuse the existing top surface at the configured border width. They
add no text, native windows, backing-store height, or application-layout changes.
Covered members of the focused stack release their own borders so they cannot
show through the gaps. Focus changes within the known stack transfer the same
four surfaces to the new target; only the top surface needs repainting when
geometry and border color remain unchanged. Leaving the stack restores ordinary
borders for its members.

## Structure

```text
WindowServer + display notifications
                    │
                    ▼
              WindowManager
              ├── tracked targets / focus
              ├── visible desktop + display scale
              └── optional border group per target
                              │
                              ▼
                  BorderGeometry → CoreGraphics
                              │
                              ▼
                  SkyLight placement transaction
```

| File | Responsibility |
| --- | --- |
| `src/main.rs` | CLI and process lifetime |
| `src/manager.rs` | Window inventory, focus, visibility, group lifetime |
| `src/geometry.rs` | Disjoint surface rectangles and rounded-ring coordinates |
| `src/color.rs` | ARGB parsing and RGBA conversion |
| `src/paint.rs` | Border color and optional top-segment appearance |
| `src/stack.rs` | Asynchronous, coalesced yabai stack queries |
| `src/macos/events.rs` | Coalesced notifications and main-run-loop delivery |
| `src/macos/windows.rs` | Window discovery, eligibility, focus, snapshots |
| `src/macos/displays.rs` | Display bounds, backing scales, visible Spaces |
| `src/macos/mod.rs` | Native surface ownership, drawing, placement |
| `src/macos/ffi.rs` | Private macOS entry points |

Window state and drawing are main-thread-owned. Callbacks only collect changed
IDs. A `Surface` owns one native window; CoreGraphics contexts exist only while
drawing. Offscreen targets retain metadata, not pixel buffers.

## Measured results

Before stack-label integration, on macOS 15.3.1, Apple Silicon, for one
`947 × 1045pt` target at 2×:

| Current layout | CG backing stores | Process footprint |
| --- | ---: | ---: |
| `empty` | 0 MiB | 4.81 MiB |
| `full` | 15.4375 MiB | 21.59 MiB |
| `strips` | 0.96875 MiB | 6.42 MiB |

Backing-store allocation is **93.72% lower** in the strip layout.

The default all-window run tracked seven windows across Spaces, with two
visible targets / eight surfaces: **1.9375 MiB backing stores**, **7.02 MiB
process footprint**. These are local samples, not a memory limit.

See [measurement and validation details](docs/measurements.md), including the
earlier controlled prototype comparison and remaining validation limits.

## Platform and validation

Private SkyLight calls have been exercised on **macOS 15.3.1, Apple Silicon**,
with SIP enabled. No OS-version compatibility paths are implemented.

Window creation/close, same-application focus changes, movement, resize,
minimize/restore, Space changes, native fullscreen round trips, sticky state,
and nonzero sub-level replication were exercised with disposable windows.

Automatic scale selection was exercised on the available 2× display. Moving
between physical 1×/2× displays and display hot-plugging still need hardware
validation. Short interactive runs do not establish multi-day memory stability
or frame-perfect tracking during continuous mouse dragging.

```sh
cargo fmt --check
cargo clippy --release -- -D warnings
cargo build --release
```

Reference implementations:
[JankyBorders](https://github.com/FelixKratz/JankyBorders) and
[yabai](https://github.com/asmvik/yabai).
