//! BLUEPRINT-P63 — Shell & platform spike: the thin engine⇄platform boundary.
//!
//! This crate is the **runnable, falsifiable core** of P63. P63's full contract
//! (`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P63-shell-platform-spike.md`)
//! is six *measurement* spikes (SP-1..SP-6). Five of them (SP-1..SP-5) need
//! physical hardware — a real desktop GPU, a budget Android, an iOS device, a
//! native-payment SDK, a soft-keyboard on a real mobile browser. Those cannot
//! run headless, so they are encoded here as the **verdict schema + the honesty
//! gate** (§2 / §4.1 / §4.3) and committed as honestly-`Blocked` rows in
//! `docs/design/.../P63-VERDICTS.md`.
//!
//! What this crate *does* prove — the one thing that does not need hardware —
//! is **the boundary holds**:
//!   * frames travel engine→platform unmodified through a `&[u8]`/`FrameSink`
//!     contract (the generalization of `bridge::GpuUploadSink` to a settled
//!     RGBA frame, not a vertex slice — reuse-first, BLUEPRINT-P63 §1.1);
//!   * the platform never sees engine internals (`Scene`/`FieldEquilibrium`);
//!   * the engine never sees platform events (`ShellEvent`/`PlatformShell`);
//!   * events are translated to render intents ONLY at the `RenderLoop`;
//!   * SP-6's floor-parity METHOD is real and catches both adversarial cases
//!     (a deliberately-blank rung and a deliberately-WebGPU-only effect).
//!
//! Dependency direction (the boundary contract):
//!   * Platform depends on `FrameSink` (`&[u8]`) — never on `Scene`.
//!   * Engine depends on `RenderEngine` (`Vec<u8>` + a settle gate) — never on
//!     `ShellEvent`.
//!   * `RenderLoop` is the ONLY coupling point.
//!
//! Isolation (BLUEPRINT-P63 §4.3 bulkhead): this is a standalone crate with a
//! **one-directional** dependency (it depends on `dowiz-engine`; `dowiz-engine`
//! does NOT and cannot depend on it). The canonical order/money graph never
//! compiles this code. At close, `rm -rf tools/shell-spike/` leaves the product
//! tree byte-unchanged (§4.5).

use dowiz_engine::field_frame::{self, FieldEquilibrium};
use dowiz_engine::scene::{Scene, SdfShape};
use dowiz_engine::FrameProfiler;

pub mod floor_parity;
pub mod verdict;

pub use verdict::{
    BatteryVerdictRecord, DesktopOs, HwClass, Platform, SpikeId, SpikeVerdict, VerdictError,
    VerdictRecord,
};

// ── Falsifiable bars (BLUEPRINT-P63 §2, named BEFORE any prototype) ──────────
pub const DESKTOP_FRAME_BUDGET_MS: f64 = 16.7; // 60 FPS interactive target
pub const DESKTOP_FRAME_FLOOR_MS: f64 = 33.0; // 30 FPS hard floor (P38 §6)
pub const INPUT_LATENCY_BAR_MS_P95: f64 = 50.0; // tap/keystroke→pixel, p95
pub const MOBILE_SOAK_MIN: u32 = 30; // continuous render soak
pub const MOBILE_FLICKER_FRAMES_MAX: u32 = 0; // webview↔wgpu contention flicker
pub const MOBILE_FRAME_FLOOR_MS: f64 = 33.0;
pub const SHEET_PRESENT_DROPPED_MAX: u32 = 3; // frames lost presenting SDK sheet
pub const SHEET_RECOVERY_MS_MAX: f64 = 250.0; // render loop back within budget
pub const SHIFT_HOURS_SIM: f64 = 6.0; // simulated shift length
pub const SETTLED_DRAIN_BAR_PCT_PER_HR: f64 = 4.0; // settled screen, foreground
pub const SETTLE_SAVINGS_MIN_PCT: f64 = 30.0; // settle-ON beats settle-OFF by ≥30%
pub const THERMAL_SUSTAIN_MS: f64 = 33.0; // sustained frame time over soak
pub const PARITY_PERCEPTUAL_DELTA_MAX: f64 = 0.02; // normalized per-pixel ΔE / (1−SSIM)

/// Steps the spike's reference frame evolves. Small — this is a boundary proof,
/// not a visual-quality render.
const SPIKE_STEPS: usize = 4;

// ── The thin boundary traits ─────────────────────────────────────────────────

