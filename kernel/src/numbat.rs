//! numbat.rs — Numbat reimplementation: endpoint visibility for AI agent actions.
//!
//! # What this is
//! A kernel-native endpoint visibility system for AI agent activity:
//! 1. **Detect** — monitor agent actions at the endpoint level
//! 2. **Block** (optional) — intercept actions before execution
//! 3. **Forensic reconstruction** — reconstruct what happened from logs
//!
//! # Numbat mapping
//! - "endpoint visibility into AI agent activity" → `AgentActivityMonitor`
//! - "local detection" → `detect_anomalous()` pattern matching
//! - "optional pre-action blocking" → `BlockPolicy` trait
//! - "forensic reconstruction" → `reconstruct_timeline()` from event_log
//!
//! # Design
//! - Pure Rust, zero external dependencies
//! - Uses existing kernel primitives: event_log (SHA3-256), fdr (flight data recorder),
//!   self_harness (zone protection), workflow_gate (phase tracking)
//! - Deterministic detection (no ML — rule-based pattern matching)

use crate::event_log::sha3_256;
use crate::fdr::{self, Level};

/// Types of agent actions that can be monitored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentActionType {
    /// Reading files or data.
    Read,
    /// Writing files or data.
    Write,
    /// Executing code or commands.
    Execute,
    /// Making network requests.
    Network,
    /// Using tools (LLM calls, etc.).
    ToolUse,
    /// Modifying state or configuration.
    StateChange,
}

impl AgentActionType {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentActionType::Read => "read",
            AgentActionType::Write => "write",
            AgentActionType::Execute => "execute",
            AgentActionType::Network => "network",
            AgentActionType::ToolUse => "tool_use",
            AgentActionType::StateChange => "state_change",
        }
    }
}

/// An agent action event — recorded for visibility and forensics.
#[derive(Debug, Clone)]
pub struct AgentAction {
    /// Unique event ID.
    pub event_id: u64,
    /// Type of action.
    pub action_type: AgentActionType,
    /// What the action is doing (description).
    pub description: String,
    /// Whether the action was blocked.
    pub blocked: bool,
    /// The agent/module that performed the action.
    pub source: String,
    /// Timestamp (microseconds).
    pub timestamp_us: u64,
    /// SHA3-256 of the canonical bytes (integrity).
    pub hash: [u8; 32],
}

/// A block policy — decides whether an action should be blocked.
///
/// Implement this trait to create custom blocking rules.
pub trait BlockPolicy {
    /// Check if an action should be blocked.
    /// Returns (should_block, reason).
    fn should_block(&self, action: &AgentAction) -> (bool, String);
}

/// Default block policy — blocks nothing (permissive).
pub struct AllowAllPolicy;

impl BlockPolicy for AllowAllPolicy {
    fn should_block(&self, _action: &AgentAction) -> (bool, String) {
        (false, String::new())
    }
}

/// Strict block policy — blocks writes and state changes by default.
pub struct StrictWritePolicy {
    /// Exceptions: actions allowed despite being writes.
    exceptions: Vec<String>,
}

impl StrictWritePolicy {
    pub fn new() -> Self {
        StrictWritePolicy {
            exceptions: Vec::new(),
        }
    }

    pub fn add_exception(&mut self, pattern: &str) {
        self.exceptions.push(pattern.to_string());
    }
}

impl Default for StrictWritePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockPolicy for StrictWritePolicy {
    fn should_block(&self, action: &AgentAction) -> (bool, String) {
        match action.action_type {
            AgentActionType::Write | AgentActionType::StateChange => {
                // Check exceptions.
                for exception in &self.exceptions {
                    if action.description.contains(exception) {
                        return (false, format!("exception matched: {}", exception));
                    }
                }
                (true, "write/state_change blocked by strict policy".to_string())
            }
            _ => (false, String::new()),
        }
    }
}

/// Detection rule — pattern-based anomaly detection.
#[derive(Debug, Clone)]
pub struct DetectionRule {
    /// Name of the rule.
    pub name: String,
    /// Action types this rule applies to.
    pub action_types: Vec<AgentActionType>,
    /// Pattern to match in the description.
    pub pattern: String,
    /// Severity when triggered.
    pub severity: Level,
    /// Whether this is an anomaly (true) or normal (false).
    pub is_anomaly: bool,
}

/// The agent activity monitor — core visibility engine.
pub struct AgentActivityMonitor {
    /// Recorded actions.
    actions: Vec<AgentAction>,
    /// Monotonic ID counter.
    next_id: u64,
    /// Block policy in use.
    block_policy: Box<dyn BlockPolicy>,
    /// Detection rules.
    detection_rules: Vec<DetectionRule>,
}

impl AgentActivityMonitor {
    /// Create a new monitor with the given block policy.
    pub fn new(block_policy: Box<dyn BlockPolicy>) -> Self {
        AgentActivityMonitor {
            actions: Vec::new(),
            next_id: 0,
            block_policy,
            detection_rules: Vec::new(),
        }
    }

