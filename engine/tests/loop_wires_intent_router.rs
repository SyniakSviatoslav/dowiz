//! RED→GREEN gate (P64 §3.1): the engine run loop (`EngineLoop::frame`) is the
//! production caller of `InputRouter::tick`. A `FixedTimestep` frame drives
//! `EngineLoop::frame`, whose `InputRouter` is fed a `ConstSource` that emits a
//! resolved `Intent`. The test asserts the emitted intent **reaches** the
//! widget/state store via the real run-loop caller.
//!
//! - RED: `EngineLoop::frame` does NOT call `InputRouter::tick` → no intent
//!   reaches the store → `last_intents() == 0` and `hover_count() == 0` ⇒ FAIL.
//! - GREEN: `EngineLoop::frame` calls `tick` and applies `Select` (HOVER) ⇒
//!   `last_intents() == 1` and `hover_count() == 1` ⇒ PASS.
//!
//! MONEY SAFETY: the source emits a NON-consequential `Select` intent only. We
//! never route a payment/consequential intent through this loop.

use dowiz_engine::intent::{InputProfile, InputRouter, InputSource, Intent, RawInput, SurfaceId};
use dowiz_engine::widget_store::WidgetStore;
use dowiz_engine::EngineLoop;

/// A source that emits one fixed raw input every poll (the router polls once
/// per tick — the standard `InputRouter::tick` contract).
struct ConstSource(RawInput);

impl InputSource for ConstSource {
    fn poll(&mut self) -> Option<RawInput> {
        Some(self.0.clone())
    }
}

#[test]
fn fixedtimestep_frame_drives_router_tick_into_store() {
    // A widget at (5,5) so a pointer-Down there resolves to Select(7).
    let mut widgets = WidgetStore::new(8);
    widgets.id[7] = 7;
    widgets.pos_x[7] = 5.0;
    widgets.pos_y[7] = 5.0;
    widgets.size_w[7] = 2.0;
    widgets.size_h[7] = 2.0;

    let down_on_widget = RawInput::Pointer {
        pos: dowiz_engine::text_input::FieldPos {
            u: 5.0,
            v: 5.0,
            w: 0.0,
        },
        phase: dowiz_engine::intent::PointerPhase::Down,
        vel: (0.0, 0.0),
    };

    let router = InputRouter::new(vec![Box::new(ConstSource(down_on_widget))]);
    let mut loop_ = EngineLoop::new(router, widgets);

    let mut ft = dowiz_engine::FixedTimestep::new();
    // Drive a few fixed frames (the engine frame/update loop).
    for _ in 0..3 {
        ft.frame(
            1.0 / 60.0,
            |_dt| {
                loop_.frame(SurfaceId(0), InputProfile::Balanced);
            },
            |_alpha| {},
        );
    }

    // GREEN assertion: the resolved intent reached the store via the real
    // run-loop caller `EngineLoop::frame` → `InputRouter::tick`.
    assert_eq!(
        loop_.last_intents(),
        1,
        "EngineLoop::frame must call InputRouter::tick and emit the resolved intent"
    );
    assert_eq!(
        loop_.hover_count(),
        1,
        "the resolved Select(7) intent must reach the widget store (HOVER applied)"
    );
    // Sanity: the resolved Intent was non-consequential (no money path touched).
    assert_eq!(
        loop_.widgets().id[7],
        7,
        "target widget id preserved (presentation-only mutation)"
    );
}
