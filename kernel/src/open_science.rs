//! open_science.rs — Open Science Desktop reimplementation: AI research workbench.
//!
//! # What this is
//! A kernel-native research workbench: agents + notebooks + files + reports in
//! an auditable workflow. Maps to kernel primitives for paper discovery,
//! knowledge storage, skill extraction, and section-based search.
//!
//! # Open Science Desktop mapping
//! - "Research engine" → `ResearchEngine` (existing research.rs)
//! - "Knowledge storage" → `Academia` (existing academia.rs — 8D crystal lattice)
//! - "Skill extractor" → `SkillExtractor` (existing skill_extractor.rs)
//! - "Notebook organization" → `Notebook` + `MemorySearchEngine`
//! - "Auditable workflow" → `WorkbenchAuditLog` (SHA3-256 event log)
//!
//! # Design
//! - Pure Rust, zero external dependencies
//! - Orchestrates existing kernel modules into a workbench facade
//! - Deterministic, testable, auditable

use crate::academia::{Academia, QuarkSig};

/// A locally stored research artifact.
///
/// `Academia` deliberately stores only compact 8-byte signatures and returns
/// insertion indices from search.  The workbench owns the auditable metadata
/// and uses the same insertion order to resolve those indices.
#[derive(Debug, Clone)]
pub struct ResearchArtifact {
    /// Stable workbench ID (starts at one, so zero remains an invalid ID).
    pub id: u64,
    pub title: String,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub arxiv_id: String,
    /// Signature returned by the real `Academia::insert` API.
    pub quark_sig: QuarkSig,
}

/// A notebook entry — a unit of research work.
#[derive(Debug, Clone)]
pub struct NotebookEntry {
    /// Unique ID.
    pub id: u64,
    /// Title of the notebook entry.
    pub title: String,
    /// Content (markdown or structured text).
    pub content: String,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Related paper IDs (from academia).
    pub related_papers: Vec<u64>,
    /// Timestamp (microseconds).
    pub created_at_us: u64,
}

/// A report — assembled from notebook entries and research findings.
#[derive(Debug, Clone)]
pub struct Report {
    /// Report ID.
    pub id: u64,
    /// Title.
    pub title: String,
    /// Sections (each with a heading and content).
    pub sections: Vec<ReportSection>,
    /// References (paper IDs from academia).
    pub references: Vec<u64>,
    /// SHA3-256 integrity hash.
    pub hash: [u8; 32],
}

/// A section within a report.
#[derive(Debug, Clone)]
pub struct ReportSection {
    /// Section heading.
    pub heading: String,
    /// Section content.
    pub content: String,
}

/// The audit log entry — every action is recorded.
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    /// Entry ID (monotonically increasing).
    pub seq: u64,
    /// What action was performed.
    pub action: String,
    /// Target (paper ID, notebook ID, etc.).
    pub target: Option<u64>,
    /// Actor (who performed the action).
    pub actor: String,
    /// Timestamp (microseconds).
    pub timestamp_us: u64,
    /// SHA3-256 integrity hash.
    pub hash: [u8; 32],
}

/// The research workbench — orchestrates all components.
pub struct ResearchWorkbench {
    /// Compact Academia index for paper discovery.
    academia: Academia,
    /// Local-first metadata, in exactly the same insertion order as Academia.
    papers: Vec<ResearchArtifact>,
    /// Notebook entries.
    notebooks: Vec<NotebookEntry>,
    /// Reports.
    reports: Vec<Report>,
    /// Audit log.
    audit_log: Vec<AuditLogEntry>,
    /// Next IDs.
    next_notebook_id: u64,
    next_report_id: u64,
    next_audit_seq: u64,
    next_paper_id: u64,
}

impl ResearchWorkbench {
    /// Create a new research workbench.
    pub fn new() -> Self {
        ResearchWorkbench {
            academia: Academia::new(),
            papers: Vec::new(),
            notebooks: Vec::new(),
            reports: Vec::new(),
            audit_log: Vec::new(),
            next_notebook_id: 0,
            next_report_id: 0,
            next_audit_seq: 0,
            next_paper_id: 1,
        }
    }

    /// Record an audit entry.
    fn audit(&mut self, action: &str, target: Option<u64>, actor: &str) {
        let seq = self.next_audit_seq;
        self.next_audit_seq += 1;

        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&seq.to_le_bytes());
        buf.extend_from_slice(action.as_bytes());
        buf.extend_from_slice(&timestamp_us.to_le_bytes());
        let hash = crate::event_log::sha3_256(&buf);

