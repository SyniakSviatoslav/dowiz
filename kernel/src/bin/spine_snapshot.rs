//! `spine_snapshot` — wires `dowiz_kernel::spine::KnowledgeSpine` to a real
//! corpus: the project's memory/design-doc `.md` files.
//!
//! Closes a real, confirmed gap: `KnowledgeSpine` (a tamper-evident hash-chain
//! ledger) was built and tested but had ZERO callers — and separately, this
//! session's own strategic audit found the project's knowledge corpus is
//! exactly what's been drifting (a self-contradicting memory index, a design
//! doc citing a ruling that was never made, a quality claim silently
//! regressing on port). This binary is the wiring: it snapshots every `.md`
//! file under the given directories into the spine as a content-hash record,
//! and — on every subsequent run — reloads the persisted chain via
//! `KnowledgeSpine::from_persisted` (which VERIFIES before accepting), so a
//! tampered/corrupted chain file is refused outright rather than silently
//! trusted.
//!
//! What this proves: the SEQUENCE of observed content-hashes for the corpus
//! hasn't been tampered with after the fact. What this does NOT do: it is not
//! a live file-integrity monitor (it only sees drift when run), and a file
//! genuinely edited between runs is expected and recorded as a new chain
//! entry, not flagged as tampering — only editing the PERSISTED CHAIN FILE
//! itself, or mismatching the recorded hash for an unrevised entry, is.
//!
//! Usage:
//!   spine_snapshot --chain docs/ledger/corpus-spine-chain.jsonl \
//!       --dir docs/design --dir /root/.claude/projects/-root-dowiz/memory
//!   spine_snapshot --selftest

use dowiz_kernel::spine::{hash_payload, KnowledgeSpine, PendingRecord, RecordKind, SpineRecord};
use std::path::{Path, PathBuf};
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut chain_path: Option<PathBuf> = None;
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut selftest = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--chain" => {
                i += 1;
                chain_path = Some(PathBuf::from(&args[i]));
            }
            "--dir" => {
                i += 1;
                dirs.push(PathBuf::from(&args[i]));
            }
            "--selftest" => selftest = true,
            "-h" | "--help" => {
                println!(
                    "spine_snapshot — wire KnowledgeSpine to the memory/design corpus\n\
                     usage: spine_snapshot --chain FILE --dir DIR [--dir DIR ...]\n\
                     \x20      spine_snapshot --selftest"
                );
                exit(0);
            }
            other => {
                eprintln!("spine_snapshot: unknown arg `{other}`");
                exit(2);
            }
        }
        i += 1;
    }

    if selftest {
        run_selftest();
        return;
    }

    let Some(chain_path) = chain_path else {
        eprintln!("spine_snapshot: --chain FILE is required (or pass --selftest)");
        exit(2);
    };
    if dirs.is_empty() {
        eprintln!("spine_snapshot: at least one --dir is required (or pass --selftest)");
        exit(2);
    }

    let loaded = load_chain(&chain_path);
    let mut spine = match loaded {
        Ok(records) => match KnowledgeSpine::from_persisted(records) {
            Ok(s) => s,
            Err(()) => {
                eprintln!(
                    "spine_snapshot: REFUSED — persisted chain at {} failed verification \
                     (tampered, corrupted, or hand-edited). Not loading, not overwriting. \
                     Investigate before proceeding.",
                    chain_path.display()
                );
                exit(1);
            }
        },
        Err(LoadError::NotFound) => KnowledgeSpine::new(),
        Err(LoadError::Malformed(msg)) => {
            eprintln!(
                "spine_snapshot: REFUSED — chain file at {} is not valid persisted JSONL ({msg}). \
                 Not overwriting a file we can't parse.",
                chain_path.display()
            );
            exit(1);
        }
    };

    let mut files: Vec<PathBuf> = Vec::new();
    for dir in &dirs {
        walk_md_files(dir, &mut files);
    }
    files.sort();

    let mut new_or_changed = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("spine_snapshot: skip (unreadable): {}", path.display());
            continue;
        };
        let id = path.display().to_string();
        let payload_hash = hash_payload(&bytes);
        let last_known = spine
            .records()
            .iter()
            .rev()
            .find(|r| r.id == id)
            .map(|r| r.payload_hash);
        if last_known != Some(payload_hash) {
            spine.append(PendingRecord {
                id,
                kind: RecordKind::Memory,
                payload_hash,
            });
            new_or_changed += 1;
        }
    }

    if !spine.verify_chain() {
        eprintln!("spine_snapshot: INTERNAL ERROR — freshly built chain fails self-verification");
        exit(1);
    }

    if let Err(e) = save_chain(&chain_path, spine.records()) {
        eprintln!(
            "spine_snapshot: failed to write {}: {e}",
            chain_path.display()
        );
        exit(1);
    }

    println!(
        "spine_snapshot: {} files scanned, {} new/changed record(s) appended, \
         chain length {} , verify_chain() = {}",
        files.len(),
        new_or_changed,
        spine.len(),
        spine.verify_chain()
    );
}

fn walk_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip disposable/VCS/build directories some corpora may nest.
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == ".git" || name == "target" || name.starts_with('.') {
                    continue;
                }
            }
            walk_md_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

#[derive(Debug)]
enum LoadError {
    NotFound,
    Malformed(String),
}

