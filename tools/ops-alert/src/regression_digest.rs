// P45-W0 §4c.2 regression-digest — generate docs/regressions/REGRESSION-DIGEST.md
// from the LIVE rows of docs/regressions/REGRESSION-LEDGER.md (§3 VIEW schema).
// Pure std. Emits a one-line-per-row human-readable table + a header summary.
// Status is derived cheaply (no re-run of the world): if the guardrail's `Where`
// references a path that exists in the repo it is GREEN; rows whose referenced
// artifacts are gone (deleted legacy code) are UNVERIFIED — never fabricated green.
// Drift gate: CI regenerates and `git diff`s against the committed digest; a stale
// or hand-edited digest is RED (same regenerate-and-diff pattern as P-A codegen gate).
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::exit;

struct Row {
    id: String,
    class: String,
    guardrail_type: String,
    where_col: String,
    last_verified: String,
}

fn find_root() -> PathBuf {
    if let Ok(r) = env::var("DIGEST_ROOT") {
        return PathBuf::from(r);
    }
    let mut dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join("docs/regressions/REGRESSION-LEDGER.md").exists() {
            return dir;
        }
        if !dir.pop() {
            return env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

// Extract LIVE-section table rows only.
fn parse_live_rows(txt: &str) -> Vec<Row> {
    let lines: Vec<&str> = txt.lines().collect();
    let mut in_live = false;
    let mut rows = Vec::new();
    for line in lines {
        let t = line.trim();
        if t.starts_with("## Ledger — LIVE") {
            in_live = true;
            continue;
        }
        if in_live && t.starts_with("## ") {
            break; // next section
        }
        if !in_live {
            continue;
        }
        if !t.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = t.split('|').map(|c| c.trim()).collect();
        // table: ["" , #, Symptom, Root cause, Guardrail type, Where, Date/commit, ""]
        if cells.len() < 7 {
            continue;
        }
        // skip header + separator
        if cells[1] == "#" || cells[1].starts_with("---") || cells[1].is_empty() {
            continue;
        }
        let id = cells[1].to_string();
        let symptom = strip_md(cells[2]);
        let class = first_words(&symptom, 15);
        let guardrail_type =
            strip_md(cells[4]).split_whitespace().collect::<Vec<_>>()[0..].join(" ");
        let where_col = strip_md(cells[5]);
        let last_verified = strip_md(cells[6]);
        rows.push(Row {
            id,
            class,
            guardrail_type,
            where_col,
            last_verified,
        });
    }
    rows
}

fn strip_md(s: &str) -> String {
    s.replace('`', "")
        .replace("**", "")
        .replace('*', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn first_words(s: &str, n: usize) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let take = words.len().min(n);
    words[..take].join(" ")
}

// Derive status: if `where_col` names a path that exists in the repo -> GREEN,
// else UNVERIFIED. We search for a plausible path token (file or dir) in the column.
fn derive_status(root: &PathBuf, where_col: &str) -> &'static str {
    for tok in where_col
        .split(|c: char| !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-')
    {
        if tok.len() < 3 {
            continue;
        }
        // try a few normalizations
        for cand in [tok.to_string(), tok.trim_start_matches("./").to_string()] {
            if cand.contains('/')
                || cand.ends_with(".rs")
                || cand.ends_with(".sh")
                || cand.ends_with(".toml")
                || cand.ends_with(".ts")
                || cand.ends_with(".sql")
                || cand.ends_with(".py")
                || cand.ends_with(".md")
            {
                if root.join(&cand).exists() {
                    return "GREEN";
                }
            }
        }
    }
    "UNVERIFIED"
}

fn main() {
    let root = find_root();
    let ledger = root.join("docs/regressions/REGRESSION-LEDGER.md");
    let txt = match fs::read_to_string(&ledger) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("regression-digest: cannot read {}: {e}", ledger.display());
            exit(2);
        }
    };
    let rows = parse_live_rows(&txt);
    let mut green = 0usize;
    let mut unverified = 0usize;
    let mut out = String::new();
    out.push_str("# Regression Digest — dowiz / DeliveryOS\n\n");
    out.push_str("> GENERATED VIEW. The `REGRESSION-LEDGER.md` is authoritative; this digest is\n");
    out.push_str("> a one-line-per-row readable projection. Each row links back via its id.\n");
    out.push_str("> Regenerate with `cargo run --manifest-path tools/ops-alert/Cargo.toml --bin regression-digest`.\n\n");
    // summary line (machine-checkable)
    for r in &rows {
        let st = derive_status(&root, &r.where_col);
        if st == "GREEN" {
            green += 1;
        } else {
            unverified += 1;
        }
    }
    out.push_str(&format!(
        "SUMMARY: {} live guardrails | {} green | {} unverified (generated {} by regression-digest)\n\n",
        rows.len(),
        green,
        unverified,
        now_str()
    ));
    out.push_str(
        "| # | class (≤15 words) | guardrail type | how to run | status | last verified |\n",
    );
    out.push_str("|---|---|---|---|---|---|\n");
    for r in &rows {
        let st = derive_status(&root, &r.where_col);
        let run = if r.where_col.contains("CI:") {
            r.where_col.clone()
        } else {
            format!("path: {}", r.where_col)
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            r.id, r.class, r.guardrail_type, run, st, r.last_verified
        ));
    }
    out.push_str("\n---\n*Readability sign-off (operator): ___ how many live / which red / re-run row N ___ (DoD-c)\n");

    if let Some(arg) = env::args().nth(1) {
        if arg == "--check" {
            // drift gate: compare to committed file; exit 1 if differs
            let committed = root.join("docs/regressions/REGRESSION-DIGEST.md");
            match fs::read_to_string(&committed) {
                Ok(prev) if prev == out => {
                    println!("regression-digest: committed digest is current");
                    return;
                }
                _ => {
                    eprintln!("::error::REGRESSION-DIGEST.md is stale or missing — regenerate (drift gate)");
                    exit(1);
                }
            }
        }
    }
    let dest = root.join("docs/regressions/REGRESSION-DIGEST.md");
    if let Err(e) = fs::write(&dest, out) {
        eprintln!("regression-digest: cannot write {}: {e}", dest.display());
        exit(2);
    }
    println!(
        "regression-digest: wrote {} ({} rows)",
        dest.display(),
        rows.len()
    );
}

fn now_str() -> String {
    // no RNG/chrono dep: use a fixed placeholder the regen gate tolerates via --check on content
    // (the summary timestamp is informational; --check compares full content so we keep it stable)
    "via regression-digest".to_string()
}
