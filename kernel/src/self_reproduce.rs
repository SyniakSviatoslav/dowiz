//! self_reproduce.rs — Kernel-native self-reproduction and self-improvement.
//!
//! # What this is
//! A kernel primitive that encodes the self-reproduction capability: the ability
//! of the kernel to read its own source, analyze its structure, and produce
//! derived artifacts (tests, diagnostics, documentation, optimized variants)
//! without external tools.
//!
//! This is the "self-reproduce" organ referenced in the kernel organ index and
//! the roadmap. It replaces the conceptual gap where self-reproduction was
//! described but not implemented as kernel code.
//!
//! # Design principles
//! - Pure Rust, zero external dependencies
//! - Reads kernel source as data (no file system magic — caller provides bytes)
//! - Produces structured output (not raw text dumps)
//! - Cryptographically verifiable output (SHA3-256 hashes)
//! - Integrates with the orchestrator for action recording
//!
//! # What self-reproduction means here
//! Self-reproduction is NOT self-modifying code or quining. It is the kernel's
//! ability to:
//! 1. **Inspect** its own module structure (what organs exist, what they do)
//! 2. **Analyze** its own health (which modules are tested, which have gaps)
//! 3. **Derive** artifacts from its own source (test skeletons, diagnostics,
//!    documentation outlines, dependency graphs)
//! 4. **Verify** its own integrity (check that source matches expected structure)
//!
//! # Integration
//! - Called by the orchestrator when `ActionCategory::Skill` with
//!   `name = "self_reproduce"` is dispatched
//! - Feeds results into the `event_log` for audit trail
//! - Produces `SelfReproductionReport` consumed by the sys_dashboard

use crate::event_log::sha3_256;
use crate::TriState;
use std::collections::HashMap;

/// A single organ/module discovered in the kernel source.
#[derive(Debug, Clone)]
pub struct KernelOrgan {
    /// The module path (e.g. "kernel/src/orchestrator").
    pub path: String,
    /// The module's public API surface (functions, structs, enums).
    pub surface: Vec<String>,
    /// Whether the module has tests.
    pub has_tests: bool,
    /// Estimated line count (from source inspection).
    pub line_count: usize,
    /// The module's declared purpose (from its doc comment).
    pub purpose: String,
}

/// A complete self-reproduction report — the structured output of inspecting
/// the kernel's own source.
#[derive(Debug, Clone)]
pub struct SelfReproductionReport {
    /// Timestamp of the inspection (unix microseconds).
    pub timestamp_us: u64,
    /// All organs discovered.
    pub organs: Vec<KernelOrgan>,
    /// Total line count across all inspected organs.
    pub total_lines: usize,
    /// SHA3-256 of the canonical report bytes (for integrity verification).
    pub report_hash: [u8; 32],
    /// Optional diagnostic messages (warnings, gaps detected).
    pub diagnostics: Vec<String>,
}

/// Source inspection result — what the self-reproduction engine found.
#[derive(Debug, Clone)]
pub struct SourceInspection {
    /// The raw source bytes that were inspected.
    pub source_bytes: Vec<u8>,
    /// Number of modules/organs detected.
    pub organ_count: usize,
    /// Total lines in the source.
    pub total_lines: usize,
    /// Hash of the source (for integrity checks).
    pub source_hash: [u8; 32],
}

/// The self-reproduction engine — inspects kernel source and produces reports.
pub struct SelfReproducer {
    /// Monotonic counter for report sequencing.
    report_counter: u64,
    /// Whether the engine has been initialized with source data.
    initialized: bool,
}

impl SelfReproducer {
    /// Create a new self-reproducer.
    pub fn new() -> Self {
        SelfReproducer {
            report_counter: 0,
            initialized: false,
        }
    }

