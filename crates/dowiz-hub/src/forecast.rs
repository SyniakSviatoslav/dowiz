//! PURE. THE KITCHEN'S FORECAST (W-PREP; row P6 of
//! `docs/research/2026-10-03-depth-stock-analytics-models.md`, K14 of
//! `docs/research/2026-09-26-kitchen-role.md`): how many orders and portions a
//! venue-day will see, from the same weekday over the last weeks -- and how
//! far off that guess usually is, MEASURED on the venue's own days.
//!
//! NO CLOCK. `as_of` is the first day NOT in the history (the venue's today):
//! the caller hands it in. INTEGERS ONLY: orders and portions are whole, TSB
//! runs in millionths, MASE is per mille. No person is in here: a day is
//! counts of orders and of portions per dish.
//!
//! THE RULES, each pinned by `forecast/tests.rs`:
//!   * A SAMPLE is the same weekday 1..=`WEEKS_MAX` weeks before the day, taken
//!     only before `as_of` and not before the series' first day. A day in that
//!     range with no row is a zero (closed, or nobody came).
//!   * Fewer than `WEEKS_MIN` samples: LEARNING, never a number.
//!   * A REGULAR series (at most a third of the samples are zero): the median.
//!     An INTERMITTENT one: TSB (Teunter, Syntetos & Babai 2011), which
//!     smooths how OFTEN a dish sells apart from how MUCH.
//!   * THE ERROR: each of the `BACKTEST_DAYS` days before `as_of`, forecast as
//!     of that day, against what was sold, beside the seasonal-naive guess (the
//!     same weekday a week before). Both counted on the same days only.

use std::collections::BTreeMap;

/// Below this many same-weekday samples a series says "learning".
pub const WEEKS_MIN: usize = 3;
/// The most same-weekday samples a forecast reads.
pub const WEEKS_MAX: usize = 8;
/// The days the error is measured over: four weeks.
pub const BACKTEST_DAYS: i64 = 4 * 7;
/// TSB's smoothing of the size and of the probability, per mille.
pub const TSB_ALPHA_PM: i64 = 200;
pub const TSB_BETA_PM: i64 = 200;
/// Where the day is cut into bands, in minutes after midnight: 14:00, 18:00.
pub const CUTS: [i64; 2] = [14 * 60, 18 * 60];
/// An hour before this belongs to the evening that ran past midnight.
pub const NIGHT_ENDS: usize = 4;
const MICRO: i128 = 1_000_000;
const DAY_MIN: i64 = 24 * 60;

/// One venue-day as the forecast reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Day {
    /// Accepted orders (a refused order made no food).
    pub orders: i64,
    /// Orders placed per band (`band_of_hour`).
    pub bands: [i64; 3],
    /// Portions sold, per dish id.
    pub dishes: BTreeMap<String, i64>,
}

impl Day {
    pub fn portions(&self) -> i64 {
        self.dishes.values().sum()
    }
}

/// The venue's days by day number (1970-01-01 = 0).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    pub days: BTreeMap<i64, Day>,
}

/// How a number was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Median,
    Tsb,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Median => "median",
            Method::Tsb => "tsb",
        }
    }
}

/// A forecast, or the honest absence of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estimate {
    Learning { weeks: usize },
    Number { value: i64, method: Method, weeks: usize },
}

impl Estimate {
    pub fn value(&self) -> Option<i64> {
        match self {
            Estimate::Number { value, .. } => Some(*value),
            Estimate::Learning { .. } => None,
        }
    }
    pub fn weeks(&self) -> usize {
        match self {
            Estimate::Number { weeks, .. } | Estimate::Learning { weeks } => *weeks,
        }
    }
}

/// The median, the middle pair's mean rounded half up. 0 for nothing.
pub fn median(v: &[i64]) -> i64 {
    let mut s = v.to_vec();
    s.sort_unstable();
    let n = s.len();
    match n {
        0 => 0,
        _ if n % 2 == 1 => s[n / 2],
        _ => (s[n / 2 - 1] + s[n / 2] + 1).div_euclid(2),
    }
}

/// More than a third of the samples are zero.
pub fn intermittent(v: &[i64]) -> bool {
    v.iter().filter(|x| **x <= 0).count() * 3 > v.len()
}

/// TSB over `v` (oldest first), in millionths per period. Starts from the
/// series' own share of selling periods and mean size, then updates per
/// period: a sale moves the probability toward 1 and the size toward it; a
/// zero moves the probability toward 0 and leaves the size.
pub fn tsb_micro(v: &[i64]) -> i64 {
    let sold: Vec<i128> = v.iter().filter(|x| **x > 0).map(|x| i128::from(*x)).collect();
    if sold.is_empty() {
        return 0;
    }
    let (a, b) = (i128::from(TSB_ALPHA_PM), i128::from(TSB_BETA_PM));
    let mut p = sold.len() as i128 * MICRO / v.len() as i128;
    let mut z = sold.iter().sum::<i128>() * MICRO / sold.len() as i128;
    for x in v {
        if *x > 0 {
            p += (MICRO - p) * b / 1000;
            z += (i128::from(*x) * MICRO - z) * a / 1000;
        } else {
            p -= p * b / 1000;
        }
    }
    i64::try_from(p * z / MICRO).unwrap_or(i64::MAX)
}