        self.audit_log.push(AuditLogEntry {
            seq,
            action: action.to_string(),
            target,
            actor: actor.to_string(),
            timestamp_us,
            hash,
        });
    }

    // ─── Academia integration ────────────────────────────────────────────

    /// Add a paper to the academia knowledge base.
    pub fn add_paper(&mut self, title: &str, abstract_text: &str, categories: &[String], arxiv_id: &str) -> u64 {
        let id = self.next_paper_id;
        self.next_paper_id += 1;
        let quark_sig = self.academia.insert(title);
        self.papers.push(ResearchArtifact {
            id,
            title: title.to_string(),
            abstract_text: abstract_text.to_string(),
            categories: categories.to_vec(),
            arxiv_id: arxiv_id.to_string(),
            quark_sig,
        });
        self.audit("paper_added", Some(id), "workbench");
        id
    }

    /// Search local metadata by keyword, then supplement it with Academia's
    /// crystal-lattice neighbours.  Returned artifacts remain locally owned.
    pub fn search_papers(&self, keyword: &str) -> Vec<&ResearchArtifact> {
        let keyword = keyword.to_lowercase();
        let mut indices: Vec<usize> = self.papers.iter().enumerate()
            .filter(|(_, paper)| {
                paper.title.to_lowercase().contains(&keyword)
                    || paper.abstract_text.to_lowercase().contains(&keyword)
                    || paper.arxiv_id.to_lowercase().contains(&keyword)
                    || paper.categories.iter().any(|category| category.to_lowercase().contains(&keyword))
            })
            .map(|(index, _)| index)
            .take(10)
            .collect();

        if indices.len() < 10 {
            for (index, _) in self.academia.search(&keyword, 10) {
                if index < self.papers.len() && !indices.contains(&index) {
                    indices.push(index);
                    if indices.len() == 10 { break; }
                }
            }
        }

        indices.into_iter().map(|index| &self.papers[index]).collect()
    }

    /// Get a paper by ID.
    pub fn get_paper(&self, id: u64) -> Option<&ResearchArtifact> {
        self.papers.iter().find(|paper| paper.id == id)
    }

    /// Get the number of stored papers.
    pub fn paper_count(&self) -> usize {
        debug_assert_eq!(self.papers.len(), self.academia.len());
        self.papers.len()
    }

    // ─── Notebook management ─────────────────────────────────────────────

    /// Create a notebook entry.
    pub fn create_notebook(&mut self, title: &str, content: &str, tags: Vec<String>, related_papers: Vec<u64>) -> u64 {
        let id = self.next_notebook_id;
        self.next_notebook_id += 1;

        let entry = NotebookEntry {
            id,
            title: title.to_string(),
            content: content.to_string(),
            tags,
            related_papers,
            created_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        };

        self.notebooks.push(entry);
        self.audit("notebook_created", Some(id), "workbench");
        id
    }

    /// Get a notebook entry by ID.
    pub fn get_notebook(&self, id: u64) -> Option<&NotebookEntry> {
        self.notebooks.iter().find(|n| n.id == id)
    }

    /// Search notebooks by tag.
    pub fn search_notebooks_by_tag(&self, tag: &str) -> Vec<&NotebookEntry> {
        self.notebooks.iter()
            .filter(|n| n.tags.contains(&tag.to_string()))
            .collect()
    }

    /// List all notebook entries.
    pub fn list_notebooks(&self) -> Vec<&NotebookEntry> {
        self.notebooks.iter().collect()
    }

    /// Get the number of notebook entries.
    pub fn notebook_count(&self) -> usize {
        self.notebooks.len()
    }

    // ─── Report generation ───────────────────────────────────────────────

    /// Generate a report from notebook entries and research findings.
    pub fn generate_report(&mut self, title: &str, sections: Vec<(String, String)>, reference_paper_ids: Vec<u64>) -> u64 {
        let id = self.next_report_id;
        self.next_report_id += 1;

        let report_sections: Vec<ReportSection> = sections
            .into_iter()
            .map(|(heading, content)| ReportSection { heading, content })
            .collect();

        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(title.as_bytes());
        for ref_id in &reference_paper_ids {
            buf.extend_from_slice(&ref_id.to_le_bytes());
        }
        let hash = crate::event_log::sha3_256(&buf);

        let report = Report {
            id,
            title: title.to_string(),
            sections: report_sections,
            references: reference_paper_ids,
            hash,
        };

        self.reports.push(report);
        self.audit("report_generated", Some(id), "workbench");
        id
    }

    /// Get a report by ID.
    pub fn get_report(&self, id: u64) -> Option<&Report> {
        self.reports.iter().find(|r| r.id == id)
    }

    /// List all reports.
    pub fn list_reports(&self) -> Vec<&Report> {
        self.reports.iter().collect()
    }

    /// Get the number of reports.
    pub fn report_count(&self) -> usize {
        self.reports.len()
    }

    // ─── Audit log ───────────────────────────────────────────────────────

    /// Get audit entries.
    pub fn audit_log(&self) -> &[AuditLogEntry] {
        &self.audit_log
    }

    /// Get audit entries for a specific action.
    pub fn audit_for_action(&self, action: &str) -> Vec<&AuditLogEntry> {
        self.audit_log.iter()
            .filter(|e| e.action == action)
            .collect()
    }

    // ─── Skill extraction (wraps SkillExtractor concept) ─────────────────

    /// Extract structured skills from a document string.
    ///
    /// This wraps the SkillExtractor concept: extracts frameworks, decision rules,
    /// anti-patterns from text. Returns a list of extracted skill summaries.
    pub fn extract_skills_from_document(&self, document: &str) -> Vec<ExtractedSkill> {
        // Simple extraction: find lines that look like frameworks/rules/anti-patterns.
        let mut skills = Vec::new();

        for line in document.lines() {
            let lower = line.to_lowercase();
            if lower.starts_with("framework:") || lower.starts_with("rule:") || lower.starts_with("anti-pattern:") {
                skills.push(ExtractedSkill {
                    kind: if lower.starts_with("framework:") { "framework".to_string() }
                        else if lower.starts_with("rule:") { "rule".to_string() }
                        else { "anti-pattern".to_string() },
                    title: line.trim_start_matches(|c: char| c == ':' || c == ' ').to_string(),
                    savings_ratio: 0.5, // placeholder
                });
            }
        }

        skills
    }

    // ─── ASCII report ────────────────────────────────────────────────────

    /// ASCII report of the workbench state.
    pub fn ascii_report(&self) -> String {
        let mut out = String::from("=== Research Workbench Report ===\n");
        out.push_str(&format!("Papers: {}\n", self.paper_count()));
        out.push_str(&format!("Notebooks: {}\n", self.notebook_count()));
        out.push_str(&format!("Reports: {}\n", self.report_count()));
        out.push_str(&format!("Audit entries: {}\n", self.audit_log.len()));

        out.push_str("\nRecent papers:\n");
        for paper in self.papers.iter().rev().take(3) {
            out.push_str(&format!(
                "  [{}] {}\n",
                paper.arxiv_id, paper.title
            ));
        }

        out.push_str("\nRecent notebooks:\n");
        for notebook in self.notebooks.iter().rev().take(3) {
            out.push_str(&format!(
                "  #{}: {} [{}]\n",
                notebook.id, notebook.title, notebook.tags.join(",")
            ));
        }

        out.push_str("\nReports:\n");
        for report in &self.reports {
            out.push_str(&format!(
                "  #{}: {} ({} sections, {} refs)\n",
                report.id, report.title, report.sections.len(), report.references.len()
            ));
        }

        out.push_str("\n=== End Report ===\n");
        out
    }
}

