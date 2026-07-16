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

fn prune(dry: bool, days: u64) {
    // Safe prune: ONLY ended sessions older than `days` (never touch active/open sessions).
    // FTS5 shadow tables are rebuilt + VACUUM after delete to stay consistent.
    let cutoff = (ts() as f64) - (days as f64) * 86400.0;
    log(&format!("[prune] cutoff={cutoff:.0} (>{days}d old, ended-only), dry={dry}"));
    if dry {
        if let Ok(conn) = rusqlite::Connection::open(STATE_DB) {
            let n_s: i64 = conn
                .query_row("SELECT COUNT(*) FROM sessions WHERE ended_at IS NOT NULL AND ended_at < ?", [cutoff], |r| r.get(0))
                .unwrap_or(0);
            let n_m: i64 = conn
                .query_row("SELECT COUNT(*) FROM messages WHERE session_id IN (SELECT id FROM sessions WHERE ended_at IS NOT NULL AND ended_at < ?)", [cutoff], |r| r.get(0))
                .unwrap_or(0);
            log(&format!("[prune][dry] would delete {n_s} sessions, {n_m} messages"));
        }
        return;
    }
    match rusqlite::Connection::open(STATE_DB) {
        Ok(conn) => {
            conn.execute("DELETE FROM messages WHERE session_id IN (SELECT id FROM sessions WHERE ended_at IS NOT NULL AND ended_at < ?)", [cutoff]).ok();
            conn.execute("DELETE FROM sessions WHERE ended_at IS NOT NULL AND ended_at < ?", [cutoff]).ok();
            // Sync FTS5 shadow tables (external-content triggers may not cover bulk delete).
            let _ = conn.execute_batch("INSERT INTO messages_fts(messages_fts) VALUES('rebuild');");
            conn.execute_batch("VACUUM;").ok();
            log("[prune] done: messages+sessions deleted, FTS rebuilt, VACUUM");
        }
        Err(e) => log(&format!("[prune] ERROR opening state.db: {e}")),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let commit = args.iter().any(|a| a == "--commit");
    let dry = !commit;
    let days: u64 = args
        .iter()
        .position(|a| a == "--days")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    if args.iter().any(|a| a == "vacuum") {
        vacuum(dry);
    }
    if args.iter().any(|a| a == "clean") {
        clean(dry);
    }
    if args.iter().any(|a| a == "prune") {
        prune(dry, days);
    }
    if args.iter().any(|a| a == "all") {
        vacuum(dry);
        clean(dry);
        prune(dry, days);
    }
    if !args.iter().any(|a| a == "vacuum" || a == "clean" || a == "prune" || a == "all") {
        println!("usage: deep-clean [vacuum|clean|prune|all] [--commit] [--days N]  (dry-run by default)");
    }
}
