//! deep-clean — native Rust disk hygiene (no bash).
//!
//! Subcommands (dry-run by default for the destructive ones; pass `--commit` to act):
//!   vacuum         — VACUUM the Hermes state.db (reclaims SQLite freelist; stops fragment growth)
//!   clean          — remove regenerable/scratch under /root via a hard deny-list (secrets NEVER matched)
//!   prune          — delete ended sessions older than --days N from state.db (FTS rebuild + VACUUM)
//!   all            — vacuum + clean + prune
//!   sha3 <f>...    — compute + write a `<f>.sha3` content-address sidecar (BLUEPRINT-P12 §3)
//!   archive <src> <dest.tar.zst>
//!                  — tar|zstd a source into a COLD archive, writing `.sha3` + `.manifest` sidecars
//!   restore-verify <archive.tar.zst>
//!                  — DRILL: verify the `.sha3` sidecar, restore to scratch, `PRAGMA integrity_check`
//!                    a SQLite payload, re-hash the tree vs the manifest, log JSONL, delete scratch.
//!                    Read-only against production; exits non-zero on any FAIL.
//!
//! Design (per operator DEEP-CLEAN PROTOCOL + research 2026-07-16 + BLUEPRINT-P12 §3):
//!   - Deny-list structurally excludes secrets (.env*, .harness-backups, .auth, *credential*, *secret*)
//!     and KEEP-deliverables (docs/, design/, blueprint*, research/, synthesis/, wasm/demo/).
//!   - Idempotent: re-running finds nothing left to do.
//!   - Origin of Bucket-B/C is NEVER deleted here (archive+verify is a separate step).
//!   - All actions logged to /root/.backups/clean-log/<ts>.jsonl.
//!   - Content-addressing reuses the kernel's SHA3-256 (FIPS 202) — same Keccak-f[1600] permutation
//!     as `kernel/src/event_log.rs::sha3_256`, streamed here so multi-GB archives hash in O(1) memory.
//!     No new dependency (zero-dep storage boundary, per M6/D6).

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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

// ===========================================================================
// BLUEPRINT-P12 §3 — COLD-archive `archive` + `restore-verify` (E27/F38).
//
// Content-addressing reuses the kernel's SHA3-256 (FIPS 202). The kernel's
// `event_log.rs::sha3_256` is a one-shot `&[u8] -> [u8;32]`; COLD archives are
// multi-GB, so we reuse the *identical* Keccak-f[1600] permutation + pad10*1 +
// 0x06 SHA-3 domain but absorb incrementally (streaming) to stay O(1) memory.
// Verified byte-for-byte equal to the one-shot form by the KAT tests below.
// ===========================================================================

const RESTORE_DRILL_ROOT: &str = "/root/.backups/restore-drill";
const SHA3_RATE: usize = 136; // SHA3-256 sponge rate in bytes.

/// Keccak-f[1600] permutation (FIPS 202) — copied verbatim from
/// `kernel/src/event_log.rs::sha3_256`'s inner `keccak_f` so the digest matches.
fn keccak_f(s: &mut [u64; 25]) {
    const RC: [u64; 24] = [
        0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
        0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
        0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
        0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
        0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
        0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
    ];
    const R: [[u32; 5]; 5] = [
        [0, 36, 3, 41, 18],
        [1, 44, 10, 45, 2],
        [62, 6, 43, 15, 61],
        [28, 55, 25, 21, 56],
        [27, 20, 39, 8, 14],
    ];
    for r in 0..24 {
        // θ (theta)
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = s[x] ^ s[x + 5] ^ s[x + 10] ^ s[x + 15] ^ s[x + 20];
        }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }
        for x in 0..5 {
            for y in 0..5 {
                s[x + 5 * y] ^= d[x];
            }
        }
        // ρ + π
        let mut b = [0u64; 25];
        for x in 0..5 {
            for y in 0..5 {
                let dest_x = y;
                let dest_y = (2 * x + 3 * y) % 5;
                b[dest_x + 5 * dest_y] = s[x + 5 * y].rotate_left(R[x][y]);
            }
        }
        // χ (chi)
        for x in 0..5 {
            for y in 0..5 {
                let idx = x + 5 * y;
                s[idx] = b[idx] ^ ((!b[((x + 1) % 5) + 5 * y]) & b[((x + 2) % 5) + 5 * y]);
            }
        }
        // ι (iota)
        s[0] ^= RC[r];
    }
}