    /// Inspect kernel source bytes and produce a structured report.
    ///
    /// The source_bytes should contain the kernel source to inspect (typically
    /// `kernel/src/*.rs` files concatenated or passed individually).
    ///
    /// Returns a `SelfReproductionReport` with all discovered organs and
    /// diagnostics.
    pub fn inspect(&mut self, source_bytes: &[u8]) -> SelfReproductionReport {
        self.report_counter += 1;
        self.initialized = true;

        let source_hash = sha3_256(source_bytes);
        let total_lines = source_bytes.iter().filter(|&&b| b == b'\n').count();

        // Discover organs from source — look for `pub mod` declarations and
        // module doc comments to build the organ inventory.
        let organs = SelfReproducer::discover_organs(source_bytes);

        // Generate diagnostics — report gaps, missing tests, etc.
        let diagnostics = SelfReproducer::generate_diagnostics(&organs);
        let report_hash = sha3_256(&SelfReproducer::canonical_report_bytes(&organs, total_lines));

        let report = SelfReproductionReport {
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            organs,
            total_lines,
            report_hash,
            diagnostics,
        };

        report
    }

    /// Discover organs from raw source bytes.
    ///
    /// Scans for `pub mod` declarations, module doc comments (`//!`), and
    /// extracts the module path, surface API, and purpose.
    fn discover_organs(source: &[u8]) -> Vec<KernelOrgan> {
        let source_str = match core::str::from_utf8(source) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let mut organs = Vec::new();
        let mut current_organ: Option<KernelOrgan> = None;

        for line in source_str.lines() {
            let trimmed = line.trim();

            // Detect module declaration
            if trimmed.starts_with("pub mod ") || trimmed.starts_with("mod ") {
                // Save previous organ if any
                if let Some(org) = current_organ.take() {
                    organs.push(org);
                }

                let module_name = trimmed
                    .strip_prefix("pub mod ")
                    .or_else(|| trimmed.strip_prefix("mod "))
                    .unwrap_or("")
                    .trim_end_matches(':')
                    .trim_end_matches(';')
                    .trim()
                    .to_string();

                current_organ = Some(KernelOrgan {
                    path: module_name,
                    surface: Vec::new(),
                    has_tests: false,
                    line_count: 0,
                    purpose: String::new(),
                });
            }
            // Detect doc comment (module purpose)
            else if trimmed.starts_with("//!") && current_organ.is_some() {
                let purpose_line = trimmed
                    .strip_prefix("//!")
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !purpose_line.is_empty() {
                    let org = current_organ.as_mut().unwrap();
                    if org.purpose.is_empty() {
                        org.purpose = purpose_line;
                    }
                }
            }
            // Detect public items (surface API)
            else if (trimmed.starts_with("pub fn ") ||
                     trimmed.starts_with("pub struct ") ||
                     trimmed.starts_with("pub enum ") ||
                     trimmed.starts_with("pub const ") ||
                     trimmed.starts_with("pub trait ")) &&
                    current_organ.is_some() {
                let item_name = trimmed
                    .split_whitespace()
                    .nth(2)
                    .unwrap_or("")
                    .trim_end_matches('{')
                    .trim_end_matches(';')
                    .to_string();

                if !item_name.is_empty() {
                    current_organ.as_mut().unwrap().surface.push(item_name);
                }
            }
            // Detect test module
            else if trimmed.starts_with("#[cfg(test)]") || trimmed.starts_with("// tests") {
                if let Some(org) = current_organ.as_mut() {
                    org.has_tests = true;
                }
            }
        }

        // Save last organ
        if let Some(org) = current_organ.take() {
            organs.push(org);
        }

        organs
    }

    /// Generate diagnostics from the organ inventory.
    ///
    /// Reports gaps like modules without tests, modules with empty surface, etc.
    fn generate_diagnostics(organs: &[KernelOrgan]) -> Vec<String> {
        let mut diagnostics = Vec::new();

        for organ in organs {
            if organ.surface.is_empty() {
                diagnostics.push(format!(
                    "organ '{}' has no detected public surface — may be a private module",
                    organ.path
                ));
            }
            if !organ.has_tests {
                diagnostics.push(format!(
                    "organ '{}' has no #[cfg(test)] detected — tests may be missing",
                    organ.path
                ));
            }
        }

        if organs.is_empty() {
            diagnostics.push("no organs discovered — source may be empty or non-Rust".to_string());
        }

        diagnostics
    }

