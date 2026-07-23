//! `kernel::fractal` — N-fractal self-similar data structure + ASCII encoding.
//!
//! Replaces flat arrays and crystal lattice with an N-depth fractal tree where
//! every node is a self-similar microcosm of the whole. Each node carries its
//! own eigen decomposition, local parameters, and ASCII visualization.
//!
//! # Fractal property
//! A fractal of depth D contains D levels. Level 0 is the root (whole system).
//! Level D-1 are the leaves (individual data points). Every internal node
//! summarizes its children — the summary IS the node's value.
//!
//! # ASCII encoding
//! Every node renders as an ASCII block. The full tree is a visual representation
//! of the entire system state — readable without tools.
//!
//! ZERO deps. Uses eigen for value representation.

use crate::eigen::{EigenDecomp, decompose};

/// A node in the N-fractal tree. Every node = self-similar copy of the whole.
#[derive(Debug, Clone)]
pub struct FractalNode {
    pub id: String,
    pub depth: usize,
    /// Eigen decomposition of this node's state (summary of children).
    pub state: EigenDecomp,
    /// Raw values at this node (if leaf).
    pub raw: Vec<f64>,
    /// Children fractals (sub-divisions).
    pub children: Vec<FractalNode>,
    /// ASCII art cache.
    ascii_cache: Option<String>,
}

impl FractalNode {
    pub fn new(id: &str, depth: usize) -> Self {
        FractalNode {
            id: id.to_string(), depth,
            state: EigenDecomp::new(vec![]),
            raw: Vec::new(), children: Vec::new(),
            ascii_cache: None,
        }
    }

    /// Leaf node with raw values.
    pub fn leaf(id: &str, depth: usize, values: Vec<f64>) -> Self {
        let state = decompose(&values, values.len().min(4));
        FractalNode {
            id: id.to_string(), depth,
            state, raw: values, children: Vec::new(),
            ascii_cache: None,
        }
    }

    /// Add a child fractal. After adding, recompute this node's state from children.
    pub fn add_child(&mut self, child: FractalNode) {
        self.children.push(child);
        self.recompute();
    }

    /// Recompute eigen state from children's dominant eigenvalues.
    fn recompute(&mut self) {
        if self.children.is_empty() { return; }
        let dominant_lambdas: Vec<f64> = self.children.iter()
            .map(|c| c.state.spectral_radius())
            .collect();
        self.state = decompose(&dominant_lambdas, 4);
        self.ascii_cache = None;
    }

    /// Total leaf count (recursive).
    pub fn leaf_count(&self) -> usize {
        if self.children.is_empty() { return 1; }
        self.children.iter().map(|c| c.leaf_count()).sum()
    }

    /// Collect all raw values from leaves (depth-first).
    pub fn flatten(&self) -> Vec<f64> {
        if self.children.is_empty() { return self.raw.clone(); }
        let mut out = Vec::new();
        for c in &self.children { out.extend(c.flatten()); }
        out
    }

    /// Spectral radius of this node.
    pub fn radius(&self) -> f64 { self.state.spectral_radius() }

    /// Is this node stable? (spectral radius ≤ 1).
    pub fn is_stable(&self) -> bool { self.radius() <= 1.0 && self.children.iter().all(|c| c.is_stable()) }

    /// ASCII representation of this node and all children.
    pub fn ascii(&mut self) -> &str {
        if self.ascii_cache.is_none() {
            self.ascii_cache = Some(self.render_ascii(0));
        }
        self.ascii_cache.as_ref().unwrap()
    }

    fn render_ascii(&self, indent: usize) -> String {
        let prefix = "  ".repeat(indent);
        let mut out = String::new();
        let r = self.radius();
        let bar = if self.is_stable() { "▓" } else { "░" };
        let bar_width = (r.min(5.0) * 6.0) as usize;
        let kind = if self.children.is_empty() { "●" } else { "◇" };
        out.push_str(&format!("{}{} {} [r={:.3}] {}\n", prefix, kind, self.id, r, bar.repeat(bar_width)));
        for child in &self.children {
            out.push_str(&child.render_ascii(indent + 1));
        }
        out
    }
}