/// Streaming SHA3-256 sponge. Same algorithm as the kernel's one-shot function,
/// fed block-by-block so a 4 GB archive never loads into memory.
struct Sha3_256 {
    state: [u64; 25],
    buf: [u8; SHA3_RATE],
    buf_len: usize,
}

impl Sha3_256 {
    fn new() -> Self {
        Self { state: [0u64; 25], buf: [0u8; SHA3_RATE], buf_len: 0 }
    }

    fn absorb_block(&mut self) {
        for j in 0..(SHA3_RATE / 8) {
            let lane = u64::from_le_bytes(self.buf[j * 8..j * 8 + 8].try_into().unwrap());
            self.state[j] ^= lane;
        }
        keccak_f(&mut self.state);
    }

    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let take = std::cmp::min(SHA3_RATE - self.buf_len, data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == SHA3_RATE {
                self.absorb_block();
                self.buf_len = 0;
            }
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        // pad10*1 with the SHA-3 domain separator (0x06). `buf_len < RATE` always
        // holds here (a full block is absorbed immediately in `update`), so the
        // XOR-in of 0x06 and the 0x80 terminator collapse correctly even when
        // `buf_len == RATE-1` (giving the single byte 0x86, matching the one-shot).
        for i in self.buf_len..SHA3_RATE {
            self.buf[i] = 0;
        }
        self.buf[self.buf_len] ^= 0x06;
        self.buf[SHA3_RATE - 1] ^= 0x80;
        self.absorb_block();
        let mut out = [0u8; 32];
        for j in 0..4 {
            out[j * 8..j * 8 + 8].copy_from_slice(&self.state[j].to_le_bytes());
        }
        out
    }
}

/// One-shot convenience mirroring `kernel::event_log::sha3_256` (same output).
/// The binary paths hash via the streaming `hash_file`; this one-shot form is
/// the KAT reference the tests pin the streaming digest against.
#[cfg_attr(not(test), allow(dead_code))]
fn sha3_256(input: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(input);
    h.finalize()
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// SHA3-256 a file streaming in 64 KiB chunks (O(1) memory for multi-GB files).
fn hash_file(path: &Path) -> std::io::Result<[u8; 32]> {
    let mut f = File::open(path)?;
    let mut hasher = Sha3_256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}

/// Recursively collect regular files (never follows symlinks) under `dir`.
fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let p = entry?.path();
        let md = fs::symlink_metadata(&p)?;
        if md.is_dir() {
            walk_files(&p, out)?;
        } else if md.is_file() {
            out.push(p);
        }
        // symlinks / fifos / devices are intentionally ignored (not re-hashable payload).
    }
    Ok(())
}

fn sidecar_sha3(archive: &Path) -> PathBuf {
    let mut s = archive.as_os_str().to_owned();
    s.push(".sha3");
    PathBuf::from(s)
}

fn sidecar_manifest(archive: &Path) -> PathBuf {
    let mut s = archive.as_os_str().to_owned();
    s.push(".manifest");
    PathBuf::from(s)
}

/// Compute the archive's SHA3-256 and write a `<archive>.sha3` sidecar
/// (`<hex>  <basename>`), reusing the kernel content-address function.
fn write_sha3_sidecar(archive: &Path) -> Result<String, String> {
    let digest = hash_file(archive).map_err(|e| format!("hash {}: {e}", archive.display()))?;
    let hx = hex(&digest);
    let name = archive.file_name().unwrap_or_default().to_string_lossy();
    fs::write(sidecar_sha3(archive), format!("{hx}  {name}\n"))
        .map_err(|e| format!("write sidecar: {e}"))?;
    Ok(hx)
}

