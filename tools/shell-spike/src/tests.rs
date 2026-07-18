//! BLUEPRINT-P63 — boundary-proof tests (the runnable, headless core of the spike).
//!
//! These tests assert the thin engine⇄platform boundary actually holds:
//!   * frames travel engine→platform BYTE-IDENTICAL through `FrameSink` (`&[u8]`);
//!   * the platform never sees engine internals (`Scene` is never named on the
//!     platform side — `HeadlessShell` only ever holds `&[u8]`);
//!   * the engine never sees platform events (`SceneRenderer` never names
//!     `ShellEvent`);
//!   * the GREEN loop performs zero JSON and exactly one present per frame;
//!   * the FE-14 settle gate skips the engine entirely across the boundary;
//!   * resize / close intents are translated at the `RenderLoop` and reach the
//!     right side without leaking types.

use crate::{
    tiny_scene, DesktopOs, FieldEquilibrium, HwClass, Platform, RenderEngine, RenderLoop,
    SceneRenderer, ShellEvent, SpikeId, SpikeVerdict, HeadlessShell, VerdictRecord,
};

#[test]
fn boundary_delivers_engine_frame_unmodified_to_platform() {
    let scene = tiny_scene();
    let eq = FieldEquilibrium::default();
    let (w, h) = (32u32, 32u32);

    let mut shell = HeadlessShell::new(w, h);
    let mut engine = SceneRenderer::new(scene.clone(), eq);
    let mut loop_ = RenderLoop::new();

    let presented = loop_.run(&mut shell, &mut engine, 3);

    // Every rendered frame was presented to the platform.
    assert_eq!(presented, 3, "loop presented exactly max_frames frames");
    assert_eq!(shell.present_calls, 3);
    assert_eq!(engine.render_calls, 3, "engine rendered once per frame");

    // The platform received EXACTLY the engine's compose() output, unmodified —
    // the boundary is a transparent &[u8] pipe, not a transform.
    let expected = crate::field_frame_compose(&scene, w as usize, h as usize);
    for (i, frame) in shell.presented.iter().enumerate() {
        assert_eq!(
            frame.len(),
            expected.len(),
            "frame {i} byte length matches compose()"
        );
        assert!(
            frame == &expected,
            "frame {i} is byte-identical to the engine's compose() output (boundary unmodified)"
        );
    }

    // GREEN gate: zero JSON, one present per frame.
    assert_eq!(
        loop_.profiler().json_parse_calls,
        0,
        "GREEN loop: ZERO JSON in the frame path"
    );
    assert_eq!(
        loop_.profiler().write_buffer_calls,
        3,
        "one present per rendered frame"
    );
}

#[test]
fn settle_gate_skips_engine_render_across_boundary() {
    let mut shell = HeadlessShell::new(32, 32);
    let mut engine = SceneRenderer::new(tiny_scene(), FieldEquilibrium::default());
    engine.set_settled(true); // field is settled → don't render
    let mut loop_ = RenderLoop::new();

    let presented = loop_.run(&mut shell, &mut engine, 5);

    assert_eq!(presented, 0, "settled: nothing presented");
    assert_eq!(
        engine.render_calls, 0,
        "settled: engine NOT rendered across the boundary"
    );
    assert_eq!(shell.present_calls, 0);
    // The platform has no field-energy state; it only saw the gate's decision.
    assert!(!shell.closed);
}

#[test]
fn resize_intent_reaches_engine_across_boundary() {
    // Resize, then pointer/key intents (NO Close) — frames render and the
    // resize intent must reach the engine via the next render size.
    let mut shell = HeadlessShell::new(32, 32).with_events(vec![
        ShellEvent::Resize(64, 48),
        ShellEvent::Pointer(10.0, 20.0),
        ShellEvent::Key(65),
    ]);
    let mut engine = SceneRenderer::new(tiny_scene(), FieldEquilibrium::default());
    let mut loop_ = RenderLoop::new();

    let presented = loop_.run(&mut shell, &mut engine, 5);
    assert_eq!(presented, 5, "rendered all frames when no Close");
    assert_eq!(
        shell.last_resize,
        Some((64, 48)),
        "Resize reached the shell at the boundary"
    );
    assert_eq!(
        engine.last_size,
        (64, 48),
        "Resize intent reached the engine via render size"
    );
    // All presented frames are at the post-resize size.
    assert!(shell.presented.iter().all(|f| f.len() == 64 * 48 * 4));
}

#[test]
fn close_intent_terminates_loop_at_boundary() {
    let mut shell = HeadlessShell::new(32, 32)
        .with_events(vec![ShellEvent::Resize(64, 48), ShellEvent::Close]);
    let mut engine = SceneRenderer::new(tiny_scene(), FieldEquilibrium::default());
    let mut loop_ = RenderLoop::new();

    let presented = loop_.run(&mut shell, &mut engine, 100);
    // Close fired before any render frame → loop returns immediately.
    assert_eq!(presented, 0, "Close fired before any render frame");
    assert!(shell.closed, "Close event was honored at the boundary");
    // Resize intent still reached the shell even though no frame rendered.
    assert_eq!(shell.last_resize, Some((64, 48)), "Resize observed on shell");
    // The engine (which depends only on FrameSink/render intents) never ran.
    assert_eq!(engine.last_size, (0, 0), "engine untouched when loop closed early");
}

#[test]
fn resize_applied_to_subsequent_renders() {
    let mut shell = HeadlessShell::new(32, 32).with_events(vec![ShellEvent::Resize(16, 16)]);
    let mut engine = SceneRenderer::new(tiny_scene(), FieldEquilibrium::default());
    let mut loop_ = RenderLoop::new();
    loop_.run(&mut shell, &mut engine, 2);
    assert_eq!(engine.last_size, (16, 16), "resize applied to subsequent renders");
    assert!(shell.presented.iter().all(|f| f.len() == 16 * 16 * 4));
}

#[test]
fn verdict_honesty_gate_blocks_non_confirms() {
    let base = VerdictRecord {
        spike: SpikeId::Sp1Desktop,
        hypothesis: "winit+wgpu+AccessKit meets budget",
        method: "FrameProfiler distribution",
        bar: "p95 ≤ 16.7ms",
        measured: "BLOCKED: no physical desktop GPU in CI".to_string(),
        platform: Platform::Desktop { os: DesktopOs::Linux },
        hw_class: HwClass::Emulator,
        verdict: SpikeVerdict::Blocked {
            on: "physical desktop GPU".to_string(),
        },
        captured_utc: 1,
    };
    assert!(!base.is_measured_pass(), "Blocked is never a pass");
    let row = base.to_markdown_row();
    assert!(row.contains("Blocked"), "markdown row names the Blocked arm");

    let refines = VerdictRecord {
        verdict: SpikeVerdict::Refines {
            delta: "caret needs manual SetTextSelection push".to_string(),
        },
        ..base.clone()
    };
    assert!(!refines.is_measured_pass());

    let confirms = VerdictRecord {
        verdict: SpikeVerdict::Confirms,
        ..base.clone()
    };
    assert!(confirms.is_measured_pass(), "Confirms is the only pass");
}