fn load_chain(path: &Path) -> Result<Vec<SpineRecord>, LoadError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Err(LoadError::NotFound),
    };
    let mut records = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v = dowiz_kernel::json::parse(line)
            .map_err(|e| LoadError::Malformed(format!("line {}: {e}", lineno + 1)))?;
        let id = v
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or_else(|| LoadError::Malformed(format!("line {}: missing id", lineno + 1)))?
            .to_string();
        let kind_str = v
            .get("kind")
            .and_then(|x| x.as_str())
            .ok_or_else(|| LoadError::Malformed(format!("line {}: missing kind", lineno + 1)))?;
        let kind = match kind_str {
            "memory" => RecordKind::Memory,
            "identity" => RecordKind::Identity,
            "intent" => RecordKind::Intent,
            other => {
                return Err(LoadError::Malformed(format!(
                    "line {}: unknown kind `{other}`",
                    lineno + 1
                )))
            }
        };
        let payload_hash = v
            .get("payload_hash")
            .and_then(|x| x.as_str())
            .and_then(decode_hex32)
            .ok_or_else(|| {
                LoadError::Malformed(format!("line {}: bad payload_hash", lineno + 1))
            })?;
        let prev_hash = v
            .get("prev_hash")
            .and_then(|x| x.as_str())
            .and_then(decode_hex32)
            .ok_or_else(|| LoadError::Malformed(format!("line {}: bad prev_hash", lineno + 1)))?;
        let record_hash = v
            .get("record_hash")
            .and_then(|x| x.as_str())
            .and_then(decode_hex32)
            .ok_or_else(|| LoadError::Malformed(format!("line {}: bad record_hash", lineno + 1)))?;
        records.push(SpineRecord {
            id,
            kind,
            payload_hash,
            prev_hash,
            record_hash,
        });
    }
    Ok(records)
}

fn save_chain(path: &Path, records: &[SpineRecord]) -> std::io::Result<()> {
    let mut out = String::new();
    for r in records {
        let kind_str = match r.kind {
            RecordKind::Memory => "memory",
            RecordKind::Identity => "identity",
            RecordKind::Intent => "intent",
        };
        out.push_str(&format!(
            "{{\"id\":{},\"kind\":\"{}\",\"payload_hash\":\"{}\",\"prev_hash\":\"{}\",\"record_hash\":\"{}\"}}\n",
            json_escape(&r.id),
            kind_str,
            encode_hex32(&r.payload_hash),
            encode_hex32(&r.prev_hash),
            encode_hex32(&r.record_hash),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, out)
}

/// Minimal JSON string escaping sufficient for real filesystem paths
/// (quotes/backslashes; paths in this repo do not contain control chars).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn encode_hex32(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn decode_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn run_selftest() {
    let tmp = std::env::temp_dir().join(format!("spine-snapshot-selftest-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create temp dir");
    let file_a = tmp.join("a.md");
    let file_b = tmp.join("b.md");
    std::fs::write(&file_a, "content A").unwrap();
    std::fs::write(&file_b, "content B").unwrap();
    let chain_path = tmp.join("chain.jsonl");

    // Pass 1: fresh chain, both files new.
    run_one_pass(&chain_path, &[tmp.clone()]);
    let records = load_chain(&chain_path).unwrap_or_else(|_| panic!("pass 1 chain must load"));
    assert_eq!(records.len(), 2, "pass 1 should record both files");
    assert!(
        KnowledgeSpine::from_persisted(records).is_ok(),
        "pass 1 chain must verify"
    );

    // Pass 2: no changes — chain length must stay the same (no duplicate records).
    run_one_pass(&chain_path, &[tmp.clone()]);
    let records = load_chain(&chain_path).unwrap();
    assert_eq!(
        records.len(),
        2,
        "unchanged files must not append duplicate records"
    );

    // Pass 3: change file A — exactly one new record.
    std::fs::write(&file_a, "content A, revised").unwrap();
    run_one_pass(&chain_path, &[tmp.clone()]);
    let records = load_chain(&chain_path).unwrap();
    assert_eq!(
        records.len(),
        3,
        "a changed file must append exactly one new record"
    );
    assert!(
        KnowledgeSpine::from_persisted(records.clone()).is_ok(),
        "pass 3 chain must still verify"
    );

    // Pass 4: tamper with the persisted file directly, confirm refusal.
    // Flip a hex digit inside the first payload_hash occurrence.
    let mut bytes = std::fs::read(&chain_path).unwrap();
    if let Some(pos) = find_bytes(&bytes, b"\"payload_hash\":\"") {
        let digit_pos = pos + b"\"payload_hash\":\"".len();
        bytes[digit_pos] = if bytes[digit_pos] == b'0' { b'1' } else { b'0' };
    }
    std::fs::write(&chain_path, &bytes).unwrap();
    let loaded = load_chain(&chain_path).unwrap();
    assert!(
        KnowledgeSpine::from_persisted(loaded).is_err(),
        "a hand-tampered chain file must be REFUSED on load"
    );

    std::fs::remove_dir_all(&tmp).ok();
    println!("spine_snapshot --selftest: PASS (4/4 checks)");
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn run_one_pass(chain_path: &Path, dirs: &[PathBuf]) {
    let loaded = load_chain(chain_path);
    let mut spine = match loaded {
        Ok(records) => KnowledgeSpine::from_persisted(records).expect("selftest chain must verify"),
        Err(LoadError::NotFound) => KnowledgeSpine::new(),
        Err(LoadError::Malformed(m)) => panic!("selftest: malformed chain: {m}"),
    };
    let mut files = Vec::new();
    for d in dirs {
        walk_md_files(d, &mut files);
    }
    files.sort();
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let id = path.display().to_string();
        let payload_hash = hash_payload(&bytes);
        let last_known = spine
            .records()
            .iter()
            .rev()
            .find(|r| r.id == id)
            .map(|r| r.payload_hash);
        if last_known != Some(payload_hash) {
            spine.append(PendingRecord {
                id,
                kind: RecordKind::Memory,
                payload_hash,
            });
        }
    }
    save_chain(chain_path, spine.records()).unwrap();
}