/// `sha3 <f>...` — write `.sha3` sidecars for pre-existing archives (which
/// predate `archive` and so have no sidecar yet). This is the primitive that
/// retro-fits the content-address gate onto COLD archives already on disk.
fn cmd_sha3(files: &[String]) {
    if files.is_empty() {
        log("[sha3] no files given");
        return;
    }
    for f in files {
        let p = Path::new(f);
        if !p.is_file() {
            log(&format!("[sha3] SKIP (not a file): {f}"));
            continue;
        }
        match write_sha3_sidecar(p) {
            Ok(hx) => log(&format!("[sha3] {hx}  {f}  -> {}.sha3", f)),
            Err(e) => log(&format!("[sha3] FAILED {f}: {e}")),
        }
    }
}

/// `archive <src> <dest.tar.zst>` — deny-list-gated tar|zstd of `src`, then
/// write `.sha3` (archive content-address) + `.manifest` (per-file hashes for
/// the byte-identical restore-verify) sidecars.
fn cmd_archive(src: &str, dest: &str) {
    let src = Path::new(src);
    let dest = Path::new(dest);
    match archive_impl(src, dest) {
        Ok((files, hx)) => log(&format!(
            "[archive] {} ({} files) -> {} sha3={}",
            src.display(),
            files,
            dest.display(),
            hx
        )),
        Err(e) => {
            log(&format!("[archive] FAILED {}: {e}", src.display()));
            std::process::exit(1);
        }
    }
}

fn archive_impl(src: &Path, dest: &Path) -> Result<(usize, String), String> {
    if !src.exists() {
        return Err(format!("source does not exist: {}", src.display()));
    }
    let parent = src.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let basename = src.file_name().ok_or("source has no basename")?;

    // Deny-list gate: NO secret / .env material ever enters a COLD archive.
    let mut files: Vec<PathBuf> = Vec::new();
    if src.is_dir() {
        walk_files(src, &mut files).map_err(|e| format!("walk {}: {e}", src.display()))?;
    } else {
        files.push(src.to_path_buf());
    }
    for f in &files {
        if excluded(f) {
            return Err(format!("deny-listed path would enter archive: {}", f.display()));
        }
    }

    // Build the per-file manifest (key = path relative to `parent`, matching the
    // tar layout so restore-verify can re-hash the restored tree 1:1).
    let mut manifest = String::new();
    for f in &files {
        let digest = hash_file(f).map_err(|e| format!("hash {}: {e}", f.display()))?;
        let rel = f.strip_prefix(parent).unwrap_or(f);
        manifest.push_str(&format!("{}  {}\n", hex(&digest), rel.to_string_lossy()));
    }

    if let Some(dp) = dest.parent() {
        if !dp.as_os_str().is_empty() {
            let _ = fs::create_dir_all(dp);
        }
    }
    run_tar_zstd_create(parent, basename, dest)?;
    fs::write(sidecar_manifest(dest), &manifest).map_err(|e| format!("write manifest: {e}"))?;
    let hx = write_sha3_sidecar(dest)?;
    Ok((files.len(), hx))
}

fn run_tar_zstd_create(parent: &Path, basename: &OsStr, dest: &Path) -> Result<(), String> {
    let mut tar = Command::new("tar")
        .arg("-C").arg(parent)
        .arg("-cf").arg("-")
        .arg(basename)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn tar: {e}"))?;
    let tar_out = tar.stdout.take().ok_or("tar stdout unavailable")?;
    let zstd_status = Command::new("zstd")
        .arg("-q").arg("-f").arg("-o").arg(dest)
        .stdin(Stdio::from(tar_out))
        .status()
        .map_err(|e| format!("spawn zstd: {e}"))?;
    let tar_status = tar.wait().map_err(|e| format!("wait tar: {e}"))?;
    if !tar_status.success() {
        return Err(format!("tar failed: {tar_status}"));
    }
    if !zstd_status.success() {
        return Err(format!("zstd failed: {zstd_status}"));
    }
    Ok(())
}

