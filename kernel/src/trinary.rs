//! `kernel::trinary` — three-valued logic, matrix algebra, RGB delta encoding.
//!
//! Replaces binary (true/false) with trinary (True/False/Unknown) everywhere.
//! Encodes trinary states as RGB color vectors for visualization & delta computation.
//!
//! # Why trinary?
//! Binary logic forces a choice where information is incomplete. Trinary adds
//! the "unknown/uncertain/pending" state that most real-world systems need:
//! - FSM: Closed/Open/Killed → True/False/Unknown (with uncertainty propagation)
//! - Auth: Allow/Deny/Pending — trinary access control
//! - Enrichment: Match/NoMatch/Partial — graded confidence instead of binary
//! - Money: Credit/Debit/Hold — three-way ledger
//!
//! # RGB Encoding
//! Each trinary value maps to an RGB-24 color:
//! - True  = (0, 255, 0) = #00FF00 = Green  (positive/allow/match)
//! - False = (255, 0, 0) = #FF0000 = Red    (negative/deny/no-match)
//! - Unknown = (0, 0, 255) = #0000FF = Blue  (uncertain/pending/partial)
//!
//! Delta between states = Euclidean distance in RGB space → scalar confidence.
//! Matrix of trinary values → RGB image → visual system state at a glance.
//!
//! ZERO external dependencies (pure std).

use std::ops;

/// Three-valued logic value. Replaces `bool` in trinary-aware code paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Tri {
    /// Affirmative / positive / allow / match — encoded as Green.
    True = 0,
    /// Negative / deny / no-match — encoded as Red.
    False = 1,
    /// Uncertain / pending / partial — encoded as Blue.
    Unknown = 2,
}

impl Tri {
    pub fn from_bool(b: bool) -> Self { if b { Tri::True } else { Tri::False } }
    pub fn is_known(&self) -> bool { *self != Tri::Unknown }
    pub fn is_true(&self) -> bool { *self == Tri::True }

    /// RGB-24 encoding: each Tri maps to a canonical color.
    pub fn rgb(&self) -> (u8, u8, u8) {
        match self {
            Tri::True    => (0, 255, 0),    // #00FF00 Green
            Tri::False   => (255, 0, 0),    // #FF0000 Red
            Tri::Unknown => (0, 0, 255),    // #0000FF Blue
        }
    }

    /// Hex color string (e.g. "#00FF00").
    pub fn hex(&self) -> String {
        let (r, g, b) = self.rgb();
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    /// Logical AND in trinary (Kleene logic): Unknown propagates.
    pub fn and(self, other: Tri) -> Tri {
        match (self, other) {
            (Tri::False, _) | (_, Tri::False) => Tri::False,
            (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
            _ => Tri::True,
        }
    }

    /// Logical OR in trinary (Kleene logic).
    pub fn or(self, other: Tri) -> Tri {
        match (self, other) {
            (Tri::True, _) | (_, Tri::True) => Tri::True,
            (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
            _ => Tri::False,
        }
    }

    /// Logical NOT: True↔False, Unknown stays Unknown.
    pub fn not(self) -> Tri {
        match self {
            Tri::True    => Tri::False,
            Tri::False   => Tri::True,
            Tri::Unknown => Tri::Unknown,
        }
    }

    /// Majority vote: returns the most common value (ties → Unknown).
    pub fn majority(votes: &[Tri]) -> Tri {
        let mut counts = [0usize; 3];
        for v in votes { counts[*v as u8 as usize] += 1; }
        let max = counts.iter().max().copied().unwrap_or(0);
        if max == 0 { return Tri::Unknown; }
        let winners: Vec<usize> = (0..3).filter(|&i| counts[i] == max).collect();
        if winners.len() == 1 { Tri::from_u8(winners[0] as u8) } else { Tri::Unknown }
    }

    fn from_u8(v: u8) -> Tri {
        match v { 0 => Tri::True, 1 => Tri::False, _ => Tri::Unknown }
    }
}

// ─── RGB Vector & Delta ────────────────────────────────────────────────────

/// An RGB-24 color vector. Encodes a trinary state or a composite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const GREEN:  Rgb = Rgb(0, 255, 0);
    pub const RED:    Rgb = Rgb(255, 0, 0);
    pub const BLUE:   Rgb = Rgb(0, 0, 255);
    pub const BLACK:  Rgb = Rgb(0, 0, 0);
    pub const WHITE:  Rgb = Rgb(255, 255, 255);

    pub fn from_tri(t: Tri) -> Self {
        let (r, g, b) = t.rgb();
        Rgb(r, g, b)
    }

    /// Euclidean distance between two RGB vectors (0–441).
    /// Normalized to [0, 1] for confidence scoring.
    pub fn delta(&self, other: &Rgb) -> f64 {
        let dr = self.0 as f64 - other.0 as f64;
        let dg = self.1 as f64 - other.1 as f64;
        let db = self.2 as f64 - other.2 as f64;
        (dr * dr + dg * dg + db * db).sqrt()
    }

    /// Normalized delta [0, 1] — 0 = identical, 1 = opposite.
    pub fn delta_norm(&self, other: &Rgb) -> f64 {
        (self.delta(other) / 441.67).clamp(0.0, 1.0) // max euclidean dist in RGB cube
    }

    /// Blend two RGB vectors by weight w ∈ [0,1].
    pub fn blend(&self, other: &Rgb, w: f64) -> Rgb {
        let w = w.clamp(0.0, 1.0);
        Rgb(
            (self.0 as f64 * (1.0 - w) + other.0 as f64 * w) as u8,
            (self.1 as f64 * (1.0 - w) + other.1 as f64 * w) as u8,
            (self.2 as f64 * (1.0 - w) + other.2 as f64 * w) as u8,
        )
    }

    /// Hex string.
    pub fn hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

impl ops::Add for Rgb {
    type Output = Rgb;
    fn add(self, rhs: Rgb) -> Rgb {
        Rgb(self.0.saturating_add(rhs.0), self.1.saturating_add(rhs.1), self.2.saturating_add(rhs.2))
    }
}

impl ops::Sub for Rgb {
    type Output = Rgb;
    fn sub(self, rhs: Rgb) -> Rgb {
        Rgb(self.0.saturating_sub(rhs.0), self.1.saturating_sub(rhs.1), self.2.saturating_sub(rhs.2))
    }
}

// ─── TriMatrix — N×M matrix of trinary values with RGB encoding ────────────

/// An N×M matrix of trinary values. Each cell = Tri. The whole matrix
/// encodes as an RGB bitmap for visualization and delta computation.
#[derive(Debug, Clone)]
pub struct TriMatrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<Tri>,
}

impl TriMatrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        TriMatrix { rows, cols, data: vec![Tri::Unknown; rows * cols] }
    }

