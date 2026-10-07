//! `restore_drill <bundle.json> [<stamp>.witness.json]` -- the restore drill over a REAL
//! nightly copy downloaded from a venue's bucket (W-PITR, R-BEBOPDB D.1 #10).
//!
//! A RUST BIN, NOT A NODE SCRIPT: the drill is only worth anything if it loads each image
//! through the same checked loaders the venue's object uses (`Hub::load`, `Catalog::load` =
//! `Kv::load_checked` with crc, `LogImage::load`, ...), and those are this crate. A `.mjs`
//! would be a second reader that agrees with the first only until one of them changes.
//!
//! The bucket holds `<venue>/<stamp>.json.gz.sealed` (or `.json.gz`, or `.json`). Open it first:
//!   seal-open open <secret-key> <stamp>.json.gz.sealed copy.json.gz   # only if sealed
//!   gunzip -k copy.json.gz                                             # only if gzip
//!   cargo run --release --bin restore_drill -- copy.json <stamp>.witness.json
//! Exit 0 = PASS, 1 = REFUSED (each reason printed), 2 = could not read the files.
//! It writes nothing anywhere.

use std::process::ExitCode;

use dowiz_hub::drill::{self, Tip};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(bundle), witness) = (args.first(), args.get(1)) else {
        eprintln!("usage: restore_drill <bundle.json> [<stamp>.witness.json]");
        return ExitCode::from(2);
    };
    let read = |p: &String| std::fs::read(p).map_err(|e| eprintln!("restore_drill: {p}: {e}"));
    let Ok(b) = read(bundle) else { return ExitCode::from(2) };
    if b.starts_with(&[0x1f, 0x8b]) {
        eprintln!("restore_drill: {bundle} is gzip; run `gunzip -k` on it first");
        return ExitCode::from(2);
    }
    let w = match witness.map(read) {
        Some(Err(())) => return ExitCode::from(2),
        Some(Ok(w)) => Some(w),
        None => None,
    };
    let d = drill::run(&b, w.as_deref());
    println!("venue {} taken_at_ms {}", d.venue, d.taken_at_ms);
    for l in &d.loaded {
        println!("  loaded {:<16} {:>9} bytes{}", l.id, l.bytes, l.records.map(|n| format!("  {n} records")).unwrap_or_default());
    }
    if let Some(c) = d.chain {
        println!("  chain  {} records: {} chained, {} legacy, {} redacted, {} broken", c.records, c.chained, c.legacy, c.redacted, c.broken);
    }
    let tip = |t: &Option<String>| t.as_deref().map(|s| s[..s.len().min(16)].to_string()).unwrap_or_else(|| "-".into());
    println!("  tip    log {} witness {} -> {}", tip(&d.log_tip), tip(&d.witness_tip), match d.tip {
        Tip::NoWitness => "no witness given",
        Tip::Equal => "EQUAL",
        Tip::Held => "held, with records after it",
        Tip::Missing => "MISSING",
    });
    for n in &d.notes {
        println!("  note   {n}");
    }
    for r in &d.refused {
        println!("  REFUSED {r}");
    }
    if d.passed() {
        println!("restore-drill: PASS");
        ExitCode::SUCCESS
    } else {
        println!("restore-drill: REFUSED ({} reason(s))", d.refused.len());
        ExitCode::from(1)
    }
}