/// The contract the ENGINE writes through. The platform's present path
/// implements this; the engine only ever sees `&[u8]`. This is the
/// generalization of `bridge::GpuUploadSink` to a settled RGBA frame rather
/// than a vertex slice (reuse-first, BLUEPRINT-P63 §1.1: extend, don't fork).
pub trait FrameSink {
    /// Receive one fully-rendered RGBA8 frame from the engine. The platform owns
    /// how it gets to the swapchain; the engine does not know.
    fn accept_frame(&mut self, frame: &[u8]);
}

/// The render side as seen by the shell. A `RenderEngine` produces frames and
/// exposes its FE-14 settle gate (`should_render`) — but it knows NOTHING about
/// windows, events, or platforms.
pub trait RenderEngine {
    /// Produce one RGBA8 frame at `w×h` after `steps` evolution steps.
    fn render_frame(&mut self, w: usize, h: usize, steps: usize) -> Vec<u8>;
    /// FE-14 lazy-render-on-settle (BLUEPRINT-P63 §0 cites P38 §3.5). When the
    /// field is settled the engine asks NOT to be rendered — the platform must
    /// honor this without knowing *why*.
    fn should_render(&self) -> bool;
}

/// Platform-side input event. Defined on the platform side of the boundary; the
/// engine type system never names it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellEvent {
    Close,
    Resize(u32, u32),
    Pointer(f32, f32),
    Key(u32),
}

/// A platform shell: owns the window + input, and presents frames via `FrameSink`.
/// `RenderLoop` is the only place that talks to both `PlatformShell` and
/// `RenderEngine`.
pub trait PlatformShell: FrameSink {
    /// Return the next pending input event, or `None` if the queue is empty.
    fn poll_event(&mut self) -> Option<ShellEvent>;
    /// Current swapchain size in physical pixels.
    fn size(&self) -> (u32, u32);
    /// Platform-side response to a resize intent: update the swapchain/viewport.
    fn apply_resize(&mut self, w: u32, h: u32);
    /// Platform-side response to a close intent: mark the shell closing.
    fn request_close(&mut self);
}

// ── Engine-side implementation (wraps the real `compose()`) ──────────────────

/// A `RenderEngine` backed by the engine's bit-deterministic
/// `field_frame::compose`. Holds the scene + equilibrium; never references any
/// platform type.
pub struct SceneRenderer {
    scene: Scene,
    eq: FieldEquilibrium,
    /// FE-14 settle state. When `true`, `should_render()` reports the field is
    /// settled and the shell should skip rendering (the battery lever, SP-5).
    settled: bool,
    /// Count of times `render_frame` was actually invoked (boundary-behavior probe).
    pub render_calls: usize,
    /// Last (w,h) the loop asked us to render at (proves resize reached the engine).
    pub last_size: (usize, usize),
}

impl SceneRenderer {
    pub fn new(scene: Scene, eq: FieldEquilibrium) -> Self {
        SceneRenderer {
            scene,
            eq,
            settled: false,
            render_calls: 0,
            last_size: (0, 0),
        }
    }

    /// Set the FE-14 settle state. The render side decides this from its own
    /// energy/settled predicate; the platform never computes it.
    pub fn set_settled(&mut self, settled: bool) {
        self.settled = settled;
    }
}

impl RenderEngine for SceneRenderer {
    fn render_frame(&mut self, w: usize, h: usize, steps: usize) -> Vec<u8> {
        self.render_calls += 1;
        self.last_size = (w, h);
        field_frame::compose(&self.scene, &self.eq, w, h, steps)
    }

    fn should_render(&self) -> bool {
        !self.settled
    }
}

// ── Platform-side implementation (headless mock — proves the boundary headless)

/// A headless `PlatformShell` used by the spike's proof. It records every frame
/// the engine delivers (the swapchain mirror) and replays a scripted event
/// queue. It never imports `Scene`/`FieldEquilibrium` — it only sees `&[u8]`.
#[derive(Default)]
pub struct HeadlessShell {
    events: Vec<ShellEvent>,
    cursor: usize,
    width: u32,
    height: u32,
    /// The swapchain mirror: every `&[u8]` the engine delivered, in order.
    pub presented: Vec<Vec<u8>>,
    /// Number of `accept_frame` calls (= frames actually presented).
    pub present_calls: usize,
    /// The last resize the loop translated into a render request.
    pub last_resize: Option<(u32, u32)>,
    /// Whether a `Close` event was observed and honored.
    pub closed: bool,
}

impl HeadlessShell {
    pub fn new(width: u32, height: u32) -> Self {
        HeadlessShell {
            events: Vec::new(),
            cursor: 0,
            width,
            height,
            presented: Vec::new(),
            present_calls: 0,
            last_resize: None,
            closed: false,
        }
    }