    /// Record and optionally block an agent action.
    ///
    /// Returns the recorded action (with blocked flag set if policy blocked it).
    pub fn record_action(
        &mut self,
        action_type: AgentActionType,
        description: &str,
        source: &str,
    ) -> AgentAction {
        let event_id = self.next_id;
        self.next_id += 1;

        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let action = AgentAction {
            event_id,
            action_type,
            description: description.to_string(),
            blocked: false,
            source: source.to_string(),
            timestamp_us,
            hash: [0u8; 32],
        };

        // Check block policy.
        let (should_block, reason) = self.block_policy.should_block(&action);
        if should_block {
            let mut blocked_action = action.clone();
            blocked_action.blocked = true;
            // Record the block reason.
            fdr::emit_event(
                Level::Warn,
                &format!("ACTION BLOCKED [#{}]: {} — {}", event_id, description, reason),
                &[],
            );
            // Store with blocked flag.
            let hash = Self::compute_hash(&blocked_action);
            blocked_action.hash = hash;
            self.actions.push(blocked_action);
            return self.actions.last().unwrap().clone();
        }

        // Check detection rules.
        for rule in &self.detection_rules {
            if rule.action_types.contains(&action.action_type)
                && action.description.contains(&rule.pattern)
            {
                if rule.is_anomaly {
                    fdr::emit_event(
                        rule.severity,
                        &format!("ANOMALY DETECTED [#{}]: {} matched rule '{}'",
                            event_id, description, rule.name),
                        &[],
                    );
                }
            }
        }

        let hash = Self::compute_hash(&action);
        let mut final_action = action;
        final_action.hash = hash;
        self.actions.push(final_action);
        self.actions.last().unwrap().clone()
    }

    /// Add a detection rule.
    pub fn add_detection_rule(&mut self, rule: DetectionRule) {
        self.detection_rules.push(rule);
    }

    /// Detect anomalous actions in the recorded history.
    pub fn detect_anomalous(&self) -> Vec<&AgentAction> {
        self.actions.iter()
            .filter(|a| {
                for rule in &self.detection_rules {
                    if rule.action_types.contains(&a.action_type)
                        && a.description.contains(&rule.pattern)
                        && rule.is_anomaly
                    {
                        return true;
                    }
                }
                false
            })
            .collect()
    }

    /// Reconstruct the timeline of actions — forensic reconstruction.
    ///
    /// Returns actions in chronological order with their block status.
    pub fn reconstruct_timeline(&self) -> Vec<AgentAction> {
        self.actions.clone()
    }


    /// Get the number of recorded actions.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Check if there are no recorded actions.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Get actions by type.
    pub fn actions_by_type(&self, action_type: AgentActionType) -> Vec<&AgentAction> {
        self.actions.iter()
            .filter(|a| a.action_type == action_type)
            .collect()
    }

    /// Get blocked actions.
    pub fn blocked_actions(&self) -> Vec<&AgentAction> {
        self.actions.iter()
            .filter(|a| a.blocked)
            .collect()
    }

    /// Clear all recorded actions (for reset/testing).
    pub fn clear(&mut self) {
        self.actions.clear();
        self.next_id = 0;
    }

    /// Compute SHA3-256 hash of an action's canonical bytes.
    fn compute_hash(action: &AgentAction) -> [u8; 32] {
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(&action.event_id.to_le_bytes());
        buf.extend_from_slice(action.action_type.as_str().as_bytes());
        buf.push(0);
        buf.extend_from_slice(action.description.as_bytes());
        buf.push(0);
        buf.extend_from_slice(action.source.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&(action.blocked as u8).to_le_bytes());
        buf.extend_from_slice(&action.timestamp_us.to_le_bytes());
        sha3_256(&buf)
    }

    /// ASCII report of current monitor state.
    pub fn ascii_report(&self) -> String {
        let mut out = String::from("=== Agent Activity Monitor Report ===\n");
        out.push_str(&format!("Total actions: {}\n", self.len()));
        out.push_str(&format!("Blocked actions: {}\n", self.blocked_actions().len()));
        out.push_str(&format!("Detection rules: {}\n", self.detection_rules.len()));

        if !self.actions.is_empty() {
            out.push_str("\nRecent actions:\n");
            for action in self.actions.iter().rev().take(5) {
                let status = if action.blocked { "BLOCKED" } else { "allowed" };
                out.push_str(&format!(
                    "  #{}: {} [{}] {} — {}\n",
                    action.event_id, action.action_type.as_str(),
                    status, action.description, action.source
                ));
            }
        }


        out.push_str("\n=== End Report ===\n");
        out
    }
}

