//! deep-clean — native Rust disk hygiene (no bash).
//!
//! Subcommands (dry-run by default; pass `--commit` to act):
//!   vacuum   — VACUUM the Hermes state.db (reclaims SQLite freelist; stops fragment growth)
//!   clean    — remove regenerable/scratch under /root via a hard deny-list (secrets NEVER matched)
//!
//! Design (per operator DEEP-CLEAN PROTOCOL + research 2026-07-16):
//!   - Deny-list structurally excludes secrets (.env*, .harness-backups, .auth, *credential*, *secret*)
//!     and KEEP-deliverables (docs/, design/, blueprint*, research/, synthesis/, wasm/demo/).
//!   - Idempotent: re-running finds nothing left to do.
//!   - Origin of Bucket-B/C is NEVER deleted here (archive+verify is a separate step).
//!   - All actions logged to /root/.backups/clean-log/<ts>.jsonl.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const STATE_DB: &str = "/root/.hermes/state.db";
const CLEAN_LOG: &str = "/root/.backups/clean-log";

// Hard deny-list: paths matching ANY of these are NEVER touched.
const EXCLUDE: &[&str] = &[
    ".env", ".env.", ".secrets", ".harness-backups", ".auth", "credential", "secret",
    "docs/", "design/", "blueprint", "research/", "synthesis/", "wasm/demo/",
    "backups/hot/", "backups/cold/",
];

// Regenerable/scratch removal targets (Bucket A + D).
const REMOVE_DIRS: &[&str] = &[
    "/root/bebop-repo/target",
    "/root/dowiz-pq/target",
    "/root/dowiz/engine/target",
    "/root/dowiz/node_modules",
    "/root/.local/share/uv",
    "/root/.local/share/pnpm",
    "/root/.npm/_cacache",
    "/root/.cache",
    "/tmp/claude-0",
    "/tmp/pytest-of-root",
];

fn ts() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn log(line: &str) {
    let entry = format!("{{\"ts\":{},\"msg\":{}}}\n", ts(), serde_str(line));
    let _ = fs::create_dir_all(CLEAN_LOG);
    let _ = fs::OpenOptions::new().create(true).append(true).open(format!("{CLEAN_LOG}/{}.jsonl", ts() / 86400))
        .and_then(|mut f| f.write_all(entry.as_bytes()));
    println!("{line}");
}

fn serde_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn excluded(p: &Path) -> bool {
    let s = p.to_string_lossy();
    EXCLUDE.iter().any(|e| s.contains(e))
}

fn vacuum(dry: bool) {
    let before = fs::metadata(STATE_DB).map(|m| m.len()).unwrap_or(0);
    log(&format!("[vacuum] state.db before={before} bytes, dry={dry}"));
    if dry {
        return;
    }
    match rusqlite::Connection::open(STATE_DB) {
        Ok(conn) => {
            // VACUUM reclaims the freelist and compacts the file. Safe, standard op.
            conn.execute_batch("VACUUM;").ok();
            let after = fs::metadata(STATE_DB).map(|m| m.len()).unwrap_or(0);
            log(&format!("[vacuum] state.db after={after} bytes (reclaimed {})", before.saturating_sub(after)));
        }
        Err(e) => log(&format!("[vacuum] ERROR opening state.db: {e}")),
    }
}

fn remove_one(p: &str, dry: bool) {
    let path = Path::new(p);
    if !path.exists() {
        return;
    }
    if excluded(path) {
        log(&format!("[clean] SKIP excluded: {p}"));
        return;
    }
    if dry {
        log(&format!("[clean][dry] would remove: {p}"));
        return;
    }
    let res = if path.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
    match res {
        Ok(_) => log(&format!("[clean] removed: {p}")),
        Err(e) => log(&format!("[clean] FAILED {p}: {e}")),
    }
}

fn clean(dry: bool) {
    log(&format!("[clean] start dry={dry}"));
    for d in REMOVE_DIRS {
        remove_one(d, dry);
    }
    // Named scratch files (operator convention) + editor leftovers.
    if let Ok(entries) = fs::read_dir("/tmp") {
        for e in entries.flatten() {
            let name = e.file_name();
            let n = name.to_string_lossy();
            if n.starts_with("hermes-verify-") {
                remove_one(e.path().to_str().unwrap_or(""), dry);
            }
        }
    }
    log("[clean] done");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let commit = args.iter().any(|a| a == "--commit");
    let dry = !commit;
    if args.iter().any(|a| a == "vacuum") {
        vacuum(dry);
    }
    if args.iter().any(|a| a == "clean") {
        clean(dry);
    }
    if !args.iter().any(|a| a == "vacuum" || a == "clean") {
        println!("usage: deep-clean [vacuum|clean] [--commit]  (dry-run by default)");
    }
}
