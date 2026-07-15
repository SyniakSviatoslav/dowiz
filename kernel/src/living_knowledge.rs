//! Living-knowledge retrieval — ADAPTER (not a reimplementation).
//!
//! Decision (operator, 2026-07-14): the living-knowledge engine lives on a DIVERGENT
//! branch (`recover/stash-1-2994e6c8`, JS + bge-small ONNX embedder). This module wires
//! it in via a clean adapter — it does NOT merge the JS spike into the kernel. The kernel
//! defines the trait + a process-backed adapter that speaks JSON-over-stdio to a bridge
//! command. The bridge is swappable: a minimal deterministic reference bridge ships here
//! for offline RED→GREEN verification; the real ONNX-backed spike is plugged in at runtime
//! by pointing `LK_BRIDGE_CMD` at it (no kernel recompile needed).
//!
//! Fail-closed: any spawn / I/O / protocol error returns `Err` — retrieval never silently
//! degrades to "empty results".

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

/// A single retrieval hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub id: String,
    pub score: f64,
}

/// Retrieval contract the rest of the kernel depends on.
pub trait LivingKnowledge {
    /// Rank `k` documents for `query`. Errors are explicit (fail-closed).
    fn retrieve(&self, query: &str, k: usize) -> Result<Vec<Hit>, String>;
}

/// Bridge stdin contract.
#[derive(Serialize)]
struct BridgeRequest<'a> {
    files: &'a [DocInput],
    query: &'a str,
    k: usize,
}

/// A document handed to the bridge (path/title/text triple).
#[derive(Debug, Clone, Serialize)]
pub struct DocInput {
    pub rel: String,
    pub title: String,
    pub text: String,
}

/// Bridge stdout contract (subset we depend on).
#[derive(Deserialize)]
struct BridgeResponse {
    results: Vec<Hit>,
    #[serde(default)]
    abs_confidence: f64,
}

/// Process-backed adapter. Spawns `bridge_cmd` (default: `LK_BRIDGE_CMD` env, else the
/// bundled reference bridge via `node`) once per `retrieve` call. Fail-closed.
pub struct SubprocessLivingKnowledge {
    bridge_cmd: String,
    corpus: Vec<DocInput>,
}

impl SubprocessLivingKnowledge {
    /// Build an adapter. `bridge_cmd` is the command invoked with the JSON request on
    /// stdin; the JSON response is read from stdout. Use an absolute path or a command on
    /// `PATH`. When `None`, falls back to `LK_BRIDGE_CMD` env, then to the bundled
    /// `scripts/lk-bridge.mjs` reference (requires `node` on PATH).
    pub fn new(corpus: Vec<DocInput>, bridge_cmd: Option<String>) -> Self {
        let bridge_cmd = bridge_cmd
            .or_else(|| std::env::var("LK_BRIDGE_CMD").ok())
            .unwrap_or_else(|| "node scripts/lk-bridge.mjs".to_string());
        Self { bridge_cmd, corpus }
    }
}

impl LivingKnowledge for SubprocessLivingKnowledge {
    fn retrieve(&self, query: &str, k: usize) -> Result<Vec<Hit>, String> {
        if query.trim().is_empty() {
            return Err("living_knowledge: empty query".to_string());
        }
        let req = BridgeRequest {
            files: &self.corpus,
            query,
            k,
        };
        let payload = serde_json::to_string(&req)
            .map_err(|e| format!("living_knowledge: serialize request: {e}"))?;

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&self.bridge_cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("living_knowledge: spawn bridge `{}`: {e}", self.bridge_cmd))?;