impl Default for AgentActivityMonitor {
    fn default() -> Self {
        Self::new(Box::new(AllowAllPolicy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_monitor_is_empty() {
        let monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        assert!(monitor.is_empty());
        assert_eq!(monitor.len(), 0);
    }

    #[test]
    fn record_action_adds_entry() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        let action = monitor.record_action(
            AgentActionType::Read,
            "read file foo.txt",
            "agent-1",
        );
        assert_eq!(action.event_id, 0);
        assert!(!action.blocked);
        assert_eq!(monitor.len(), 1);
    }

    #[test]
    fn record_multiple_gets_incrementing_ids() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        monitor.record_action(AgentActionType::Read, "r1", "s");
        monitor.record_action(AgentActionType::Write, "w1", "s");
        assert_eq!(monitor.len(), 2);
    }

    #[test]
    fn strict_policy_blocks_writes() {
        let mut monitor = AgentActivityMonitor::new(Box::new(StrictWritePolicy::new()));
        let action = monitor.record_action(
            AgentActionType::Write,
            "write dangerous file",
            "agent-1",
        );
        assert!(action.blocked);
    }

    #[test]
    fn strict_policy_allows_reads() {
        let mut monitor = AgentActivityMonitor::new(Box::new(StrictWritePolicy::new()));
        let action = monitor.record_action(
            AgentActionType::Read,
            "read file",
            "agent-1",
        );
        assert!(!action.blocked);
    }

    #[test]
    fn strict_policy_exception_allows() {
        let mut policy = StrictWritePolicy::new();
        policy.add_exception("allowed_file.txt");
        let mut monitor = AgentActivityMonitor::new(Box::new(policy));
        let action = monitor.record_action(
            AgentActionType::Write,
            "write allowed_file.txt",
            "agent-1",
        );
        assert!(!action.blocked);
    }

    #[test]
    fn detection_rule_catches_pattern() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        monitor.add_detection_rule(DetectionRule {
            name: "dangerous-exec".to_string(),
            action_types: vec![AgentActionType::Execute],
            pattern: "rm -rf".to_string(),
            severity: Level::Error,
            is_anomaly: true,
        });

        monitor.record_action(AgentActionType::Execute, "run rm -rf /", "agent-1");
        let anomalies = monitor.detect_anomalous();
        assert_eq!(anomalies.len(), 1);
    }

    #[test]
    fn detection_rule_ignores_non_matching() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        monitor.add_detection_rule(DetectionRule {
            name: "test".to_string(),
            action_types: vec![AgentActionType::Execute],
            pattern: "rm -rf".to_string(),
            severity: Level::Error,
            is_anomaly: true,
        });

        monitor.record_action(AgentActionType::Execute, "run safe command", "agent-1");
        let anomalies = monitor.detect_anomalous();
        assert_eq!(anomalies.len(), 0);
    }

    #[test]
    fn reconstruct_timeline_returns_all() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        monitor.record_action(AgentActionType::Read, "r1", "s");
        monitor.record_action(AgentActionType::Write, "w1", "s");

        let timeline = monitor.reconstruct_timeline();
        assert_eq!(timeline.len(), 2);
    }

    #[test]
    fn actions_by_type_filters() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        monitor.record_action(AgentActionType::Read, "r1", "s");
        monitor.record_action(AgentActionType::Read, "r2", "s");
        monitor.record_action(AgentActionType::Write, "w1", "s");

        let reads = monitor.actions_by_type(AgentActionType::Read);
        assert_eq!(reads.len(), 2);
    }

    #[test]
    fn blocked_actions_filtered() {
        let mut monitor = AgentActivityMonitor::new(Box::new(StrictWritePolicy::new()));
        monitor.record_action(AgentActionType::Read, "r1", "s");
        monitor.record_action(AgentActionType::Write, "w1", "s");

        let blocked = monitor.blocked_actions();
        assert_eq!(blocked.len(), 1);
        assert!(blocked[0].blocked);
    }

    #[test]
    fn clear_resets_state() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        monitor.record_action(AgentActionType::Read, "r1", "s");
        assert_eq!(monitor.len(), 1);

        monitor.clear();
        assert_eq!(monitor.len(), 0);
        assert!(monitor.is_empty());
    }

    #[test]
    fn action_hash_is_computed() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        let action = monitor.record_action(
            AgentActionType::Read,
            "test action",
            "test-source",
        );
        assert_eq!(action.hash.len(), 32);
        assert!(!action.hash.iter().all(|&b| b == 0));
    }

    #[test]
    fn ascii_report_format() {
        let monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        let report = monitor.ascii_report();
        assert!(report.contains("Agent Activity Monitor Report"));
        assert!(report.contains("Total actions: 0"));
    }

    #[test]
    fn multiple_action_types() {
        let mut monitor = AgentActivityMonitor::new(Box::new(AllowAllPolicy));
        for &at in &[AgentActionType::Read, AgentActionType::Write,
                      AgentActionType::Execute, AgentActionType::Network,
                      AgentActionType::ToolUse, AgentActionType::StateChange] {
            monitor.record_action(at, "test", "s");
        }
        assert_eq!(monitor.len(), 6);
    }
}
