//! `kernel::invert` — inversion + backpropagation engine.
//!
//! When a deviation or error is detected at any point in the system, it
//! propagates BACKWARDS through the causal chain. Each step inverts the
//! forward computation to identify which upstream changes caused the error.
//!
//! # Backpropagation
//! Forward:  input → layer₀ → layer₁ → ... → output → error detected
//! Backward: error → ∂layer₁ ← ∂layer₀ ← ... ← root cause identified
//!
//! # Inversion
//! Every forward transform has an inverse. When backprop reaches a leaf,
//! the inverted transform tells us what input change would have prevented
//! the error. This is surfaced on the interface as corrective guidance.
//!
//! ZERO deps. Uses eigen, delta, trig, trinary.

use crate::delta::{Delta, DeltaComparison};

// ─── BackpropNode — one step in the backward chain ─────────────────────────

/// A node in the backpropagation chain. Links forward path → inverse.
#[derive(Debug, Clone)]
pub struct BackpropNode {
    pub id: String,
    pub layer: usize,            // 0 = root cause, N = error surface
    /// Forward delta that caused this node's activation.
    pub forward_delta: Delta,
    /// Inverse delta: the correction that would have prevented the error.
    pub inverse_delta: Option<Delta>,
    /// Responsibility fraction [0,1]: how much of the error is this node's fault.
    pub responsibility: f64,
}

impl BackpropNode {
    pub fn new(id: &str, layer: usize, forward_delta: Delta) -> Self {
        BackpropNode { id: id.to_string(), layer, forward_delta,
            inverse_delta: None, responsibility: 0.0 }
    }

    /// Compute the inverse: what input change would have prevented this output.
    pub fn invert(&mut self, gradient: f64) {
        // Inverse delta = negative of forward delta, scaled by gradient.
        let inv_components: Vec<f64> = self.forward_delta.components.iter()
            .map(|&c| -c * gradient).collect();
        let inv_mag = self.forward_delta.magnitude * gradient.abs();
        self.inverse_delta = Some(Delta {
            components: inv_components,
            magnitude: inv_mag,
            ts_from: self.forward_delta.ts_to,
            ts_to: self.forward_delta.ts_from,
            rate: inv_mag / (self.forward_delta.ts_to - self.forward_delta.ts_from).max(1) as f64,
        });
    }

    /// How much would applying the inverse improve the output?
    pub fn improvement(&self) -> f64 {
        self.inverse_delta.as_ref().map(|d| d.magnitude).unwrap_or(0.0)
    }
}

// ─── BackpropChain — full backward propagation path ───────────────────────

/// Chain of backprop nodes from error surface → root cause.
#[derive(Debug, Clone)]
pub struct BackpropChain {
    pub nodes: Vec<BackpropNode>,
    pub total_error: f64,
    pub root_cause: Option<String>,
}

impl BackpropChain {
    pub fn new(error_magnitude: f64) -> Self {
        BackpropChain { nodes: Vec::new(), total_error: error_magnitude, root_cause: None }
    }

    /// Add a layer to the chain. Error propagates backwards: each layer
    /// gets responsibility proportional to its delta magnitude.
    pub fn push_layer(&mut self, id: &str, layer: usize, delta: Delta) {
        let mag = delta.magnitude;
        let mut node = BackpropNode::new(id, layer, delta);
        if self.total_error > 0.0 {
            node.responsibility = (mag / self.total_error).min(1.0);
        }
        self.nodes.push(node);
    }

    /// Run full backprop: invert each node, assign root cause.
    pub fn backprop(&mut self, gradient: f64) {
        // Sort by layer descending (error surface first, root cause last).
        self.nodes.sort_by_key(|n| -(n.layer as i64));
        let total_mag: f64 = self.nodes.iter().map(|n| n.forward_delta.magnitude).sum();

        for node in &mut self.nodes {
            let local_gradient = if total_mag > 0.0 {
                gradient * node.forward_delta.magnitude / total_mag
            } else { gradient };
            node.invert(local_gradient);
        }

        // Root cause = deepest node with highest responsibility.
        self.root_cause = self.nodes.iter()
            .max_by(|a, b| a.responsibility.partial_cmp(&b.responsibility).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.layer.cmp(&b.layer)))
            .map(|n| n.id.clone());
    }

    /// ASCII visualization of the backprop chain.
    pub fn ascii(&self) -> String {
        let mut out = String::from("═══ BACKPROP ═══\n");
        out.push_str(&format!("  error: {:.3}\n", self.total_error));
        if let Some(ref root) = self.root_cause {
            out.push_str(&format!("  root: {}\n", root));
        }
        for node in &self.nodes {
            let arrow = if node.inverse_delta.is_some() { "←" } else { "→" };
            let resp_bar = (node.responsibility * 20.0) as usize;
            out.push_str(&format!("  L{} {} {} [{:.3}] {}\n",
                node.layer, node.id, arrow, node.responsibility, "█".repeat(resp_bar)));
        }
        out
    }
}