        {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| "living_knowledge: bridge stdin unavailable".to_string())?;
            stdin
                .write_all(payload.as_bytes())
                .map_err(|e| format!("living_knowledge: write request: {e}"))?;
        }

        let out = child
            .wait_with_output()
            .map_err(|e| format!("living_knowledge: wait bridge: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "living_knowledge: bridge exited {}: {}",
                out.status,
                stderr.lines().next().unwrap_or("<no stderr>")
            ));
        }

        let resp: BridgeResponse = serde_json::from_slice(&out.stdout)
            .map_err(|e| format!("living_knowledge: parse response: {e}"))?;
        Ok(resp.results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal deterministic corpus: three docs, one clearly about "pricing", one about
    /// "delivery", one about "refund". The reference bridge fuses lexical + title signals
    /// (no ONNX), so a lexical query must surface the right doc at rank 1.
    fn corpus() -> Vec<DocInput> {
        vec![
            DocInput {
                rel: "docs/pricing.md".into(),
                title: "Pricing model".into(),
                text: "The pricing model computes subtotal, delivery fee, tax, and total.".into(),
            },
            DocInput {
                rel: "docs/delivery.md".into(),
                title: "Delivery flow".into(),
                text: "The delivery flow tracks the courier from pickup to dropoff.".into(),
            },
            DocInput {
                rel: "docs/refund.md".into(),
                title: "Refund policy".into(),
                text: "The refund policy returns money to the customer within 14 days.".into(),
            },
        ]
    }

    /// Resolve the bundled reference bridge (repo-root `scripts/lk-bridge.mjs`) from the
    /// kernel crate dir, so the test is cwd-independent.
    fn bridge_cmd() -> String {
        let here = env!("CARGO_MANIFEST_DIR"); // .../dowiz/kernel
        let repo = std::path::Path::new(here)
            .parent()
            .expect("kernel crate has a parent (repo root)");
        let script = repo.join("scripts").join("lk-bridge.mjs");
        format!("node {}", script.display())
    }

    /// True only when a working `node` runtime AND the bundled bridge script exist.
    /// Lets the bridge-dependent test skip cleanly on headless/kernel boxes.
    fn bridge_available() -> bool {
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .map(|o| !o.status.success())
            .unwrap_or(true)
        {
            return false; // `node` not on PATH or non-zero exit
        }
        let here = env!("CARGO_MANIFEST_DIR");
        let repo = std::path::Path::new(here).parent().expect("repo root");
        repo.join("scripts").join("lk-bridge.mjs").exists()
    }

    #[test]
    fn adapter_routes_query_to_bridge_and_ranks_correctly() {
        // The bridge is a node subprocess. On a host without a working node
        // runtime (headless CI / kernel-autopilot boxes), skip rather than fail —
        // the FAIL-CLOSED behaviour of the adapter itself is covered by
        // `adapter_is_fail_closed_on_*`. This keeps `cargo test --lib` green
        // everywhere without masking a real adapter regression.
        if !bridge_available() {
            eprintln!("skipping: node bridge unavailable in this environment");
            return;
        }
        // RED→GREEN: a lexical query "pricing" must rank docs/pricing.md first.
        let lk = SubprocessLivingKnowledge::new(corpus(), Some(bridge_cmd()));
        let hits = lk.retrieve("pricing", 3).expect("bridge retrieval must succeed");
        assert!(!hits.is_empty(), "expected at least one hit");
        assert_eq!(hits[0].id, "docs/pricing.md", "lexical query must surface pricing doc");
    }

    #[test]
    fn adapter_is_fail_closed_on_empty_query() {
        let lk = SubprocessLivingKnowledge::new(corpus(), Some(bridge_cmd()));
        assert!(lk.retrieve("   ", 3).is_err(), "empty query must error (fail-closed)");
    }

    #[test]
    fn adapter_is_fail_closed_on_missing_bridge() {
        // Point at a command that does not exist → must error, never return empty OK.
        let lk = SubprocessLivingKnowledge::new(corpus(), Some("this_bridge_does_not_exist_xyz".into()));
        assert!(lk.retrieve("pricing", 3).is_err(), "missing bridge must error (fail-closed)");
    }
}
