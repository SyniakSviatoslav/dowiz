//! BLUEPRINT-P63 §2 — the verdict schema (the durable, type-checked output).
//!
//! The verdict schema IS the spec of "done": every SP-* lane writes a
//! `VerdictRecord` row. The `measured` field is either a REAL number / a
//! distribution summary, or the literal string `BLOCKED: <resource>` — never an
//! estimate. The `SpikeVerdict` enum is four-valued (BLUEPRINT-P63 §4.1):
//!
//!   * `Confirms`        — the directional ruling holds (a real pass);
//!   * `Refines{delta}`  — ruling stands + a named bound/caveat;
//!   * `Contradicts{ev}` — measured value crossed the bar the wrong way (evidence
//!                         governs; forces a P39-rev refinement block);
//!   * `Blocked{on}`     — honest arm when a resource (device/account) is absent.
//!                         NOT a pass, NOT a guess.
//!
//! Hazard-safety as math (§4.1): an un-measured/asserted number has no
//! representation — `is_measured_pass()` is `true` ONLY for `Confirms`. And the
//! SP-5 battery row CANNOT be built from an `HwClass::Emulator` reading
//! (`BatteryVerdictRecord::try_new`), so a faked emulator battery figure is
//! unrepresentable at the type level.

/// Which measurement spike a verdict row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpikeId {
    Sp1Desktop,
    Sp2MobileSurface,
    Sp3PaymentBridge,
    Sp4WebKeyboard,
    Sp5Battery,
    Sp6FloorParity,
}

/// The platform a measurement ran on.
#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Desktop { os: DesktopOs },
    MobileAndroid { chipset: String },
    MobileIos { model: String },
    WebMobile { browser: String, os: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopOs {
    Linux,
    MacOs,
    Windows,
}

/// Hardware class. `Emulator` is legal for every spike EXCEPT SP-5 (battery),
/// which forbids it at construction time (see `BatteryVerdictRecord`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwClass {
    BudgetAndroid,
    MidAndroid,
    MidDesktop,
    HighDesktop,
    Emulator,
}

/// The four-valued verdict (BLUEPRINT-P63 §2 / §4.1). `Blocked` is an honest
/// arm, NEVER a pass: an unmeasured value has no representation.
#[derive(Debug, Clone, PartialEq)]
pub enum SpikeVerdict {
    Confirms,
    Refines { delta: String },
    Contradicts { evidence: String },
    Blocked { on: String },
}

/// One row of evidence. `measured` is a REAL number, a distribution summary, or
/// the literal string `BLOCKED: <resource>` — never an estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct VerdictRecord {
    pub spike: SpikeId,
    pub hypothesis: &'static str,
    pub method: &'static str,
    pub bar: &'static str,
    pub measured: String,
    pub platform: Platform,
    pub hw_class: HwClass,
    pub verdict: SpikeVerdict,
    /// Caller-supplied monotonic counter (no ambient clock).
    pub captured_utc: u64,
}

impl VerdictRecord {
    /// A row counts as a real pass ONLY when it `Confirms`. `Refines`/`Contradicts`/
    /// `Blocked` are never a pass — this is the hazard-safety check that an
    /// asserted/blocked number can never masquerade as a measured pass.
    pub fn is_measured_pass(&self) -> bool {
        matches!(self.verdict, SpikeVerdict::Confirms)
    }

    /// Render one markdown table row for `P63-VERDICTS.md`.
    pub fn to_markdown_row(&self) -> String {
        let verdict = match &self.verdict {
            SpikeVerdict::Confirms => "Confirms".to_string(),
            SpikeVerdict::Refines { delta } => format!("Refines ({delta})"),
            SpikeVerdict::Contradicts { evidence } => format!("Contradicts ({evidence})"),
            SpikeVerdict::Blocked { on } => format!("Blocked ({on})"),
        };
        format!(
            "| {:?} | {} | {} | {} | {} | {:?} | {:?} | {} |",
            self.spike,
            self.bar,
            self.method,
            self.measured,
            verdict,
            self.platform,
            self.hw_class,
            self.captured_utc
        )
    }
}