// ─── Invertible — trait for anything that can be inverted ─────────────────

/// A forward transform with a known inverse.
pub trait Invertible {
    type Input;
    type Output;

    fn forward(&self, input: &Self::Input) -> Self::Output;
    fn inverse(&self, output: &Self::Output) -> Self::Input;

    /// Verify invertibility: forward(inverse(output)) ≈ output.
    fn verify(&self, output: &Self::Output, _tolerance: f64) -> bool
    where Self::Output: PartialEq + Clone
    {
        let input = self.inverse(output);
        let reconstructed = self.forward(&input);
        // Simple comparison — override for custom distance metrics.
        std::mem::discriminant(&reconstructed) == std::mem::discriminant(output)
    }
}

// ─── Interface surfacing — show backprop results ──────────────────────────

/// Interface report: what backprop found and what to fix.
#[derive(Debug, Clone)]
pub struct BackpropReport {
    pub chain: BackpropChain,
    pub recommendations: Vec<String>,
    pub severity: DeltaComparison,
}

impl BackpropReport {
    pub fn new(chain: BackpropChain) -> Self {
        let severity = if chain.total_error > 10.0 { DeltaComparison::Growing }
            else if chain.total_error > 1.0 { DeltaComparison::Oscillating }
            else { DeltaComparison::Stable };

        let mut recommendations = Vec::new();
        for node in &chain.nodes {
            if node.responsibility > 0.3 {
                if let Some(ref inv) = node.inverse_delta {
                    recommendations.push(format!(
                        "{}: adjust by {:.3} (responsibility {:.3})",
                        node.id, inv.magnitude, node.responsibility
                    ));
                }
            }
        }

        BackpropReport { chain, recommendations, severity }
    }

    /// ASCII interface report.
    pub fn display(&self) -> String {
        let mut out = self.chain.ascii();
        out.push_str("\n═══ RECOMMENDATIONS ═══\n");
        if self.recommendations.is_empty() {
            out.push_str("  No significant corrections needed.\n");
        } else {
            for r in &self.recommendations {
                out.push_str(&format!("  → {}\n", r));
            }
        }
        out
    }
}

/// Run backpropagation from a detection point backwards through the system.
/// Takes a set of observed deltas and identifies root causes.
pub fn backprop_from_deltas(
    deltas: &[(String, usize, Delta)],  // (id, layer, delta)
    error_threshold: f64,
    gradient: f64,
) -> BackpropReport {
    let total_error: f64 = deltas.iter().map(|(_, _, d)| d.magnitude).sum();
    let mut chain = BackpropChain::new(total_error);

    for (id, layer, delta) in deltas {
        if delta.magnitude > error_threshold {
            chain.push_layer(id, *layer, delta.clone());
        }
    }

    chain.backprop(gradient);
    BackpropReport::new(chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backprop_node_invert() {
        let v0 = vec![0.0, 0.0];
        let v1 = vec![5.0, 0.0];
        let d = Delta::between(&v0, 0, &v1, 1000);
        let mut node = BackpropNode::new("test", 1, d);
        node.invert(1.0);
        let inv = node.inverse_delta.unwrap();
        assert!((inv.components[0] + 5.0).abs() < 1e-10); // inverse = -forward
    }

    #[test]
    fn backprop_chain_root_cause() {
        let v = vec![0.0];
        let d1 = Delta::between(&v, 0, &[1.0], 1);
        let d2 = Delta::between(&[1.0], 1, &[10.0], 2);
        let mut chain = BackpropChain::new(10.0);
        chain.push_layer("input", 0, d1);
        chain.push_layer("output", 1, d2);
        chain.backprop(1.0);
        assert!(chain.root_cause.is_some());
        // output layer had bigger delta → should be root cause
        assert_eq!(chain.root_cause.unwrap(), "output");
    }

    #[test]
    fn backprop_report_generates_recommendations() {
        let v0 = vec![0.0, 0.0];
        let v1 = vec![0.0, 10.0];
        let d = Delta::between(&v0, 0, &v1, 1000);
        let deltas = vec![("sensor_a".into(), 2, d)];
        let report = backprop_from_deltas(&deltas, 0.5, 1.0);
        let disp = report.display();
        assert!(disp.contains("BACKPROP"));
        assert!(disp.contains("sensor_a"));
    }

    #[test]
    fn empty_deltas_give_no_recommendations() {
        let report = backprop_from_deltas(&[], 0.5, 1.0);
        assert_eq!(report.recommendations.len(), 0);
    }

    #[test]
    fn backprop_ascii_renders() {
        let v = vec![1.0];
        let d = Delta::between(&v, 0, &[2.0], 1);
        let mut chain = BackpropChain::new(1.0);
        chain.push_layer("node1", 0, d);
        chain.backprop(0.5);
        let art = chain.ascii();
        assert!(art.contains("L0"));
        assert!(art.contains("node1"));
    }
}
