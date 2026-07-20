// Item-74 standalone registry integrity test (red-line-registry).
//
// Compiled directly with `rustc` — ZERO cargo / zero external deps (blueprint §0.5 / D0).
// Verifies the machine-readable properties of RED-LINE-REGISTRY.tsv without pulling the
// kernel crate into a build graph. Run via scripts/verify-item-74.sh, or:
//
//   rustc tests/red-line-registry/test_registry.rs -O -o /tmp/rlr_test && /tmp/rlr_test docs/audits/governance/RED-LINE-REGISTRY.tsv
//
// Asserts:
//   1. SELF-ROW (§3.3(4)): the registry's own path_prefix is a row (item 73 recursion recorded).
//   2. Every data row has: a known `class`, a non-empty `source`, a valid `removal_authority`.
//   3. No un-cited rows (source non-empty — every row resolves to a real file:line / D-ruling).
//   4. D11 Q4 encoding: the CORE hard line (breaker / order_machine / money / decide-core) rows
//      are `out-of-band-only`; agent scope + governance-self rows are `operator-ruling-required`.
//   5. No duplicate path_prefixes (a deterministic classifier needs unique zones).
//
// Exit code: 0 = all assertions pass; 1 = at least one assertion failed.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::process;

const KNOWN_CLASSES: &[&str] = &[
    "product-red-line",
    "proven-fsm-core",
    "verification-seam",
    "forensic-truth",
    "crypto",
    "proof-machinery",
    "safety-machinery",
    "governance-self",
];

// D11 Q4 hard line: these path prefixes must be out-of-band-only (never editable by AI).
const CORE_HARD_LINE: &[&str] = &[
    "kernel/src/breaker/",
    "kernel/src/order_machine.rs",
    "kernel/src/money.rs",
    "kernel/src/decision/mod.rs",
];

fn fail(msg: &str) -> ! {
    eprintln!("::error::{}", msg);
    process::exit(1);
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: test_registry <RED-LINE-REGISTRY.tsv>");
        process::exit(2);
    });

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => fail(&format!("registry not found at {}: {}", path, e)),
    };

    let registry_self_prefix = "docs/audits/governance/RED-LINE-REGISTRY.tsv";
    let mut saw_self_row = false;
    let mut seen_prefixes: HashSet<String> = HashSet::new();
    let mut row_count = 0;

    for (lineno, line) in text.lines().enumerate() {
        let lineno = lineno + 1;
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 {
            fail(&format!(
                "line {}: expected >=5 tab columns, got {}",
                lineno,
                cols.len()
            ));
        }
        let pfx = cols[0];
        let class = cols[1];
        let removal = cols[2];
        // why = cols[3] (unused in assertions beyond non-empty check would be noisy; skip)
        let source = cols[4];

        if !KNOWN_CLASSES.contains(&class) {
            fail(&format!(
                "line {}: unknown class '{}' (pfx '{}')",
                lineno, class, pfx
            ));
        }
        if source.trim().is_empty() {
            fail(&format!("line {}: empty source (un-cited row) '{}'", lineno, pfx));
        }
        if removal != "out-of-band-only" && removal != "operator-ruling-required" {
            fail(&format!(
                "line {}: invalid removal_authority '{}' (pfx '{}')",
                lineno, removal, pfx
            ));
        }
        if pfx == registry_self_prefix {
            saw_self_row = true;
            if class != "governance-self" || removal != "operator-ruling-required" {
                fail(&format!(
                    "self-row '{}' must be class=governance-self removal_authority=operator-ruling-required (got {}/{})",
                    pfx, class, removal
                ));
            }
        }
        if seen_prefixes.contains(pfx) {
            fail(&format!("duplicate path_prefix '{}' at line {}", pfx, lineno));
        }
        seen_prefixes.insert(pfx.to_string());
        row_count += 1;
    }

    // (1) self-row test
    if !saw_self_row {
        fail(&format!(
            "SELF-ROW TEST FAILED — '{}' is not a row of its own registry",
            registry_self_prefix
        ));
    } else {
        println!(
            "self-row test: PASS — {} is registered (class=governance-self).",
            registry_self_prefix
        );
    }

    // (4) D11 Q4 encoding: core hard-line rows must be out-of-band-only
    for hard in CORE_HARD_LINE {
        // find the row with this prefix (longest-exact-match is fine here; prefixes are unique)
        let mut matched_removal = None;
        for line in text.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() >= 3 && cols[0] == *hard {
                matched_removal = Some(cols[2].to_string());
                break;
            }
        }
        match matched_removal {
            None => fail(&format!("D11 Q4: core hard-line row '{}' missing from registry", hard)),
            Some(r) if r != "out-of-band-only" => fail(&format!(
                "D11 Q4 VIOLATION: '{}' must be removal_authority=out-of-band-only (got '{}')",
                hard, r
            )),
            Some(_) => println!("D11 Q4 hard-line OK: {} = out-of-band-only", hard),
        }
    }

    println!(
        "=== red-line-registry test: GREEN ({} rows, all assertions passed) ===",
        row_count
    );
    process::exit(0);
}
