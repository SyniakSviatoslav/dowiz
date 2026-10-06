//! THE SWITCH AND THE SHADOW (W-SNN row 3): per venue `snn = shadow | on | off`, default shadow.
//!
//!   off      nothing about the network runs: not the weights' decode, not one stalk.
//!   shadow   the network ranks beside the current ranker; the guest is shown the CURRENT
//!            ranker's answer, byte for byte; only an aggregate per venue is counted.
//!   on       the network's answer is shown -- for after the A/B shows a win (operator's call).
//! A network that cannot run (a refused blob, an empty answer) never takes the guest's answer
//! away: whatever the mode, the current ranker's list is what is shown then.
//!
//! THE TALLY IS PER VENUE AND HAS NO GUEST IN IT: how many comparisons, how often the first dish
//! agreed, how many of the top-k were shared, how many times the network could not run. No key, no
//! id, no dish list is kept.

use serde::{Deserialize, Serialize};

use super::Refusal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Off,
    Shadow,
    On,
}

impl Mode {
    /// The settings value; anything unknown (or absent) reads as the default, `shadow`.
    pub fn parse(v: Option<&str>) -> Mode {
        match v.map(str::trim) {
            Some("off") => Mode::Off,
            Some("on") => Mode::On,
            _ => Mode::Shadow,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Off => "off",
            Mode::Shadow => "shadow",
            Mode::On => "on",
        }
    }

    /// The values an owner may set (the route refuses anything else by name).
    pub const ALL: [&'static str; 3] = ["shadow", "on", "off"];
}

/// What one comparison came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `off`: nothing ran.
    Off,
    /// The network could not run; the reason is named (a refused blob).
    Unusable(String),
    /// Both ranked: does the first dish agree, and how many of the first `k` are shared.
    Compared { top1_same: bool, shared: usize, k: usize },
}

/// One request: the list to SHOW and what the comparison came to. `network` runs only when the
/// mode is not `off`; it returns the network's top-k or the refusal.
pub fn decide<F>(mode: Mode, current: Vec<(String, i64)>, k: usize, network: F) -> (Vec<(String, i64)>, Outcome)
where
    F: FnOnce() -> Result<Vec<(String, i64)>, Refusal>,
{
    if mode == Mode::Off {
        return (current, Outcome::Off);
    }
    let snn = match network() {
        Ok(v) => v,
        Err(r) => return (current, Outcome::Unusable(r.to_string())),
    };
    let head = |v: &[(String, i64)]| v.iter().take(k).map(|x| x.0.clone()).collect::<Vec<_>>();
    let (a, b) = (head(&current), head(&snn));
    let outcome = Outcome::Compared {
        top1_same: !a.is_empty() && a.first() == b.first(),
        shared: a.iter().filter(|x| b.contains(x)).count(),
        k: a.len().max(b.len()),
    };
    let shown = if mode == Mode::On && !snn.is_empty() { snn } else { current };
    (shown, outcome)
}

/// The per-venue counter, as kept in the venue's `snn` table (one record). Fields added later
/// read as 0 from an older record (`serde(default)`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    pub compared: i64,
    pub top1_same: i64,
    /// Sum over comparisons of the dishes the two top-k shared.
    pub shared: i64,
    /// Sum over comparisons of k (so `shared / slots` is the overlap).
    pub slots: i64,
    pub unusable: i64,
    /// The first and last day (UTC day number) anything was counted.
    pub since_day: i64,
    pub last_day: i64,
    /// The model the counts are about (`Model::id`); a new model starts a new tally.
    pub model: u32,
    /// QUALITY (operator 2026-10-06): guests' next orders checked against the two top-3s held
    /// since their order page (`quality::Held`), and how many landed in each.
    pub settled: i64,
    pub current_hit: i64,
    pub snn_hit: i64,
}

impl Tally {
    pub fn add(&mut self, o: &Outcome, day: i64, model: u32) {
        if self.model != model {
            *self = Tally { model, ..Tally::default() };
        }
        match o {
            Outcome::Off => return,
            Outcome::Unusable(_) => self.unusable += 1,
            Outcome::Compared { top1_same, shared, k } => {
                self.compared += 1;
                self.top1_same += i64::from(*top1_same);
                self.shared += *shared as i64;
                self.slots += *k as i64;
            }
        }
        if self.since_day == 0 {
            self.since_day = day;
        }
        self.last_day = day;
    }

    /// One next order, checked: did a dish of it land in the current top-3, in the network's?
    /// A check about another model than the tally's is not counted (that tally was restarted).
    pub fn settle(&mut self, current_hit: bool, snn_hit: bool, day: i64, model: u32) {
        if self.model != model {
            return;
        }
        self.settled += 1;
        self.current_hit += i64::from(current_hit);
        self.snn_hit += i64::from(snn_hit);
        self.last_day = self.last_day.max(day);
    }

    /// Next orders that met the current ranker's top-3 / the network's, per mille of those settled.
    pub fn hit_pm(&self) -> (i64, i64) {
        if self.settled == 0 {
            (0, 0)
        } else {
            (self.current_hit * 1000 / self.settled, self.snn_hit * 1000 / self.settled)
        }
    }

    /// Top-k overlap, per mille (0 with nothing compared).
    pub fn overlap_pm(&self) -> i64 {
        if self.slots == 0 {
            0
        } else {
            self.shared * 1000 / self.slots
        }
    }

    /// First-dish agreement, per mille.
    pub fn top1_pm(&self) -> i64 {
        if self.compared == 0 {
            0
        } else {
            self.top1_same * 1000 / self.compared
        }
    }
}