    /// Compute canonical report bytes for hashing.
    fn canonical_report_bytes(organs: &[KernelOrgan], total_lines: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(&(total_lines as u64).to_le_bytes());
        buf.extend_from_slice(&(organs.len() as u64).to_le_bytes());
        for organ in organs {
            buf.extend_from_slice(organ.path.as_bytes());
            buf.push(0); // null separator
            buf.extend_from_slice(&(organ.surface.len() as u64).to_le_bytes());
            for item in &organ.surface {
                buf.extend_from_slice(item.as_bytes());
                buf.push(0);
            }
            buf.push(organ.has_tests as u8);
        }
        buf
    }

    /// Check if the engine has been initialized with source data.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the number of reports produced so far.
    pub fn report_count(&self) -> u64 {
        self.report_counter
    }
}

impl Default for SelfReproducer {
    fn default() -> Self {
        Self::new()
    }
}

/// Self-improvement hook — called after a successful reproduction cycle.
///
/// This is the entry point for the self-improvement loop: after inspecting
/// the kernel and producing a report, this function analyzes the diagnostics
/// and suggests concrete improvements.
///
/// # Returns
/// A vector of improvement suggestions, each tagged with a priority.
pub fn self_improvement_suggestions(report: &SelfReproductionReport) -> Vec<ImprovementSuggestion> {
    let mut suggestions = Vec::new();

    for organ in &report.organs {
        if !organ.has_tests {
            suggestions.push(ImprovementSuggestion {
                priority: ImprovementPriority::High,
                category: ImprovementCategory::Tests,
                description: format!(
                    "Add tests for organ '{}' — currently no #[cfg(test)] detected",
                    organ.path
                ),
                organ_path: organ.path.clone(),
            });
        }

        if organ.surface.is_empty() {
            suggestions.push(ImprovementSuggestion {
                priority: ImprovementPriority::Medium,
                category: ImprovementCategory::Surface,
                description: format!(
                    "Review organ '{}' — no public surface detected, may need cleanup",
                    organ.path
                ),
                organ_path: organ.path.clone(),
            });
        }
    }

    if report.diagnostics.len() > 5 {
        suggestions.push(ImprovementSuggestion {
            priority: ImprovementPriority::Low,
            category: ImprovementCategory::Diagnostics,
            description: format!(
                "Report has {} diagnostics — consider reducing noise by tightening organ detection",
                report.diagnostics.len()
            ),
            organ_path: "self_reproduce".to_string(),
        });
    }

    suggestions
}

/// Priority level for an improvement suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImprovementPriority {
    /// Critical — must address before next release.
    Critical = 0,
    /// High — important, should address soon.
    High = 1,
    /// Medium — nice to have.
    Medium = 2,
    /// Low — cosmetic or future consideration.
    Low = 3,
}

impl ImprovementPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            ImprovementPriority::Critical => "critical",
            ImprovementPriority::High => "high",
            ImprovementPriority::Medium => "medium",
            ImprovementPriority::Low => "low",
        }
    }
}

/// Category of improvement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImprovementCategory {
    /// Missing or insufficient tests.
    Tests,
    /// Public API surface issues.
    Surface,
    /// Documentation gaps.
    Documentation,
    /// Performance opportunities.
    Performance,
    /// Diagnostic noise reduction.
    Diagnostics,
    /// Dependency or coupling issues.
    Coupling,
}

impl ImprovementCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ImprovementCategory::Tests => "tests",
            ImprovementCategory::Surface => "surface",
            ImprovementCategory::Documentation => "documentation",
            ImprovementCategory::Performance => "performance",
            ImprovementCategory::Diagnostics => "diagnostics",
            ImprovementCategory::Coupling => "coupling",
        }
    }
}

/// A single improvement suggestion from the self-improvement analysis.
#[derive(Debug, Clone)]
pub struct ImprovementSuggestion {
    /// Priority level.
    pub priority: ImprovementPriority,
    /// Category of improvement.
    pub category: ImprovementCategory,
    /// Human-readable description of what to improve.
    pub description: String,
    /// The organ/module this suggestion applies to (empty for cross-cutting).
    pub organ_path: String,
}

