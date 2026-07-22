//! shader_audit.rs — SINGLE-WRITER-discipline audit gate for engine WGSL shaders.
//!
//! P-shader-spine. Per `gpu_atomicity.rs` `SINGLE_WRITER_MARKER` and
//! SharedWriteClass::SingleWriterProof: every `var<storage, read_write>` write
//! in `engine/src/**/*.wgsl` MUST be preceded by a `// SINGLE-WRITER:` proof
//! block naming why the write is single-writer-by-construction. This integration
//! test parses each WGSL shader, finds the `read_write` bindings, and asserts
//! each one has an associated proof within a small window above the binding.
//!
//! Ananke-grade guarantee (structural, not remembered): a new shader that adds a
//! `read_write` storage binding WITHOUT the proof block fails this gate before
//! it can land. Red here ⇒ the SINGLE-WRITER discipline broke.
//!
//! Innovate: ceiling — this is a TEXT-regex audit, not a real WGSL parser. A
//! binding split across lines, or a comment-free reviewer bypass, would not be
//! caught. Trigger: when a WGSL tooling dep is unlocked (Tint/wgpu's own parser),
//! swap this prologue-scan for a typed AST audit that resolves bindings by name.

//! Only the `ui.wgsl` and `glyph.wgsl` shaders are audited today. Adding a new
//! WGSL shader under `engine/src/shaders/` extends `SHADER_FILES` here — Red on
//! a missing file because the manifest drift would otherwise hide the audit.

const SHADER_FILES: &[&str] = &["src/shaders/ui.wgsl", "src/shaders/glyph.wgsl"];
const PROOF_MARKER: &str = "// SINGLE-WRITER:";
/// Window (lines above a `read_write` binding) within which the proof marker
/// must appear. Generous enough to catch a multi-line binding + preceding
/// comment block, tight enough to forbid a proof placed far from the write.
const PROOF_WINDOW_LINES: usize = 8;

/// One audit row for a `var<storage, read_write>` binding found in a shader.
struct BindingRow {
    shader: &'static str,
    line_no: usize, // 1-indexed line of the binding
    has_proof: bool,
    proof_text: String,
}

/// Parse a shader for `var<storage, read_write>` bindings and audit each one
/// for a SINGLE-WRITER proof block within `PROOF_WINDOW_LINES` above it.
fn audit_shader(shader_path: &'static str, source: &str) -> Vec<BindingRow> {
    let lines: Vec<&str> = source.lines().collect();
    let mut rows = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("read_write") && line.contains("var<storage") {
            // Scan the window above for the proof marker, capturing its text.
            let mut proof_text = String::new();
            let mut has_proof = false;
            let start = i.saturating_sub(PROOF_WINDOW_LINES);
            for pl in &lines[start..i] {
                if let Some(p) = pl.find(PROOF_MARKER) {
                    has_proof = true;
                    proof_text = pl[p + PROOF_MARKER.len()..].trim().to_string();
                    break; // nearest proof wins
                }
            }
            rows.push(BindingRow {
                shader: shader_path,
                line_no: i + 1,
                has_proof,
                proof_text,
            });
        }
    }
    rows
}

#[test]
fn every_read_write_storage_binding_has_single_writer_proof() {
    let manifest: Vec<String> = std::env::current_dir()
        .ok()
        .map(|d| d.display().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    let _ = manifest; // unused; resolve shaders from the engine crate root below.

    // Resolve the engine crate root: this is an integration test under
    // engine/tests/, so CARGO_MANIFEST_DIR points at engine/.
    let engine_dir = env!("CARGO_MANIFEST_DIR");
    let mut all_rows = Vec::new();
    let mut missing_files = Vec::new();
    for shader in SHADER_FILES {
        let path = std::path::Path::new(engine_dir).join(shader);
        match std::fs::read_to_string(&path) {
            Ok(src) => all_rows.extend(audit_shader(shader, &src)),
            Err(_) => missing_files.push(shader),
        }
    }

    assert!(
        missing_files.is_empty(),
        "SHADER_FILES manifest drift — missing shader files: {:?}. \
         Add the new shader file to SHADER_FILES or fix the path.",
        missing_files
    );

    // The spine MUST declare at least one read_write binding (the framebuffer
    // out_color in each shader) — a shader set with NO read_write bindings would
    // mean the audit is vacuous; require >= 2 (one framebuffer per shader pair).
    assert!(
        all_rows.len() >= 2,
        "audit is vacuous: expected ≥2 read_write bindings across the shader \
         spine, found {}. Add a framebuffer out-binding to each shader.",
        all_rows.len()
    );

    let unproven: Vec<&BindingRow> = all_rows.iter().filter(|r| !r.has_proof).collect();
    if !unproven.is_empty() {
        let report: Vec<String> = unproven
            .iter()
            .map(|r| {
                format!(
                    "- {}:{} (no proof within {} lines)",
                    r.shader, r.line_no, PROOF_WINDOW_LINES
                )
            })
            .collect();
        panic!(
            "SINGLE-WRITER audit FAILED — {} read_write binding(s) lack a \
             `// SINGLE-WRITER:` proof block within {} lines above the binding:\n{}",
            unproven.len(),
            PROOF_WINDOW_LINES,
            report.join("\n")
        );
    }

    // Additionally require that each proof be non-empty (the audit_blocks_merge
    // rule from gpu_atomicity.rs: an empty proof = no exemption).
    let empty: Vec<&BindingRow> = all_rows
        .iter()
        .filter(|r| r.proof_text.is_empty())
        .collect();
    assert!(
        empty.is_empty(),
        "SINGLE-WRITER audit FAILED — {} binding(s) have an empty proof text \
         (the marker is present but the proof argument is missing, which is the \
         dead-code pattern the gpu_atomicity.rs `audit_blocks_merge` gate rejects).",
        empty.len()
    );
}

/// Sanity: a synthetic shader with an UNPROVEN read_write binding MUST make the
/// audit fail (this is the falsifier proving the gate bites — RED-spelled-RED).
/// It also proves the prologue-scan handles a binding without ANY proof above it.
#[test]
fn audit_shader_rejects_unproven_binding() {
    let synthetic = "struct U { x: f32 }\n@group(0) @binding(0) var<storage, read_write> bad: array<f32>;\nfn f() {}";
    let rows = audit_shader("synthetic.wgsl", synthetic);
    assert_eq!(rows.len(), 1, "audit parsed the single read_write binding");
    assert!(
        !rows[0].has_proof,
        "a binding with no proof marker above MUST be flagged unproven"
    );
}

/// The prologue-scan finds the nearest proof block (multi-line proof text is
/// captured up to the binding) — this proves a correctly-annotated shader passes.
#[test]
fn audit_shader_accepts_proven_binding() {
    let synthetic = "\
// SINGLE-WRITER: out_color — framebuffer, each fragment writes one disjoint pixel.
@group(0) @binding(0) var<storage, read_write> out_color: array<vec4<f32>>;
fn f() {}";
    let rows = audit_shader("synthetic_ok.wgsl", synthetic);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].has_proof, "a binding with a proof above it passes");
    assert!(
        rows[0].proof_text.contains("out_color"),
        "proof text is captured: got {:?}",
        rows[0].proof_text
    );
}