/// A skill extracted from a document.
#[derive(Debug, Clone)]
pub struct ExtractedSkill {
    /// Type of skill (framework, rule, anti-pattern, etc.).
    pub kind: String,
    /// Title/description of the skill.
    pub title: String,
    /// Estimated token savings ratio (placeholder).
    pub savings_ratio: f64,
}

impl Default for ResearchWorkbench {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workbench() -> ResearchWorkbench {
        ResearchWorkbench::new()
    }

    #[test]
    fn new_workbench_is_empty() {
        let wb = make_workbench();
        assert_eq!(wb.paper_count(), 0);
        assert_eq!(wb.notebook_count(), 0);
        assert_eq!(wb.report_count(), 0);
        assert_eq!(wb.audit_log().len(), 0);
    }

    #[test]
    fn add_paper_stores_paper() {
        let mut wb = make_workbench();
        let id = wb.add_paper(
            "Test Paper",
            "This is an abstract",
            &["cs.AI".to_string()],
            "2101.00101",
        );
        assert!(id > 0);
        assert_eq!(wb.paper_count(), 1);

        let paper = wb.get_paper(id).unwrap();
        assert_eq!(paper.title, "Test Paper");
        assert_eq!(paper.arxiv_id, "2101.00101");
    }

    #[test]
    fn search_papers_finds_by_keyword() {
        let mut wb = make_workbench();
        wb.add_paper("Machine Learning Basics", "abstract about ML", &["cs.LG".to_string()], "2101.00101");
        wb.add_paper("Deep Neural Networks", "abstract about DNN", &["cs.LG".to_string()], "2101.00202");
        wb.add_paper("Quantum Computing", "abstract about QC", &["quant-ph".to_string()], "2101.00303");

        let results = wb.search_papers("neural");
        assert!(!results.is_empty());
    }