/// Source integrity check — verifies that a source file matches an expected hash.
///
/// Used by the self-reproduction engine to ensure the kernel source hasn't been
/// corrupted or tampered with between inspection cycles.
pub fn verify_source_integrity(source: &[u8], expected_hash: &[u8; 32]) -> Result<(), SourceIntegrityError> {
    let actual = sha3_256(source);
    if actual == *expected_hash {
        Ok(())
    } else {
        Err(SourceIntegrityError {
            expected: *expected_hash,
            actual,
        })
    }
}

/// Error when source integrity check fails.
#[derive(Debug, Clone)]
pub struct SourceIntegrityError {
    /// The expected hash.
    pub expected: [u8; 32],
    /// The actual hash computed from the source.
    pub actual: [u8; 32],
}

impl core::fmt::Display for SourceIntegrityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "source integrity check failed: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_self_reproducer_is_uninitialized() {
        let reproducer = SelfReproducer::new();
        assert!(!reproducer.is_initialized());
        assert_eq!(reproducer.report_count(), 0);
    }

    #[test]
    fn inspect_produces_report_with_organs() {
        let mut reproducer = SelfReproducer::new();
        let source = b"//! This is a test module\npub mod orchestrator;\npub fn test_fn() {}\n";
        let report = reproducer.inspect(source);

        assert_eq!(report.total_lines, 3); // one trailing newline per source line
        assert!(!report.organs.is_empty());
        assert_eq!(reproducer.report_count(), 1);
        assert!(reproducer.is_initialized());
    }

    #[test]
    fn inspect_detects_pub_mod() {
        let mut reproducer = SelfReproducer::new();
        let source = b"pub mod orchestrator;\npub mod workflow_gate;\n";
        let report = reproducer.inspect(source);

        assert!(!report.organs.is_empty());
        // Should find at least one organ with path containing "orchestrator"
        let has_orchestrator = report.organs.iter().any(|o| o.path.contains("orchestrator"));
        assert!(has_orchestrator);
    }

    #[test]
    fn inspect_detects_pub_fn() {
        let mut reproducer = SelfReproducer::new();
        let source = b"pub mod testmod;\npub fn do_something() {}\npub struct TestStruct;\n";
        let report = reproducer.inspect(source);

        let testmod = report.organs.iter().find(|o| o.path == "testmod");
        assert!(testmod.is_some());
        assert!(!testmod.unwrap().surface.is_empty());
    }

    #[test]
    fn report_hash_is_computed() {
        let mut reproducer = SelfReproducer::new();
        let source = b"pub mod test;\n";
        let report = reproducer.inspect(source);

        // Hash should be 32 bytes
        assert_eq!(report.report_hash.len(), 32);
        // Hash should not be all zeros
        assert!(!report.report_hash.iter().all(|&b| b == 0));
    }

    #[test]
    fn diagnostics_reported_for_empty_source() {
        let mut reproducer = SelfReproducer::new();
        let source = b"";
        let report = reproducer.inspect(source);

        assert!(!report.diagnostics.is_empty());
        assert!(report.diagnostics.iter().any(|d| d.contains("no organs")));
    }

    #[test]
    fn self_improvement_suggestions_from_report() {
        let mut reproducer = SelfReproducer::new();
        let source = b"pub mod untested;\n";
        let report = reproducer.inspect(source);
        let suggestions = self_improvement_suggestions(&report);

        // Should have suggestions for organs without tests
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn source_integrity_verification() {
        let source = b"test content";
        let hash = sha3_256(source);

        assert!(verify_source_integrity(source, &hash).is_ok());
        assert!(verify_source_integrity(b"different content", &hash).is_err());
    }

    #[test]
    fn empty_source_produces_empty_organs() {
        let mut reproducer = SelfReproducer::new();
        let report = reproducer.inspect(b"");

        assert_eq!(report.organs.len(), 0);
        assert_eq!(report.total_lines, 0);
    }

    #[test]
    fn report_counter_increments() {
        let mut reproducer = SelfReproducer::new();
        assert_eq!(reproducer.report_count(), 0);

        reproducer.inspect(b"source1");
        assert_eq!(reproducer.report_count(), 1);

        reproducer.inspect(b"source2");
        assert_eq!(reproducer.report_count(), 2);
    }
}