    /// Queue scripted input events for the loop to consume.
    pub fn with_events(mut self, events: Vec<ShellEvent>) -> Self {
        self.events = events;
        self
    }
}

impl FrameSink for HeadlessShell {
    fn accept_frame(&mut self, frame: &[u8]) {
        self.presented.push(frame.to_vec());
        self.present_calls += 1;
    }
}

impl PlatformShell for HeadlessShell {
    fn poll_event(&mut self) -> Option<ShellEvent> {
        if self.cursor < self.events.len() {
            let ev = self.events[self.cursor];
            self.cursor += 1;
            Some(ev)
        } else {
            None
        }
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn apply_resize(&mut self, w: u32, h: u32) {
        self.last_resize = Some((w, h));
        self.width = w;
        self.height = h;
    }

    fn request_close(&mut self) {
        self.closed = true;
    }
}

// ── The glue: the ONLY coupling point between engine and platform ────────────

/// Drives the shell+engine pair. This is the sole place that knows both trait
/// families: it translates `ShellEvent` → render intents (platform→engine) and
/// feeds engine output into the platform's `FrameSink` (engine→platform). The
/// engine and platform never reference each other's types.
pub struct RenderLoop {
    profiler: FrameProfiler,
}

impl Default for RenderLoop {
    fn default() -> Self {
        RenderLoop {
            profiler: FrameProfiler::default(),
        }
    }
}

impl RenderLoop {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run up to `max_frames` render iterations, or until the shell closes.
    /// Returns the number of frames actually presented.
    pub fn run<P, E>(&mut self, shell: &mut P, engine: &mut E, max_frames: usize) -> usize
    where
        P: PlatformShell,
        E: RenderEngine,
    {
        let mut presented = 0usize;
        let mut frames = 0usize;
        while frames < max_frames {
            // Platform → engine: translate input events to render intents.
            // The engine never sees `ShellEvent`; only size/settle intents.
            while let Some(ev) = shell.poll_event() {
                match ev {
                    ShellEvent::Close => {
                        shell.request_close();
                        return presented;
                    }
                    ShellEvent::Resize(w, h) => {
                        // The platform reports a new size; the engine learns it
                        // only through the next `render_frame(w,h,..)` call.
                        shell.apply_resize(w, h);
                    }
                    ShellEvent::Pointer(..) | ShellEvent::Key(..) => {
                        // Intent routing: a pointer/key event is an engine input
                        // intent. In this spike it is a no-op beyond proving the
                        // boundary translated it (no JSON, no DOM).
                        self.profiler.json_parse_calls += 0; // explicitly 0 — no JSON in the loop
                    }
                }
            }

            // Engine → platform: honor the settle gate, then render+present.
            if engine.should_render() {
                let (w, h) = shell.size();
                let frame = engine.render_frame(w as usize, h as usize, SPIKE_STEPS);
                shell.accept_frame(&frame); // through the FrameSink boundary
                self.profiler.write_buffer_calls += 1; // one present per frame
                presented += 1;
                frames += 1;
            } else {
                // Settled: the boundary SKIPS the engine entirely. No render, no
                // present. The platform does not need to know why.
                frames += 1;
            }
        }
        presented
    }

    /// The GREEN gate: zero JSON (no (de)serialize in the loop) and exactly one
    /// present per rendered frame.
    pub fn profiler(&self) -> &FrameProfiler {
        &self.profiler
    }
}

/// The bit-deterministic reference frame (BLUEPRINT-P63 §3.6 oracle): the CPU
/// `compose()` output the platform-side boundary must reproduce unmodified. Used
/// by the boundary tests to prove the `FrameSink` pipe is transparent.
pub fn field_frame_compose(scene: &Scene, w: usize, h: usize) -> Vec<u8> {
    field_frame::compose(scene, &FieldEquilibrium::default(), w, h, SPIKE_STEPS)
}

/// Build a small scene for the boundary proof (a couple of SDF instances, like
/// SP-1's "~1–2k SDF instances" reduced to a minimal, deterministic corpus).
pub fn tiny_scene() -> Scene {
    let mut s = Scene::new().with_scale(0.5);
    s.add(SdfShape::Circle {
        cx: 0.0,
        cy: 0.0,
        r: 8.0,
    })
    .add(SdfShape::Box {
        bx: 4.0,
        by: 2.0,
        hx: 1.0,
        hy: 1.0,
    });
    s
}

#[cfg(test)]
mod tests;
