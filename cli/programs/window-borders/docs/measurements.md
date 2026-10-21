# Memory and behavior measurements

Environment: macOS 15.3.1, Apple Silicon. WindowServer's detailed footprint was
unavailable without elevated privileges. All process footprint numbers below
come from `footprint`; backing-store categories were cross-checked with `vmmap`.

```sh
footprint -f bytes -p "$pid"
vmmap -w "$pid"
```

Raw process logs, snapshots, and corner captures are machine-local under
`measurements/` (gitignored). No persistent validation program is included.

## Segmented stack border

At 2pt width and 2× scale, a two-member focused stack and one other window used
eight surfaces total. Switching `1 → 2 → 1` preserved all eight native window
IDs. Pixel samples from the left and right slots exchanged active/inactive
colors; the center gap remained distinct from both.

The stack's covered member had no separate border surfaces. After returning to
another Space, the normal unsegmented borders were restored.

- Backing stores: 1,966,080 bytes / 1.875 MiB, eight regions.
- Process footprint: 10,355,384 bytes / approximately 9.88 MiB.

Sources: `measurements/segments-run.txt`,
`measurements/segments-{1,2}-of-2.png`, and
`measurements/segments-footprint.txt`.

## Former text-label implementation

The focused two-window stack was observed displaying `1/2` and `2/2` as focus
changed. Returning to an unstacked window removed the label. Closing the
disposable validation windows removed their groups without stopping the manager.

The label used a `28 × 17pt` surface at 2×. With three visible bordered windows
and one label, the process had 13 CG backing-store regions totaling 2,896 KiB;
its sampled footprint was approximately 12 MiB.

After vertical centering, a target with top edge at `y=27pt` and a 2pt border
placed the label at `y=17pt`. The top-border stroke center is `y=26pt`; the
half-point difference from the label center is native whole-point rounding.

Sources: `measurements/stack-label-footprint.txt`,
`measurements/stack-settled-{1,2}-of-2.png`, and
`measurements/stack-centered.png`.

## Manager before stack labels, one-point outer margin

Target: `947 × 1045pt`; width `3pt`; radius `9pt`; scale `2`.
Each row is a fresh process sampled after startup.

| Mode | Surfaces | Backing-store bytes | Footprint bytes |
| --- | ---: | ---: | ---: |
| `empty` | 0 | 0 | 5,046,728 |
| `full` | 1 | 16,187,392 | 22,643,264 |
| `strips` | 4 | 1,015,808 | 6,734,376 |

The backing-store reduction is 93.7247%. Pixel-area estimates logged as
`rgba8_bytes` exclude row alignment and page rounding; they are not allocation
measurements.

The full and strip bottom-left corner captures both contain 1,764 cyan pixels,
with zero differences between their cyan-pixel masks.

Sources: `measurements/current-{empty,full,strips}-{run,footprint,vmmap}.txt`
and `measurements/current-{full,strips}-corner.png`.

### Default all-window run

Seven tracked windows across Spaces, two visible targets, eight allocated
surfaces:

- Backing stores: 2,031,616 bytes / 1.9375 MiB.
- Process footprint: 7,356,896 bytes / 7.0161 MiB.
- Two idle `ps` snapshots six seconds apart both showed 0.02 seconds of total
  process CPU time.

Sources: `measurements/current-manager-{run,footprint}.txt`.

## Interactive manager validation

Two disposable Kitty windows were created after manager startup. They were
automatically discovered; closing them released both groups without stopping
the manager.

- Thirteen observed same-application focus transitions changed colors while
  retaining both groups' window IDs. A subsequent requested focus transition
  was interrupted by another window becoming active, so it was not counted.
- Sixty consecutive programmatic moves retained the same group IDs and reached
  the requested coordinates.
- Minimize released the target's surfaces; restore recreated them.
- Native fullscreen entry released the group; exit restored it.
- Space changes released offscreen groups and recreated newly visible groups.
- A separate owned native window verified sub-level `7` and sticky on/off
  replication after window-state notifications. Direct sub-level/tag writes
  alone did not reliably emit those notifications.

During sixty alternating resizes, with four visible targets and the earlier
eight-point margin, backing stores stayed at **16 regions / 4,128,768 bytes**
after returning to the same final geometry. Footprint changed from 12,173,936
to 12,763,808 bytes; the observed peak was 13,894,304 bytes.

Sources: `measurements/manager-final-run.txt`,
`measurements/manager-resize-{before,after}.txt`. The preceding interactive
manager run lasted approximately 9.5 minutes.

These observations establish short-run lifecycle behavior, not absence of
long-term leaks. Command-to-observation timing includes yabai, process launch,
and inspection overhead; it is not a rendering-latency benchmark. Physical
mixed-scale displays and hot-plugging were not available for validation.

## Initial single-window prototype: eight-point control margin

This historical comparison kept JankyBorders' eight-point outer padding in
both layouts. The current manager needs only a one-point antialiasing margin.

For the same `947 × 1045pt`, width `3`, radius `9`, scale `2` target, three fresh
processes per layout were sampled in rotating order:

| Layout | CG backing stores | Median footprint | Footprint range |
| --- | ---: | ---: | ---: |
| `empty` | 0 MiB | 3.66 MiB | 3.58–3.67 MiB |
| `full` | 15.890625 MiB | 20.88 MiB | 20.52–21.28 MiB |
| `strips` | 1.1875 MiB | 5.95 MiB | 5.89–6.19 MiB |

Backing-store bytes were identical across the three samples of each layout:
16,662,528 for full and 1,245,184 for strips, a 92.53% reduction.

Sources: `measurements/{1,2,3}-{empty,full,strips}-{footprint,vmmap,run}.txt`.

WindowServer RSS deltas varied across samples. They do not establish total
system savings, and shared mappings prevent treating the two processes' RSS as
disjoint allocations.