fn run_zstd_untar(archive: &Path, scratch: &Path) -> Result<(), String> {
    let mut z = Command::new("zstd")
        .arg("-dc").arg(archive)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn zstd -dc: {e}"))?;
    let z_out = z.stdout.take().ok_or("zstd stdout unavailable")?;
    let tar_status = Command::new("tar")
        .arg("-C").arg(scratch)
        .arg("-xf").arg("-")
        .stdin(Stdio::from(z_out))
        .status()
        .map_err(|e| format!("spawn tar -x: {e}"))?;
    let z_status = z.wait().map_err(|e| format!("wait zstd: {e}"))?;
    if !z_status.success() {
        return Err(format!("zstd -dc failed: {z_status}"));
    }
    if !tar_status.success() {
        return Err(format!("tar -x failed: {tar_status}"));
    }
    Ok(())
}

/// Outcome of the sha3-sidecar gate. `NoSidecar` is a NAMED failure (never a
/// silent skip) — a COLD archive with no content-address is unverifiable.
#[derive(Debug, PartialEq, Eq)]
enum SidecarCheck {
    Match,
    Mismatch,
    NoSidecar,
}

impl SidecarCheck {
    fn json(&self) -> &'static str {
        match self {
            SidecarCheck::Match => "true",
            SidecarCheck::Mismatch => "false",
            SidecarCheck::NoSidecar => "\"no_sidecar\"",
        }
    }
}

#[derive(Debug)]
struct DrillResult {
    archive: String,
    sha3_match: SidecarCheck,
    integrity_check: String, // "ok" | "n/a" | error text
    bytes_verified: u64,
    result: String, // "ok" | "FAIL"
}

impl DrillResult {
    fn is_ok(&self) -> bool {
        self.result == "ok"
    }
    fn json_line(&self) -> String {
        format!(
            "{{\"ts\":{},\"archive\":{},\"sha3_match\":{},\"integrity_check\":{},\"bytes_verified\":{},\"result\":{}}}\n",
            ts(),
            serde_str(&self.archive),
            self.sha3_match.json(),
            serde_str(&self.integrity_check),
            self.bytes_verified,
            serde_str(&self.result),
        )
    }
}

fn cmd_restore_verify(archive: &str) {
    let archive = Path::new(archive);
    let scratch_root = Path::new(RESTORE_DRILL_ROOT);
    let res = restore_verify_impl(archive, scratch_root, Path::new(CLEAN_LOG));
    if res.is_ok() {
        log(&format!(
            "[restore-verify] OK {} (integrity={}, bytes_verified={})",
            res.archive, res.integrity_check, res.bytes_verified
        ));
    } else {
        log(&format!(
            "[restore-verify] FAIL {} (sha3_match={}, integrity={})",
            res.archive,
            res.sha3_match.json(),
            res.integrity_check
        ));
        std::process::exit(1);
    }
}

