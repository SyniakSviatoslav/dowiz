//! `doc_verify` (W-MG-M1B): the auditor's reading of a sealed hub image, from the file plus
//! the public keys and block headers the auditor brings -- never a live service.
//!
//!     doc_verify <image> [--platform-pk <hex 1952 B>]... [--headers <file>] [--trust <actor hex>]...
//!
//! `--headers`: lines of `<height> <merkle_root_hex 32 B, raw header byte order> <time_ms>`.
//! Prints the eight counters, the heads, every anchor with its verdict, then the three
//! sentences of R-MAINGOAL-A §2d VERBATIM and what the proof does not say. Exit 0 when the
//! report holds and no anchor is bad; 3 when it does not; 2 when the file is not a sealed
//! image (the refusal named).

use dowiz_hub::doc_seal::ots::{BlockHeaders, Ots};
use dowiz_hub::doc_seal::roughtime::Roughtime;
use dowiz_hub::doc_seal::tsa::PlatformTsa;
use dowiz_hub::doc_seal::{hex32, SealedHub, SENTENCES};
use dowiz_hub::HubError;
use bebop_store::evlog::doc::{Anchor, AnchorCheck, AnchorCounts, Trust, Verdict, KIND_OTS, KIND_PLATFORM_TSA, KIND_ROUGHTIME};

struct Headers {
    rows: Vec<(u64, [u8; 32], i64)>,
}
impl BlockHeaders for Headers {
    fn merkle_root(&self, height: u64) -> Option<[u8; 32]> {
        self.rows.iter().find(|r| r.0 == height).map(|r| r.1)
    }
    fn time_ms(&self, height: u64) -> Option<i64> {
        self.rows.iter().find(|r| r.0 == height).map(|r| r.2)
    }
}

/// One checker over the three kinds: each kind to its adapter.
struct All<'a> {
    tsa: PlatformTsa<'a>,
    ots: Ots<'a>,
}
impl AnchorCheck for All<'_> {
    fn check(&self, a: &Anchor) -> Verdict {
        match a.kind {
            KIND_PLATFORM_TSA => self.tsa.check(a),
            KIND_OTS => self.ots.check(a),
            KIND_ROUGHTIME => Roughtime.check(a),
            k => Verdict::Unchecked(format!("no adapter for anchor kind {k}")),
        }
    }
}

fn unhex32(s: &str) -> Option<[u8; 32]> {
    let v = dowiz_hub::crypto::unhex(s.trim())?;
    v.try_into().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut image = None;
    let mut pks: Vec<Vec<u8>> = Vec::new();
    let mut headers = Headers { rows: Vec::new() };
    let mut trust: Vec<[u8; 32]> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--platform-pk" => {
                i += 1;
                match args.get(i).and_then(|h| dowiz_hub::crypto::unhex(h)) {
                    Some(pk) => pks.push(pk),
                    None => die(2, "--platform-pk wants a hex public key"),
                }
            }
            "--headers" => {
                i += 1;
                let Some(path) = args.get(i) else { die(2, "--headers wants a file") };
                let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(2, &format!("--headers {path}: {e}")));
                for line in text.lines().filter(|l| !l.trim().is_empty()) {
                    let f: Vec<&str> = line.split_whitespace().collect();
                    let row: Option<(u64, [u8; 32], i64)> = (|| Some((f.first()?.parse().ok()?, unhex32(f.get(1)?)?, f.get(2)?.parse().ok()?)))();
                    match row {
                        Some(r) => headers.rows.push(r),
                        None => die(2, &format!("--headers: cannot read line {line:?}")),
                    }
                }
            }
            "--trust" => {
                i += 1;
                match args.get(i).and_then(|h| unhex32(h)) {
                    Some(a) => trust.push(a),
                    None => die(2, "--trust wants a 32-byte actor id in hex"),
                }
            }
            p if image.is_none() => image = Some(p.to_string()),
            other => die(2, &format!("unknown argument {other}")),
        }
        i += 1;
    }
    let Some(path) = image else { die(2, "usage: doc_verify <image> [--platform-pk <hex>]... [--headers <file>] [--trust <hex>]...") };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| die(2, &format!("{path}: {e}")));
    let sh = match SealedHub::load(&bytes) {
        Ok(sh) => sh,
        Err(HubError::NotAHub) => die(2, "not a sealed image: no superblock or the root is not a DOC (root-version 5)"),
        Err(HubError::UnknownVersion(v)) => die(2, &format!("root-version {v} is not one this reader knows")),
        Err(e) => die(2, &format!("refused: {e:?}")),
    };
    let rep = if trust.is_empty() { sh.verify() } else { sh.verify_trusting(Trust::Only(&trust)) };
    let rep = rep.unwrap_or_else(|e| die(2, &format!("verify refused: {e:?}")));
    let c = rep.counters;
    let checked = sh
        .anchors(&All { tsa: PlatformTsa { keys: &pks }, ots: Ots { headers: if headers.rows.is_empty() { None } else { Some(&headers) } } })
        .unwrap_or_else(|e| die(2, &format!("anchors refused: {e:?}")));
    let ac = AnchorCounts::of(&checked);
    println!(
        "doc records={} chained={} legacy={} redacted={} broken={} seals_ok={} seals_bad={} forks={} keys={} admitted={} anchors={} anchors_ok={} anchors_bad={} anchors_unchecked={}",
        c.records, c.chained, c.legacy, c.redacted, c.broken, c.seals_ok, c.seals_bad, c.forks, rep.keys, rep.admitted, ac.anchors, ac.ok, ac.bad, ac.unchecked
    );
    for h in &rep.heads {
        println!("head actor={} seq={} tip={}", hex32(&h.actor_id), h.seq, hex32(&h.tip));
    }
    for x in &checked {
        let a = &x.anchor;
        let (word, why) = match &x.verdict {
            Verdict::Ok { not_after_ms, not_before_ms, what } => ("ok", format!("{what}; not_after_ms={not_after_ms:?} not_before_ms={not_before_ms:?}")),
            Verdict::Bad(w) => ("BAD", w.clone()),
            Verdict::Unchecked(w) => ("unchecked", w.clone()),
        };
        println!("anchor kind={} over={} time_ms={} names_record={} -> {word}: {why}", a.kind, hex32(&a.over), a.time_ms, x.names_record);
    }
    println!("what the proof says (R-MAINGOAL-A §2d):");
    for s in SENTENCES {
        println!("  {s}");
    }
    let holds = rep.holds() && ac.bad == 0;
    println!("{}", if holds { "HOLDS" } else { "DOES NOT HOLD" });
    std::process::exit(if holds { 0 } else { 3 });
}

fn die(code: i32, msg: &str) -> ! {
    eprintln!("doc_verify: {msg}");
    std::process::exit(code)
}
