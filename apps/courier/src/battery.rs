//! battery.rs — BLUEPRINT P71 R5: the P63 SP-5 battery gate (conditional).
//!
//! The courier battery gate is SOURCED from P63's SP-5 baseline (P63:170-173) —
//! NOT invented here. P71 does NOT run the rig (P63 SP-5 owns it). Because
//! `P63-VERDICTS.md` is ABSENT this pass (SP-5 verdict = `Blocked{physical
//! budget Android device}`), the gate's ASSERTION is `#[ignore="P63-SP5-baseline"]`
//! until SP-5 lands a real (non-`Blocked`) `VerdictRecord` — the
//! ignored-not-deleted honesty convention.
//!
//! `HwClass::Emulator` is REJECTED (P63 forbids it; P71 must not consume a
//! meaningless number). No battery %/h is ever written as MEASURED.

use crate::types::{
    BATTERY_SETTLED_DRAIN_MAX_PCT_HR, BATTERY_SETTLE_SAVINGS_MIN_PCT, BATTERY_SHIFT_HOURS,
    BATTERY_THERMAL_SUSTAIN_MS,
};

/// P63 `HwClass` (P63:128). The courier gate only accepts `BudgetAndroid`
/// (emulator forbidden — the rig must run on real budget hardware).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwClass {
    BudgetAndroid,
    Flagship,
    /// forbidden for the courier battery verdict — a meaningless number.
    Emulator,
}

/// P63 SP-5 `VerdictRecord` (the shape P71 consumes). Until SP-5 lands, the
/// verdict is `Blocked` and the number is OWED, not faked.
#[derive(Debug, Clone, PartialEq)]
pub enum VerdictRecord {
    /// SP-5 has not measured yet (no physical budget-Android device).
    Blocked { on: String },
    /// SP-5 measured and confirms the baseline.
    Confirms {
        hw: HwClass,
        shift_hours: f64,
        settled_drain_pct_hr: f64,
        settle_saving_pct: f64,
        thermal_sustain_ms: f64,
    },
    /// SP-5 measured and contradicts the baseline (e.g. settle saves < 30 %).
    /// P71 applies SP-5's one-line refinement (a nav-mode power tier) before
    /// going green.
    Refines {
        hw: HwClass,
        settle_saving_pct: f64,
        note: String,
    },
    /// SP-5 measured and contradicts fatally.
    Contradicts { hw: HwClass, reason: String },
}

impl VerdictRecord {
    /// True iff this verdict carries a REAL, usable (non-`Blocked`, non-
    /// `Emulator`) battery number that the courier gate may assert against.
    pub fn is_measured(&self) -> bool {
        match self {
            VerdictRecord::Confirms { hw, .. } | VerdictRecord::Refines { hw, .. } => {
                *hw != HwClass::Emulator
            }
            VerdictRecord::Contradicts { hw, .. } => *hw != HwClass::Emulator,
            VerdictRecord::Blocked { .. } => false,
        }
    }
}

/// The courier battery gate result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatteryGate {
    /// The gate's number is OWED (SP-5 Blocked) — cannot assert yet.
    Owed,
    /// The verdict carries an emulator number — rejected, never consumed.
    RejectedEmulator,
    /// The measured numbers satisfy the P63 SP-5 bars.
    Pass,
    /// The measured numbers fail a bar (names the failed bar).
    Fail(&'static str),
}

/// Evaluate the courier battery gate against a P63 SP-5 verdict. The bars are
/// imported from P71's §2 constants (sourced from P63). Returns `Owed` until
/// SP-5 lands a real `VerdictRecord` (the honest default this pass).
pub fn evaluate_battery_gate(v: &VerdictRecord) -> BatteryGate {
    if !v.is_measured() {
        // Blocked, or an Emulator verdict ⇒ the number is owed / rejected.
        match v {
            VerdictRecord::Blocked { .. } => BatteryGate::Owed,
            VerdictRecord::Confirms { hw, .. }
            | VerdictRecord::Refines { hw, .. }
            | VerdictRecord::Contradicts { hw, .. } => {
                if *hw == HwClass::Emulator {
                    BatteryGate::RejectedEmulator
                } else {
                    BatteryGate::Owed
                }
            }
        }
    } else if let VerdictRecord::Confirms {
        settled_drain_pct_hr,
        settle_saving_pct,
        thermal_sustain_ms,
        ..
    } = v
    {
        if *settled_drain_pct_hr > BATTERY_SETTLED_DRAIN_MAX_PCT_HR {
            return BatteryGate::Fail("settled_drain");
        }
        if *settle_saving_pct < BATTERY_SETTLE_SAVINGS_MIN_PCT {
            return BatteryGate::Fail("settle_saving");
        }
        if *thermal_sustain_ms > BATTERY_THERMAL_SUSTAIN_MS {
            return BatteryGate::Fail("thermal");
        }
        BatteryGate::Pass
    } else {
        // Refines/Contradicts measured but not a Confirms ⇒ gate not green yet
        // (refinement/nav-tier pending). Treat as Owed until re-lifted.
        BatteryGate::Owed
    }
}

/// The honest default this pass: SP-5 is Blocked (no `P63-VERDICTS.md`).
pub fn default_sp5_verdict() -> VerdictRecord {
    VerdictRecord::Blocked {
        on: "physical budget Android device / first-client device fleet".into(),
    }
}

/// Convenience: the imported bars (so tests + the gate share one source).
pub fn battery_bars() -> (f64, f64, f64, f64) {
    (
        BATTERY_SHIFT_HOURS,
        BATTERY_SETTLED_DRAIN_MAX_PCT_HR,
        BATTERY_SETTLE_SAVINGS_MIN_PCT,
        BATTERY_THERMAL_SUSTAIN_MS,
    )
}