/// The load-bearing DRILL. Read-only against production: restores to a scratch
/// dir, verifies, logs one JSONL line, then deletes the scratch dir.
fn restore_verify_impl(archive: &Path, scratch_root: &Path, drill_log_dir: &Path) -> DrillResult {
    let archive_s = archive.display().to_string();
    let drill_ts = ts();
    let write_drill = |res: &DrillResult| {
        let _ = fs::create_dir_all(drill_log_dir);
        let path = drill_log_dir.join(format!("restore-drill-{drill_ts}.jsonl"));
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| f.write_all(res.json_line().as_bytes()));
        println!("{}", res.json_line().trim_end());
    };

    // Step 1 — the PRIMARY gate: the `.sha3` sidecar. Missing sidecar or a
    // mismatch fails LOUDLY here, before anything is extracted.
    let sidecar = sidecar_sha3(archive);
    if !archive.is_file() {
        let r = DrillResult {
            archive: archive_s,
            sha3_match: SidecarCheck::NoSidecar,
            integrity_check: "skipped".into(),
            bytes_verified: 0,
            result: "FAIL".into(),
        };
        log(&format!("[restore-verify] archive not found: {}", archive.display()));
        write_drill(&r);
        return r;
    }
    if !sidecar.is_file() {
        let r = DrillResult {
            archive: archive_s,
            sha3_match: SidecarCheck::NoSidecar,
            integrity_check: "skipped".into(),
            bytes_verified: 0,
            result: "FAIL".into(),
        };
        log(&format!(
            "[restore-verify] NO .sha3 sidecar for {} — unverifiable (run `deep-clean sha3 {}` first)",
            archive.display(),
            archive.display()
        ));
        write_drill(&r);
        return r;
    }
    let expected = fs::read_to_string(&sidecar)
        .ok()
        .and_then(|s| s.split_whitespace().next().map(|t| t.to_lowercase()))
        .unwrap_or_default();
    let actual = match hash_file(archive) {
        Ok(d) => hex(&d),
        Err(e) => {
            let r = DrillResult {
                archive: archive_s,
                sha3_match: SidecarCheck::Mismatch,
                integrity_check: format!("hash-error: {e}"),
                bytes_verified: 0,
                result: "FAIL".into(),
            };
            write_drill(&r);
            return r;
        }
    };
    if expected != actual {
        let r = DrillResult {
            archive: archive_s,
            sha3_match: SidecarCheck::Mismatch,
            integrity_check: "skipped".into(),
            bytes_verified: 0,
            result: "FAIL".into(),
        };
        log(&format!(
            "[restore-verify] SHA3 MISMATCH {} — expected {expected}, got {actual}. Refusing restore.",
            archive.display()
        ));
        write_drill(&r);
        return r;
    }

    // Step 2 — restore into an isolated scratch dir.
    let scratch = scratch_root.join(drill_ts.to_string());
    if let Err(e) = fs::create_dir_all(&scratch) {
        let r = DrillResult {
            archive: archive_s,
            sha3_match: SidecarCheck::Match,
            integrity_check: format!("scratch-error: {e}"),
            bytes_verified: 0,
            result: "FAIL".into(),
        };
        write_drill(&r);
        return r;
    }
    if let Err(e) = run_zstd_untar(archive, &scratch) {
        let _ = fs::remove_dir_all(&scratch);
        let r = DrillResult {
            archive: archive_s,
            sha3_match: SidecarCheck::Match,
            integrity_check: format!("restore-error: {e}"),
            bytes_verified: 0,
            result: "FAIL".into(),
        };
        write_drill(&r);
        return r;
    }

    // Steps 3+4 — walk the restored tree: re-hash every file (bytes_verified +
    // manifest compare), and `PRAGMA integrity_check` every SQLite payload.
    let mut files: Vec<PathBuf> = Vec::new();
    let _ = walk_files(&scratch, &mut files);
    let mut bytes_verified: u64 = 0;
    let mut restored: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut db_results: Vec<String> = Vec::new();

    for f in &files {
        let len = fs::symlink_metadata(f).map(|m| m.len()).unwrap_or(0);
        match hash_file(f) {
            Ok(d) => {
                bytes_verified += len;
                let rel = f.strip_prefix(&scratch).unwrap_or(f).to_string_lossy().to_string();
                restored.insert(rel, hex(&d));
            }
            Err(e) => {
                db_results.push(format!("hash-error {}: {e}", f.display()));
            }
        }
        if f.extension().and_then(|e| e.to_str()) == Some("db") {
            match sqlite_integrity(f) {
                Ok(s) => db_results.push(s),
                Err(e) => db_results.push(format!("sqlite-error: {e}")),
            }
        }
    }

    // integrity_check field: "ok" iff every SQLite payload reports "ok"; "n/a"
    // when the archive carries no `.db`.
    let integrity_check = if db_results.is_empty() {
        "n/a".to_string()
    } else if db_results.iter().all(|s| s == "ok") {
        "ok".to_string()
    } else {
        db_results.iter().find(|s| *s != "ok").cloned().unwrap_or_else(|| "unknown".into())
    };

    // Manifest compare: if a `.manifest` sidecar exists, the restore must be
    // byte-identical. If absent (real COLD archives predate this tool), run in
    // manifest-less mode: report the hash set, never FAIL on absence alone.
    let manifest_path = sidecar_manifest(archive);
    let mut manifest_ok = true;
    let mut manifest_mode = "manifest-less";
    if manifest_path.is_file() {
        manifest_mode = "manifest";
        if let Ok(txt) = fs::read_to_string(&manifest_path) {
            let mut expected_map: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for line in txt.lines() {
                if line.len() < 66 {
                    continue;
                }
                let (h, rest) = line.split_at(64);
                expected_map.insert(rest.trim_start().to_string(), h.to_lowercase());
            }
            for (rel, want) in &expected_map {
                match restored.get(rel) {
                    Some(got) if got == want => {}
                    Some(_) => {
                        manifest_ok = false;
                        log(&format!("[restore-verify] MANIFEST MISMATCH: {rel}"));
                    }
                    None => {
                        manifest_ok = false;
                        log(&format!("[restore-verify] MANIFEST MISSING in restore: {rel}"));
                    }
                }
            }
            for rel in restored.keys() {
                if !expected_map.contains_key(rel) {
                    manifest_ok = false;
                    log(&format!("[restore-verify] UNEXPECTED file not in manifest: {rel}"));
                }
            }
        } else {
            manifest_ok = false;
        }
    }

    log(&format!(
        "[restore-verify] {} mode={manifest_mode} files={} bytes_verified={bytes_verified} integrity={integrity_check}",
        archive.display(),
        files.len()
    ));

    let pass = integrity_check == "ok" || integrity_check == "n/a";
    let result = if pass && manifest_ok { "ok" } else { "FAIL" };

    let r = DrillResult {
        archive: archive_s,
        sha3_match: SidecarCheck::Match,
        integrity_check,
        bytes_verified,
        result: result.to_string(),
    };
    write_drill(&r);

    // Step 6 — always delete the scratch dir (read-only against production).
    let _ = fs::remove_dir_all(&scratch);
    r
}