    pub fn get(&self, r: usize, c: usize) -> Tri {
        self.data.get(r * self.cols + c).copied().unwrap_or(Tri::Unknown)
    }

    pub fn set(&mut self, r: usize, c: usize, v: Tri) {
        if let Some(cell) = self.data.get_mut(r * self.cols + c) {
            *cell = v;
        }
    }

    /// Row-wise RGB encoding: each row → 3 bytes (R,G,B average).
    pub fn row_rgb(&self, r: usize) -> Rgb {
        if r >= self.rows { return Rgb::BLACK; }
        let start = r * self.cols;
        let end = start + self.cols;
        let mut rs = 0u64; let mut gs = 0u64; let mut bs = 0u64;
        for i in start..end {
            let (rr, gg, bb) = self.data[i].rgb();
            rs += rr as u64; gs += gg as u64; bs += bb as u64;
        }
        let n = self.cols as u64;
        Rgb((rs / n) as u8, (gs / n) as u8, (bs / n) as u8)
    }

    /// Full RGB bitmap: rows → [Rgb; rows].
    pub fn bitmap(&self) -> Vec<Rgb> {
        (0..self.rows).map(|r| self.row_rgb(r)).collect()
    }

    /// Delta between two matrices (average row-delta, normalized).
    pub fn delta(&self, other: &TriMatrix) -> f64 {
        let r = self.rows.min(other.rows);
        if r == 0 { return 0.0; }
        let mut total = 0.0;
        for i in 0..r {
            total += self.row_rgb(i).delta_norm(&other.row_rgb(i));
        }
        total / r as f64
    }

    /// Majority vote per column (column → Tri).
    pub fn column_majority(&self, c: usize) -> Tri {
        if c >= self.cols { return Tri::Unknown; }
        let votes: Vec<Tri> = (0..self.rows).map(|r| self.get(r, c)).collect();
        Tri::majority(&votes)
    }

    /// Count of each Tri value.
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut t = 0; let mut f = 0; let mut u = 0;
        for &v in &self.data {
            match v { Tri::True => t += 1, Tri::False => f += 1, Tri::Unknown => u += 1 }
        }
        (t, f, u)
    }
}

// ─── DeltaChain — sequence of RGB deltas for state transitions ─────────────

/// Tracks state changes as RGB deltas over time. Each step = previous→current delta.
#[derive(Debug, Clone)]
pub struct DeltaChain {
    pub history: Vec<(Rgb, f64)>,  // (current_state, delta_from_previous)
}

impl DeltaChain {
    pub fn new() -> Self { DeltaChain { history: Vec::new() } }

    pub fn push(&mut self, state: Rgb) {
        let delta = if let Some((prev, _)) = self.history.last() {
            state.delta_norm(prev)
        } else {
            0.0
        };
        self.history.push((state, delta));
    }