    #[test]
    fn create_notebook_stores_entry() {
        let mut wb = make_workbench();
        let id = wb.create_notebook(
            "My Notes",
            "Some content here",
            vec!["research".to_string(), "ml".to_string()],
            vec![],
        );
        assert_eq!(id, 0);
        assert_eq!(wb.notebook_count(), 1);

        let notebook = wb.get_notebook(id).unwrap();
        assert_eq!(notebook.title, "My Notes");
        assert_eq!(notebook.tags.len(), 2);
    }

    #[test]
    fn search_notebooks_by_tag() {
        let mut wb = make_workbench();
        wb.create_notebook("A", "content", vec!["ml".to_string()], vec![]);
        wb.create_notebook("B", "content", vec!["physics".to_string()], vec![]);
        wb.create_notebook("C", "content", vec!["ml".to_string(), "deep".to_string()], vec![]);

        let ml_notebooks = wb.search_notebooks_by_tag("ml");
        assert_eq!(ml_notebooks.len(), 2);
    }

    #[test]
    fn generate_report_creates_report() {
        let mut wb = make_workbench();
        let id = wb.generate_report(
            "Research Summary",
            vec![
                ("Introduction".to_string(), "This is the intro".to_string()),
                ("Methods".to_string(), "We used X".to_string()),
            ],
            vec![],
        );
        assert_eq!(id, 0);
        assert_eq!(wb.report_count(), 1);

        let report = wb.get_report(id).unwrap();
        assert_eq!(report.title, "Research Summary");
        assert_eq!(report.sections.len(), 2);
        assert_eq!(report.hash.len(), 32);
    }

    #[test]
    fn audit_log_records_actions() {
        let mut wb = make_workbench();
        wb.add_paper("P1", "abstract", &[], "2101.001");
        wb.create_notebook("N1", "content", vec![], vec![]);
        wb.generate_report("R1", vec![], vec![]);

        let audit = wb.audit_log();
        assert_eq!(audit.len(), 3);

        // Check actions.
        assert_eq!(audit[0].action, "paper_added");
        assert_eq!(audit[1].action, "notebook_created");
        assert_eq!(audit[2].action, "report_generated");
    }

    #[test]
    fn extract_skills_from_document() {
        let wb = make_workbench();
        let doc = r#"
framework: Transformer architecture
rule: Always validate inputs before processing
anti-pattern: Using global state for everything
some regular line that should be ignored
"#;

        let skills = wb.extract_skills_from_document(doc);
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].kind, "framework");
        assert_eq!(skills[1].kind, "rule");
        assert_eq!(skills[2].kind, "anti-pattern");
    }

    #[test]
    fn ascii_report_format() {
        let wb = make_workbench();
        let report = wb.ascii_report();
        assert!(report.contains("Research Workbench Report"));
        assert!(report.contains("Papers: 0"));
        assert!(report.contains("Notebooks: 0"));
        assert!(report.contains("Reports: 0"));
    }

    #[test]
    fn related_papers_stored_in_notebook() {
        let mut wb = make_workbench();
        let paper_id = wb.add_paper("Related", "abstract", &[], "2101.001");
        let nb_id = wb.create_notebook(
            "Notes",
            "content",
            vec![],

        vec![paper_id],
        );
        let notebook = wb.get_notebook(nb_id).unwrap();
        assert_eq!(notebook.related_papers.len(), 1);
        assert_eq!(notebook.related_papers[0], paper_id);
    }

    #[test]
    fn report_includes_references() {
        let mut wb = make_workbench();
        let paper1 = wb.add_paper("Paper 1", "abstract1", &[], "2101.001");
        let paper2 = wb.add_paper("Paper 2", "abstract2", &[], "2101.002");

        let report_id = wb.generate_report(
            "Lit Review",
            vec![("Review".to_string(), "content".to_string())],
            vec![paper1, paper2],
        );

        let report = wb.get_report(report_id).unwrap();
        assert_eq!(report.references.len(), 2);
        assert!(report.references.contains(&paper1));
        assert!(report.references.contains(&paper2));
    }
}