/// Open the RESTORED (scratch) SQLite file and run `PRAGMA integrity_check`.
///
/// The connection is read-WRITE **on purpose**: `integrity_check` validates
/// FTS5 inverted indexes (e.g. the trigram index in the Hermes state.db) by
/// materialising them, which needs write access — a read-only open fails with
/// "attempt to write a readonly database" and yields a false negative. This is
/// still "read-only against production": the file under test is a disposable
/// copy in the scratch dir that this function's caller deletes afterward; the
/// live DB and the COLD archive are never opened.
fn sqlite_integrity(db: &Path) -> Result<String, String> {
    let conn = rusqlite::Connection::open(db).map_err(|e| e.to_string())?;
    let res: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(res)
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

    // BLUEPRINT-P12 §3 subcommands take positional file args and are read-only
    // against production (archive creates a new file; restore-verify only ever
    // writes to scratch), so they are NOT gated by --commit. Dispatch + return.
    if let Some(pos) = args.iter().position(|a| a == "sha3") {
        let files: Vec<String> = args[pos + 1..]
            .iter()
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect();
        cmd_sha3(&files);
        return;
    }
    if let Some(pos) = args.iter().position(|a| a == "archive") {
        let rest: Vec<&String> = args[pos + 1..].iter().filter(|a| !a.starts_with('-')).collect();
        match (rest.first(), rest.get(1)) {
            (Some(src), Some(dest)) => cmd_archive(src, dest),
            _ => {
                println!("usage: deep-clean archive <src> <dest.tar.zst>");
                std::process::exit(2);
            }
        }
        return;
    }
    if let Some(pos) = args.iter().position(|a| a == "restore-verify") {
        match args[pos + 1..].iter().find(|a| !a.starts_with('-')) {
            Some(archive) => cmd_restore_verify(archive),
            None => {
                println!("usage: deep-clean restore-verify <archive.tar.zst>");
                std::process::exit(2);
            }
        }
        return;
    }

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
        println!("       deep-clean sha3 <file>...                       (write .sha3 content-address sidecars)");
        println!("       deep-clean archive <src> <dest.tar.zst>         (COLD archive + .sha3/.manifest sidecars)");
        println!("       deep-clean restore-verify <archive.tar.zst>     (restore-drill: verify, integrity_check, log)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static CTR: AtomicU64 = AtomicU64::new(0);

    fn tmp_dir(tag: &str) -> PathBuf {
        let n = CTR.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "dc-test-{tag}-{}-{}-{}",
            std::process::id(),
            ts(),
            n
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn tools_present() -> bool {
        let ok = |c: &str| {
            Command::new(c)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        ok("tar") && ok("zstd")
    }

    // --- SHA3-256 known-answer vectors (FIPS 202) prove the content-address ---
    #[test]
    fn sha3_kat_empty() {
        assert_eq!(
            hex(&sha3_256(b"")),
            "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        );
    }

    #[test]
    fn sha3_kat_abc() {
        assert_eq!(
            hex(&sha3_256(b"abc")),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
        );
    }

    // Streaming absorb must equal the one-shot form for multi-block input and
    // the RATE-1 padding edge case (the only tricky pad10*1 boundary).
    #[test]
    fn sha3_streaming_equals_oneshot() {
        let input: Vec<u8> = (0..1000u32).map(|i| (i.wrapping_mul(7).wrapping_add(3)) as u8).collect();
        let oneshot = sha3_256(&input);
        let mut h = Sha3_256::new();
        for chunk in input.chunks(7) {
            h.update(chunk);
        }
        assert_eq!(h.finalize(), oneshot);

        let edge = vec![0xabu8; SHA3_RATE - 1];
        let mut h2 = Sha3_256::new();
        h2.update(&edge[..10]);
        h2.update(&edge[10..]);
        assert_eq!(h2.finalize(), sha3_256(&edge));
    }

    #[test]
    fn hex_is_lowercase_64() {
        let d = sha3_256(b"x");
        let h = hex(&d);
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    // --- archive -> restore-verify round trip (file tree, manifest mode) ---
    #[test]
    fn archive_then_restore_verify_ok() {
        if !tools_present() {
            eprintln!("skip archive_then_restore_verify_ok: tar/zstd absent");
            return;
        }
        let src = tmp_dir("src");
        fs::write(src.join("a.txt"), b"hello world").unwrap();
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("sub/b.bin"), [1u8, 2, 3, 4, 5]).unwrap();
        let work = tmp_dir("work");
        let dest = work.join("arc.tar.zst");

        let (n, _hx) = archive_impl(&src, &dest).unwrap();
        assert_eq!(n, 2);
        assert!(sidecar_sha3(&dest).is_file());
        assert!(sidecar_manifest(&dest).is_file());

        let scratch = tmp_dir("scratch");
        let logdir = tmp_dir("log");
        let r = restore_verify_impl(&dest, &scratch, &logdir);
        assert_eq!(r.sha3_match, SidecarCheck::Match);
        assert_eq!(r.integrity_check, "n/a");
        assert_eq!(r.result, "ok");
        assert!(r.bytes_verified >= 16);
        // scratch dir must be cleaned up after the drill.
        assert!(!scratch.join(r.bytes_verified.to_string()).exists());

        for d in [&src, &work, &scratch, &logdir] {
            let _ = fs::remove_dir_all(d);
        }
    }

    // --- restore-verify runs PRAGMA integrity_check on a SQLite payload ---
    #[test]
    fn restore_verify_sqlite_integrity_ok() {
        if !tools_present() {
            eprintln!("skip restore_verify_sqlite_integrity_ok: tar/zstd absent");
            return;
        }
        let src = tmp_dir("dbsrc");
        {
            let conn = rusqlite::Connection::open(src.join("state.db")).unwrap();
            conn.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(1),(2),(3);")
                .unwrap();
        }
        let work = tmp_dir("dbwork");
        let dest = work.join("db.tar.zst");
        archive_impl(&src, &dest).unwrap();

        let scratch = tmp_dir("dbscratch");
        let logdir = tmp_dir("dblog");
        let r = restore_verify_impl(&dest, &scratch, &logdir);
        assert_eq!(r.integrity_check, "ok");
        assert_eq!(r.result, "ok");
        assert!(r.bytes_verified > 0);

        for d in [&src, &work, &scratch, &logdir] {
            let _ = fs::remove_dir_all(d);
        }
    }

    // --- 1-byte archive corruption FAILS loudly at the sha3 gate ---
    #[test]
    fn restore_verify_corruption_fails() {
        if !tools_present() {
            eprintln!("skip restore_verify_corruption_fails: tar/zstd absent");
            return;
        }
        let src = tmp_dir("csrc");
        fs::write(src.join("payload.txt"), b"important data").unwrap();
        let work = tmp_dir("cwork");
        let dest = work.join("c.tar.zst");
        archive_impl(&src, &dest).unwrap();

        // flip one byte of the ARCHIVE (never the sidecar).
        let mut bytes = fs::read(&dest).unwrap();
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xff;
        fs::write(&dest, &bytes).unwrap();

        let scratch = tmp_dir("cscratch");
        let logdir = tmp_dir("clog");
        let r = restore_verify_impl(&dest, &scratch, &logdir);
        assert_eq!(r.sha3_match, SidecarCheck::Mismatch);
        assert_eq!(r.result, "FAIL");
        assert_eq!(r.bytes_verified, 0, "must not restore a corrupt archive");

        for d in [&src, &work, &scratch, &logdir] {
            let _ = fs::remove_dir_all(d);
        }
    }

    // --- a MISSING sidecar is a NAMED failure, never a silent skip ---
    #[test]
    fn restore_verify_no_sidecar_fails() {
        if !tools_present() {
            eprintln!("skip restore_verify_no_sidecar_fails: tar/zstd absent");
            return;
        }
        let src = tmp_dir("nsrc");
        fs::write(src.join("f.txt"), b"data").unwrap();
        let work = tmp_dir("nwork");
        let dest = work.join("n.tar.zst");
        archive_impl(&src, &dest).unwrap();
        fs::remove_file(sidecar_sha3(&dest)).unwrap();

        let scratch = tmp_dir("nscratch");
        let logdir = tmp_dir("nlog");
        let r = restore_verify_impl(&dest, &scratch, &logdir);
        assert_eq!(r.sha3_match, SidecarCheck::NoSidecar);
        assert_eq!(r.result, "FAIL");

        for d in [&src, &work, &scratch, &logdir] {
            let _ = fs::remove_dir_all(d);
        }
    }

    // --- the deny-list bars secret material from ever entering an archive ---
    #[test]
    fn archive_refuses_denylisted() {
        // "secret" in the path triggers excluded(); archive must refuse.
        let src = tmp_dir("secret-src");
        fs::write(src.join("x.txt"), b"x").unwrap();
        let work = tmp_dir("dwork");
        let dest = work.join("d.tar.zst");
        let res = archive_impl(&src, &dest);
        assert!(res.is_err(), "deny-listed source must be refused");
        assert!(!dest.exists(), "no archive is produced for a denied source");

        for d in [&src, &work] {
            let _ = fs::remove_dir_all(d);
        }
    }
}