    /// Cumulative drift (sum of all deltas).
    pub fn total_drift(&self) -> f64 {
        self.history.iter().map(|(_, d)| d).sum()
    }

    /// Is the system stable? (recent deltas below threshold).
    pub fn is_stable(&self, window: usize, threshold: f64) -> bool {
        let start = self.history.len().saturating_sub(window);
        self.history[start..].iter().all(|(_, d)| *d <= threshold)
    }

    pub fn len(&self) -> usize { self.history.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_kleene_and() {
        assert_eq!(Tri::True.and(Tri::True), Tri::True);
        assert_eq!(Tri::True.and(Tri::False), Tri::False);
        assert_eq!(Tri::True.and(Tri::Unknown), Tri::Unknown);
        assert_eq!(Tri::False.and(Tri::Unknown), Tri::False); // false wins
    }

    #[test]
    fn tri_kleene_or() {
        assert_eq!(Tri::True.or(Tri::False), Tri::True);
        assert_eq!(Tri::False.or(Tri::Unknown), Tri::Unknown);
        assert_eq!(Tri::Unknown.or(Tri::Unknown), Tri::Unknown);
    }

    #[test]
    fn tri_not() {
        assert_eq!(Tri::True.not(), Tri::False);
        assert_eq!(Tri::False.not(), Tri::True);
        assert_eq!(Tri::Unknown.not(), Tri::Unknown);
    }

    #[test]
    fn rgb_delta_opposite() {
        // Green→Red: sqrt((0-255)²+(255-0)²+(0-0)²) = sqrt(130050) ≈ 360.6 / 441.67 ≈ 0.816
        let d = Rgb::GREEN.delta_norm(&Rgb::RED);
        assert!((d - 0.816).abs() < 0.01, "delta={d}");
        assert!((Rgb::GREEN.delta_norm(&Rgb::GREEN) - 0.0).abs() < 0.01);
    }

    #[test]
    fn rgb_blend_midpoint() {
        let mid = Rgb::GREEN.blend(&Rgb::RED, 0.5);
        assert_eq!(mid, Rgb(127, 127, 0));
    }

    #[test]
    fn tri_matrix_row_rgb() {
        let mut m = TriMatrix::new(1, 3);
        m.set(0, 0, Tri::True);
        m.set(0, 1, Tri::False);
        m.set(0, 2, Tri::True);
        // (0,255,0)+(255,0,0)+(0,255,0) = (255,510,0)/3 = (85,170,0)
        let rgb = m.row_rgb(0);
        assert_eq!(rgb.0, 85);
        assert_eq!(rgb.1, 170);
        assert_eq!(rgb.2, 0);
    }

    #[test]
    fn tri_matrix_majority() {
        let mut m = TriMatrix::new(3, 1);
        m.set(0, 0, Tri::True);
        m.set(1, 0, Tri::True);
        m.set(2, 0, Tri::False);
        assert_eq!(m.column_majority(0), Tri::True);
    }

    #[test]
    fn tri_matrix_majority_tie() {
        let mut m = TriMatrix::new(2, 1);
        m.set(0, 0, Tri::True);
        m.set(1, 0, Tri::False);
        assert_eq!(m.column_majority(0), Tri::Unknown); // tie
    }

    #[test]
    fn delta_chain_stable() {
        let mut dc = DeltaChain::new();
        dc.push(Rgb::GREEN);
        dc.push(Rgb::GREEN);  // delta 0
        dc.push(Rgb(0, 254, 1)); // tiny delta
        assert!(dc.is_stable(3, 0.01));
    }

    #[test]
    fn delta_chain_unstable() {
        let mut dc = DeltaChain::new();
        dc.push(Rgb::GREEN);
        dc.push(Rgb::RED); // big delta
        assert!(!dc.is_stable(2, 0.1));
    }

    #[test]
    fn hex_encoding() {
        assert_eq!(Tri::True.hex(), "#00FF00");
        assert_eq!(Tri::False.hex(), "#FF0000");
        assert_eq!(Tri::Unknown.hex(), "#0000FF");
    }

    #[test]
    fn tri_majority_unanimous() {
        assert_eq!(Tri::majority(&[Tri::True, Tri::True, Tri::True]), Tri::True);
    }

    #[test]
    fn tri_majority_tie_votes() {
        assert_eq!(Tri::majority(&[Tri::True, Tri::False]), Tri::Unknown);
    }

    #[test]
    fn rgb_add_sub() {
        let a = Rgb(100, 150, 200);
        let b = Rgb(50, 30, 10);
        assert_eq!(a + b, Rgb(150, 180, 210));
        assert_eq!(a - b, Rgb(50, 120, 190));
    }
}