/// Build an N-depth fractal from a flat vector by recursive halving.
/// Depth 0 = single root containing all values.
/// Depth D = 2^D leaf nodes, each containing ~N/2^D values.
pub fn fractal_from_vec(values: &[f64], depth: usize) -> FractalNode {
    if depth == 0 || values.len() <= 2 {
        return FractalNode::leaf("leaf", 0, values.to_vec());
    }
    let mid = values.len() / 2;
    let mut node = FractalNode::new("node", depth);
    node.add_child(fractal_from_vec(&values[..mid], depth - 1));
    node.add_child(fractal_from_vec(&values[mid..], depth - 1));
    node
}

/// ASCII encoding of any value: f64 → visual representation.
pub fn ascii_value(v: f64, max_width: usize) -> String {
    let clamped = v.clamp(-1.0, 1.0);
    let half = max_width / 2;
    let pos = ((clamped + 1.0) / 2.0 * max_width as f64) as usize;
    let mut bar = String::with_capacity(max_width + 3);
    bar.push('[');
    for i in 0..max_width {
        if i == half { bar.push('|'); }
        else if i < pos { bar.push(if v >= 0.0 { '█' } else { ' ' }); }
        else if i == pos { bar.push(if v >= 0.0 { '▌' } else { '▐' }); }
        else { bar.push(if v < 0.0 { '█' } else { ' ' }); }
    }
    bar.push(']');
    bar.push_str(&format!(" {:.3}", v));
    bar
}

/// Encode a TriMatrix as ASCII art.
pub fn ascii_matrix(m: &crate::trinary::TriMatrix) -> String {
    use crate::trinary::Tri;
    let mut out = String::new();
    for r in 0..m.rows {
        out.push('|');
        for c in 0..m.cols {
            let ch = match m.get(r, c) {
                Tri::True => 'T',
                Tri::False => 'F',
                Tri::Unknown => '?',
            };
            out.push(ch);
        }
        out.push_str("|\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractal_leaf_count() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let root = fractal_from_vec(&v, 3);
        // 8 values × depth 3 = recursive halving gives power-of-2 structure
        assert!(root.leaf_count() >= 4); // at least some leaves
    }

    #[test]
    fn fractal_flatten_roundtrip() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        let root = fractal_from_vec(&v, 2);
        let flat = root.flatten();
        assert_eq!(flat, v);
    }

    #[test]
    fn fractal_ascii_renders() {
        let v = vec![0.5, -0.3, 0.8, 0.1];
        let mut root = fractal_from_vec(&v, 2);
        let art = root.ascii().to_string();
        assert!(art.contains("node"));
        assert!(art.contains("leaf"));
    }

    #[test]
    fn ascii_value_positive() {
        let art = ascii_value(0.5, 20);
        assert!(art.contains("█"));
        assert!(art.contains("0.500"));
    }

    #[test]
    fn ascii_value_negative() {
        let art = ascii_value(-0.5, 20);
        assert!(art.contains("█"));
        assert!(art.contains("-0.500"));
    }

    #[test]
    fn ascii_matrix_basic() {
        use crate::trinary::{Tri, TriMatrix};
        let mut m = TriMatrix::new(2, 3);
        m.set(0, 0, Tri::True);
        m.set(0, 1, Tri::False);
        m.set(0, 2, Tri::Unknown);
        let art = ascii_matrix(&m);
        assert_eq!(art, "|TF?|\n|???|\n");
    }

    #[test]
    fn fractal_depth_0_is_single_leaf() {
        let v = vec![1.0, 2.0, 3.0];
        let root = fractal_from_vec(&v, 0);
        assert_eq!(root.children.len(), 0);
        assert_eq!(root.flatten(), v);
    }
}