/// The SP-5 honesty gate (BLUEPRINT-P63 §3.5 / §4.1): a battery verdict CANNOT
/// be constructed from an `HwClass::Emulator` reading — an emulator battery
/// number is physically meaningless and must be rejected at the type level. A
/// real battery record can only carry `BudgetAndroid`/`MidAndroid`/… readings,
/// or a `Blocked` verdict naming the missing device.
#[derive(Debug, Clone, PartialEq)]
pub struct BatteryVerdictRecord {
    inner: VerdictRecord,
}

/// Why a battery record could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictError {
    /// SP-5 forbids `HwClass::Emulator` (BLUEPRINT-P63 §3.5).
    EmulatorForbidden,
}

impl BatteryVerdictRecord {
    /// Build a battery verdict record, rejecting any emulator-class reading.
    pub fn try_new(inner: VerdictRecord) -> Result<Self, VerdictError> {
        if matches!(inner.hw_class, HwClass::Emulator) {
            Err(VerdictError::EmulatorForbidden)
        } else {
            Ok(BatteryVerdictRecord { inner })
        }
    }

    pub fn is_measured_pass(&self) -> bool {
        self.inner.is_measured_pass()
    }

    pub fn to_markdown_row(&self) -> String {
        self.inner.to_markdown_row()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> VerdictRecord {
        VerdictRecord {
            spike: SpikeId::Sp1Desktop,
            hypothesis: "winit+wgpu+AccessKit meets budget",
            method: "FrameProfiler distribution",
            bar: "p95 ≤ 16.7ms",
            measured: "BLOCKED: no physical desktop GPU in CI".to_string(),
            platform: Platform::Desktop {
                os: DesktopOs::Linux,
            },
            hw_class: HwClass::Emulator,
            verdict: SpikeVerdict::Blocked {
                on: "physical desktop GPU".to_string(),
            },
            captured_utc: 1,
        }
    }

    #[test]
    fn verdict_honesty_gate_blocks_non_confirms() {
        let b = base();
        assert!(!b.is_measured_pass(), "Blocked is never a pass");
        let row = b.to_markdown_row();
        assert!(row.contains("Blocked"), "markdown names the Blocked arm");

        let refines = VerdictRecord {
            verdict: SpikeVerdict::Refines {
                delta: "caret needs manual SetTextSelection push".to_string(),
            },
            ..b.clone()
        };
        assert!(!refines.is_measured_pass(), "Refines is not a pass");

        let confirms = VerdictRecord {
            verdict: SpikeVerdict::Confirms,
            ..b.clone()
        };
        assert!(confirms.is_measured_pass(), "Confirms is the only pass");

        let contradicts = VerdictRecord {
            verdict: SpikeVerdict::Contradicts {
                evidence: "p95 frame 41ms".to_string(),
            },
            ..b.clone()
        };
        assert!(!contradicts.is_measured_pass(), "Contradicts is not a pass");
    }

    #[test]
    fn sp5_battery_rejects_emulator_at_construction() {
        let emulator = VerdictRecord {
            spike: SpikeId::Sp5Battery,
            ..base()
        };
        assert!(
            matches!(
                BatteryVerdictRecord::try_new(emulator.clone()),
                Err(VerdictError::EmulatorForbidden)
            ),
            "SP-5 battery record MUST reject HwClass::Emulator"
        );

        // A real (honestly-Blocked) record with a physical device class builds.
        let real = VerdictRecord {
            hw_class: HwClass::BudgetAndroid,
            platform: Platform::MobileAndroid {
                chipset: "pixel-a-series".to_string(),
            },
            ..emulator.clone()
        };
        let built = BatteryVerdictRecord::try_new(real);
        assert!(built.is_ok(), "a non-emulator battery record builds");
        assert!(
            !built.unwrap().is_measured_pass(),
            "Blocked battery is not a pass"
        );
    }
}