/// The sample days for `target`, oldest first: the same weekday 1..=WEEKS_MAX
/// weeks back, before `as_of`, not before `first`.
pub fn samples(first: i64, as_of: i64, target: i64) -> Vec<i64> {
    (1..=WEEKS_MAX as i64).rev().map(|k| target - 7 * k).filter(|d| *d >= first && *d < as_of).collect()
}

/// One series' estimate for `target`. `f` reads a day; `first` is the
/// series' first day (the venue's, or a dish's first sale).
pub fn estimate_of(h: &History, first: i64, as_of: i64, target: i64, f: &dyn Fn(&Day) -> i64) -> Estimate {
    let v: Vec<i64> = samples(first, as_of, target).iter().map(|d| h.days.get(d).map_or(0, f)).collect();
    let weeks = v.len();
    if weeks < WEEKS_MIN {
        return Estimate::Learning { weeks };
    }
    if intermittent(&v) {
        let x = i128::from(tsb_micro(&v));
        return Estimate::Number { value: i64::try_from((x + MICRO / 2) / MICRO).unwrap_or(i64::MAX), method: Method::Tsb, weeks };
    }
    Estimate::Number { value: median(&v), method: Method::Median, weeks }
}

/// The venue's first day, and a dish's first sale, before `as_of`.
pub fn first_day(h: &History, as_of: i64) -> Option<i64> {
    h.days.range(..as_of).next().map(|(d, _)| *d)
}

pub fn first_sale(h: &History, dish: &str, as_of: i64) -> Option<i64> {
    h.days.range(..as_of).find(|(_, d)| d.dishes.get(dish).is_some_and(|q| *q > 0)).map(|(n, _)| *n)
}

/// The measured error of one series: absolute errors summed over the days
/// both guesses could be made.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Error {
    pub days: i64,
    pub model: i64,
    pub naive: i64,
}

impl Error {
    /// The model's mean absolute error, rounded half up: "usually off by ±n".
    pub fn off_by(&self) -> Option<i64> {
        (self.days > 0).then(|| (self.model * 2 + self.days).div_euclid(self.days * 2))
    }
    pub fn naive_off_by(&self) -> Option<i64> {
        (self.days > 0).then(|| (self.naive * 2 + self.days).div_euclid(self.days * 2))
    }
    /// MASE, per mille: the model's error over the seasonal-naive one's.
    pub fn mase_pm(&self) -> Option<i64> {
        (self.naive > 0).then(|| self.model * 1000 / self.naive)
    }
    pub fn add(&mut self, o: Error) {
        self.days += o.days;
        self.model += o.model;
        self.naive += o.naive;
    }
}

/// Forecast each of the `BACKTEST_DAYS` before `as_of` as of that day and
/// compare with what happened, beside last week's same day.
pub fn backtest(h: &History, first: i64, as_of: i64, f: &dyn Fn(&Day) -> i64) -> Error {
    let mut e = Error::default();
    let at = |d: i64| h.days.get(&d).map_or(0, f);
    for t in (as_of - BACKTEST_DAYS).max(first + 7)..as_of {
        if let Some(g) = estimate_of(h, first, t, t, f).value() {
            e.days += 1;
            e.model += (g - at(t)).abs();
            e.naive += (at(t - 7) - at(t)).abs();
        }
    }
    e
}

/// The band an hour of the venue's clock falls in: 0 opening to 14:00, 1
/// 14:00 to 18:00, 2 from 18:00 -- and the small hours, which belong to the
/// evening before.
pub fn band_of_hour(hour: usize) -> usize {
    match hour {
        h if h < NIGHT_ENDS => 2,
        h if (h as i64) * 60 < CUTS[0] => 0,
        h if (h as i64) * 60 < CUTS[1] => 1,
        _ => 2,
    }
}

/// An hour-by-hour count, summed per band.
pub fn bands_of(hours: &[i64; 24]) -> [i64; 3] {
    let mut b = [0; 3];
    for (h, n) in hours.iter().enumerate() {
        b[band_of_hour(h)] += n;
    }
    b
}

/// The bands one weekday's opening windows `(open, close)` reach, in
/// minutes (a close at or before the open runs past midnight; `to` may then
/// pass 1440). `None`: the venue is not open in that band.
pub fn bands_for(windows: &[(i64, i64)]) -> [Option<(i64, i64)>; 3] {
    let mut out: [Option<(i64, i64)>; 3] = [None; 3];
    let edges = [i64::MIN, CUTS[0], CUTS[1], i64::MAX];
    for (open, close) in windows {
        // A window that opens in the small hours is the evening's tail.
        let shift = if *open < NIGHT_ENDS as i64 * 60 { DAY_MIN } else { 0 };
        let (open, close) = (open + shift, close + shift);
        let close = if close <= open { close + DAY_MIN } else { close };
        let open = &open;
        for b in 0..3 {
            let (lo, hi) = ((*open).max(edges[b]), close.min(edges[b + 1]));
            if lo < hi {
                out[b] = Some(out[b].map_or((lo, hi), |(f, t)| (f.min(lo), t.max(hi))));
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "forecast/tests.rs"]
mod tests;
