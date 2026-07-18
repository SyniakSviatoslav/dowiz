# P63 — Verdicts (shell & platform spike)

> Evidence artifact for BLUEPRINT-P63-shell-platform-spike.md.
> Scope: BLUEPRINT-P63 §5 DoD — each spike carries a committed `VerdictRecord`
> with a REAL `measured` value or an honest `Blocked`. The runnable, headless
> proof (the engine⇄platform trait boundary) lives in `engine/src/shell.rs`
> behind `feature = "spike"` and is covered by `cargo test --features spike`.

## Verdict summary (reconciliation against consumers)

| Spike | Ruling | Consumer | Delta the consumer must apply |
|---|---|---|---|
| SP-1 desktop | **Blocked**{physical desktop GPU} | P39-rev §1.2 | boundary proven headless; real winit+AccessKit frame/latency numbers owed on hardware |
| SP-2 mobile surface | **Blocked**{iOS/Android device + toolchain} | P39-rev §1.2 | boundary proven; contention numbers owed on real devices |
| SP-3 payment bridge | **Blocked**{iOS/Android device + provider test keys} | P60 (client leg) | Path B remains directional default; real feasibility owed |
| SP-4 web keyboard | **Blocked**{real iOS-Safari + Chrome-Android} | (X2 interim) | voice + installed Tauri app remains the honest interim |
| SP-5 battery | **Blocked**{physical budget Android device} | P71 (battery gates) | bar + method fixed (BLUEPRINT-P63 §2); number owed; design unblocked |
| SP-6 floor parity | **Blocked**{WebGPU/WebGL2 device} | P69/P70/P71/P73 | **method DELIVERED + GREEN**: durable gate wired into `engine/tests/floor_parity.rs` (4 tests) + `tools/shell-spike/src/floor_parity.rs` (13 tests); CPU reference rung proven bit-deterministic and parity-adversarial cases RED correctly. GPU-rung Δ numbers still owed on real WebGPU/WebGL2 hardware |

**Boundary verdict (the part that does NOT need hardware): CONFIRMS.**
The engine(render)⇄platform(window/events) trait boundary holds: frames travel
engine→platform byte-identical through a `&[u8]`/`FrameSink` contract, the
platform never sees engine internals (`Scene`/`FieldEquilibrium`), the engine
never sees platform events (`ShellEvent`/`PlatformShell`), and the FE-14
settle gate is honored across the boundary. Proven by 8 `cargo test` gates.

## Evidence rows (machine-generated schema, `VerdictRecord`)

| spike | bar | method | measured | verdict | platform | hw_class | captured_utc |
|---|---|---|---|---|---|---|---|
| Sp1Desktop | p95 ≤ 16.7ms; p99 ≤ 33ms; input ≤ 50ms; AccessKit focus+role+value+caret | FrameProfiler distribution + input↔present counter + screen-reader checklist | BLOCKED: no physical desktop GPU in CI | Blocked (physical desktop GPU) | Desktop(Linux) | Emulator | 1 |
| Sp2MobileSurface | 0 flicker frames; p95 ≤ 33ms; ≥10 bg/fg cycles | 30-min soak, native surface vs WebGPU-in-webview | BLOCKED: no iOS/Android device or NDK/provisioning | Blocked (iOS+Android device + toolchain) | MobileAndroid(emulator) | Emulator | 2 |
| Sp3PaymentBridge | ≤3 dropped frames; ≤250ms recovery; zero corruption | ≥20 present→dismiss cycles, test mode | BLOCKED: no device + provider test keys | Blocked (iOS/Android device + provider test keys) | MobileIos(emulator) | Emulator | 3 |
| Sp4WebKeyboard | keyboard+input+no-visible-DOM+a11y-intact on BOTH iOS-Safari & Chrome-Android | candidate matrix (VirtualKeyboard API, hidden-focus, forwarding host) | BLOCKED: no real mobile browser reachable | Blocked (real iOS-Safari + Chrome-Android) | WebMobile(Safari-iOS) | Emulator | 4 |
| Sp5Battery | settled ≤ 4%/h; settle saves ≥30% vs off; sustained ≤ 33ms no throttle | scripted 6h shift, batterystats, settle-ON vs settle-OFF A/B | BLOCKED: no physical budget Android device | Blocked (physical budget Android device) | MobileAndroid(emulator) | Emulator | 5 |
| Sp6FloorParity | every rung ΔE ≤ 0.02 vs CPU reference | perceptual Δ over compose() reference, WebGPU+WebGL2+CPU | DELIVERED (method gate, CPU rung): 4 `cargo test --test floor_parity` GREEN in engine + 13 GREEN in tools/shell-spike[features=floor_parity]; oracle bit-determinism + adversarial WebGPU-only/blank catches RED correctly. **BLOCKED**: real WebGPU/WebGL2 rung Δ numbers owed (no GPU device in CI) | Blocked (WebGPU/WebGL2 device) | HighDesktop | Emulator | 6 |

> Honesty gates enforced in code:
> - `VerdictRecord::is_measured_pass()` is `true` ONLY for `Confirms`; `Blocked`/
>   `Refines`/`Contradicts` are never a pass.
> - `BatteryVerdictRecord::try_new` REJECTS `HwClass::Emulator` at construction
>   (SP-5 battery cannot be emulated — BLUEPRINT-P63 §3.5).
> - The `floor_parity` feature is OFF in the spike crate's default build; the
>   durable gate lives in `engine/tests/floor_parity.rs` (always run by
>   `cargo test --test floor_parity`) — REGRESSION-LEDGER entry
>   `p63_sp6_floor_parity_gate` (permanent).
> - The `spike` feature is OFF in the default build (REGRESSION-LEDGER entry
>   `p63_spike_feature_isolated_from_default_build`).
