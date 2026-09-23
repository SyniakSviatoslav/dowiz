Status: 2026-09-08 CURRENT — the physical box this repo self-hosts on, and why processes here die with SIGKILL

# The box

| what | value | how measured |
|---|---|---|
| host | Android 16, SDK 36 | `getprop ro.build.version.sdk` → 36 |
| userland | Termux + proot-distro Ubuntu, app uid 10546 | `id` inside proot says uid 0, `logcat` says `untrusted_app_27` / `app=com.termux.window` |
| cores | 7 in the affinity mask: 4,5,6 = A78 big, 0-3 little | `nproc`, `taskset` |
| RAM | 7.5 GB total, ~3.0 GB MemAvailable with only the session running | `/proc/meminfo` |
| swap | 5.6 GB (Android zram), ~1.9 GB already in use at idle | `/proc/meminfo` |
| idle procs | 12-14 | `ps -e --no-headers | wc -l` |

# Signal 9 (SIGKILL) — what actually kills runs here

**It is not the OOM killer.** With 3 GB available and a self-compile that peaks at 22 MB maxrss,
memory is not the binding constraint. The binding constraint is Android's **phantom process
killer**: `activity_manager/max_phantom_processes`, default **32**. Every process this proot
forks is a phantom process from ActivityManager's point of view, and the excess is SIGKILLed.

The arithmetic that used to blow past it:

| job | procs |
|---|---|
| idle box | 12-14 |
| + one `SERIAL=1` chain | ~15 |
| + the battery's *parallel* lanes (the old default) | ~40 |
| + a second heavy slot (old `SLOTS=3`) | ×2, ×3 |

So the old defaults could reach 3× the platform's cap. One heavy job alone sat at the edge.

## What CANNOT be fixed from inside this proot (operator actions)

Verified 2026-09-08 from inside: `/system/bin/device_config get activity_manager
max_phantom_processes` → `cmd: Can't find service: device_config`; `settings get global …` →
`SecurityException: … requires android.permission.INTERACT_ACROSS_USERS`; `/proc/sys/vm/*` →
`Permission denied`; `/proc/self/oom_score_adj` can only be **raised**, never lowered. The proot
`root` is uid 10546 to the platform, so the two real levers are the operator's:

1. **Lift the phantom-process cap.**
   - Android 14+ (this box is 16): *Settings → System → Developer options → **Disable child
     process restrictions*** — no PC needed. This is the same switch as the ADB command.
   - Or, from a PC with ADB:
     `adb shell /system/bin/device_config put activity_manager max_phantom_processes 2147483647`
     (does not survive a reboot on some builds; the developer-options toggle does).
   - Check it took: run a full `tools/chain.sh` with `SERIAL=0` and no slot; it should finish.
2. **Take Termux off battery restrictions.** *Settings → Apps → Termux → Battery → **Unrestricted***.
   Stops the app being frozen/killed when the screen goes off or you switch apps.

Until both are done, the in-box defaults below are what keep runs alive — they are correctness
for this box, not tuning.

## What IS done from inside (in the tree, 2026-09-08)

- `tools/slot.sh`: `SLOTS` defaults to **1** — heavy jobs serialise. Waiting on a slot is free
  (a blocking `flock`), so this costs latency, never a run.
- `tools/slot.sh`: a slot **waits** (up to `PHANTOM_WAIT_S`, then refuses with rc 95) until the
  live `ps -e` count is at or under `PHANTOM_CAP` (26) — 26 + a chain's ~15 stays inside 32.
- `tools/slot.sh`: `MEM_FLOOR_MB` 600 → **1000**, and the in-slot `PROC_CAP` 70 → **32** (70 was
  above the platform cap, i.e. the guard was off in the only direction that mattered).
- `tools/slot.sh`: the slot raises its own tree's `oom_score_adj` to **700** before exec. Raising
  is all an unprivileged process may do, and it is the direction we want: if the kernel OOM killer
  ever does fire, it takes the compile (700), not the session driving it (0).
- `tools/battery.sh`: `SERIAL` now defaults to **1** (~12 procs instead of ~40). This also retires
  the `J=3` false RED the journal recorded on 2026-09-08 — the 12 generated `gb_<op>_<sr>_0_1_0.*`
  files share one `BEBOP_TMP`, so parallel shards clobbered each other.
- **Wake lock held**: `termux-wake-lock` (verified delivered — `am startservice … TermuxService`
  returned `Starting service: Intent { act=com.termux.service_wake_lock … }`). Keeps the CPU from
  sleeping mid-run and gives Termux a foreground notification, which lowers its kill priority.
  Re-acquire it at the start of any long session; `termux-wake-unlock` releases it.

## Gap found 2026-09-08: not every heavy job takes a slot

`tools/slot.sh` gates the runners that call it, but a heavy job started OUTSIDE one is invisible
to the semaphore and still counts against Android's 32. Measured that day: a worker running
`bench/oracles/run_all.sh` directly (it forks `cargo run --release`, and `rustc` on
`dowiz_core` takes minutes) pushed the box to 30-31 procs with slot 1 FREE, so the main
session's own `slot.sh` request sat at the `PHANTOM_CAP` ceiling for its full 300 s and refused
with rc 95. `tools/reap.sh` said `clean` -- correctly, the processes were real work.

So the ceiling did its job (nothing was SIGKILLed) but the slot could not be scheduled. Two
consequences worth knowing:

- `bench/oracles/run_all.sh` and anything else that shells out to `cargo`/`rustc` belongs
  inside `tools/slot.sh` when run standalone; inside `tools/battery.sh` it already is.
- With several agents active, a comparative TIMING run may simply be unschedulable. That is the
  correct outcome -- taking the measurement anyway would produce a number that looks like a
  result and is not one. Wait for the box, or say the row is unmeasured.

## Gap found 2026-09-23: cargo's own parallelism crosses 32 with no help

The 10:27 session death that day had ONE job running, no lanes, slot free: a plain `cargo test`
over `dowiz-core`, `dowiz-hub`, `workers/api`, `native-spa-server`. Cargo defaults `jobs` to
`nproc` (7 here), and rustdoc compiles doctests `RUST_TEST_THREADS` at a time, each as
`rustc -> cc -> ld.lld`. Measured peak live processes on the `dowiz-core` doctests: **21 at 2,
25 at 3** — about 4 per extra job, so the default 7 lands near 41. `/tmp` still held the
`rustdoctest*` dirs from the moment of death. `slot.sh` cannot help: it serialises jobs, not what
happens inside one.

Fixed box-wide in `~/.cargo/config.toml` (outside every repo, so it covers all of them):
`[build] jobs = 2` and `[env] RUST_TEST_THREADS = "2"`. Verified it applies with no env vars set
(same peak 21). Anything that raises cargo parallelism on this box (`-j`, `CARGO_BUILD_JOBS`,
`--test-threads`) re-opens the gap.

## Escape hatches

`SERIAL=0` (parallel battery), `SLOTS=3` (the old three lanes) and `PHANTOM_CAP=<n>` all still
work — use them only on a box where step 1 above has actually been done and verified.
