//! `anchor` (W-MG-M6b): the operator's door to level-1 anchors (R-MAINGOAL-A §3, §7 row M6).
//!
//!     anchor stamp <path> [--calendars <url,url,...>] [--out <file.ots>]   feature `anchor-live`
//!     anchor upgrade <path>                                                 feature `anchor-live`
//!     anchor verify <path> [--headers <file>] [--platform-key <hex pk>]...
//!     anchor fetch-headers <height>... --out <file> [--explorer <api>]      feature `anchor-live`
//!
//! `<path>` is a sealed hub image (its TIP is anchored: one ANCH written into the image, the
//! proof also left at `<path>.ots`) or any other file (its sha256 is anchored, the proof left at
//! `<path>.ots`). `verify` prints the counters, every anchor's verdict and the three sentences of
//! §2d VERBATIM; it reads the file plus what the auditor brings and calls no service. A fresh
//! stamp is PENDING and this tool never says otherwise; an upgrade reports an attestation as
//! PRESENT, and only `verify --headers` says it checks.
//!
//! Exit: 0 holds / done; 2 the input was refused (named); 3 does not hold or an anchor is bad;
//! 4 still pending, nothing written; 5 built without `anchor-live` or the network failed --
//! nothing written.

use bebop_store::evlog::doc::Verdict;
use dowiz_hub::doc_seal::live::{describe, verify_image, verify_ots, FileHeaders};
use dowiz_hub::doc_seal::{hex32, ots, SENTENCES};

const USAGE: &str = "usage: anchor stamp <path> [--calendars a,b] [--out f.ots] | upgrade <path> | verify <path> [--headers f] [--platform-key hex]... | fetch-headers <h>... --out f [--explorer api]";

struct Args {
    cmd: String,
    pos: Vec<String>,
    opts: Vec<(String, String)>,
}

impl Args {
    fn parse(argv: Vec<String>) -> Args {
        let mut a = Args { cmd: argv.first().cloned().unwrap_or_default(), pos: Vec::new(), opts: Vec::new() };
        let mut i = 1;
        while i < argv.len() {
            if let Some(flag) = argv[i].strip_prefix("--") {
                i += 1;
                let Some(v) = argv.get(i) else { die(2, &format!("--{flag} wants a value")) };
                a.opts.push((flag.to_string(), v.clone()));
            } else {
                a.pos.push(argv[i].clone());
            }
            i += 1;
        }
        a
    }
    fn opt(&self, name: &str) -> Option<&str> {
        self.opts.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    fn all(&self, names: &[&str]) -> Vec<String> {
        self.opts.iter().filter(|(k, _)| names.contains(&k.as_str())).map(|(_, v)| v.clone()).collect()
    }
    fn path(&self) -> &str {
        self.pos.first().map(String::as_str).unwrap_or_else(|| die(2, USAGE))
    }
}

fn main() {
    let a = Args::parse(std::env::args().skip(1).collect());
    match a.cmd.as_str() {
        "stamp" => stamp(&a),
        "upgrade" => upgrade(&a),
        "verify" => verify(&a),
        "fetch-headers" => fetch_headers(&a),
        _ => die(2, USAGE),
    }
}

fn read(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| die(2, &format!("{path}: {e}")))
}

#[cfg(feature = "anchor-live")]
fn write(path: &str, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap_or_else(|e| die(5, &format!("{path}: {e} -- NOT written")));
}

fn headers(a: &Args) -> Option<FileHeaders> {
    a.opt("headers").map(|p| {
        let text = std::fs::read_to_string(p).unwrap_or_else(|e| die(2, &format!("--headers {p}: {e}")));
        FileHeaders::parse(&text).unwrap_or_else(|e| die(2, &format!("--headers {p}: {e}")))
    })
}

/// The verifier: from the file plus what the auditor brings. No feature, no network.
fn verify(a: &Args) {
    let path = a.path();
    let bytes = read(path);
    let h = headers(a);
    let keys: Vec<Vec<u8>> = a
        .all(&["platform-key", "platform-pk"])
        .iter()
        .map(|k| dowiz_hub::crypto::unhex(k.trim()).unwrap_or_else(|| die(2, "--platform-key wants a hex ML-DSA-65 public key")))
        .collect();
    let hd = h.as_ref().map(|h| h as &dyn ots::BlockHeaders);
    if let Ok(p) = ots::parse(&bytes) {
        let v = verify_ots(&bytes, hd);
        let (word, why) = describe(&v);
        println!("ots digest={} -> {word}: {why}", dowiz_hub::crypto::hex(&p.digest));
        println!("what the proof says (R-MAINGOAL-A §2d):");
        println!("  {}", SENTENCES[1]);
        println!("  {}", SENTENCES[3]);
        std::process::exit(match v {
            Verdict::Ok { .. } => 0,
            Verdict::Bad(_) => 3,
            Verdict::Unchecked(_) => 4,
        });
    }
    let out = verify_image(&bytes, &keys, hd).unwrap_or_else(|e| die(2, &format!("{path}: refused: {e:?}")));
    let c = &out.report.counters;
    let ac = &out.counts;
    println!(
        "doc records={} chained={} legacy={} redacted={} broken={} seals_ok={} seals_bad={} forks={} keys={} admitted={} anchors={} anchors_ok={} anchors_bad={} anchors_unchecked={}",
        c.records, c.chained, c.legacy, c.redacted, c.broken, c.seals_ok, c.seals_bad, c.forks, out.report.keys, out.report.admitted, ac.anchors, ac.ok, ac.bad, ac.unchecked
    );
    for hd in &out.report.heads {
        println!("head actor={} seq={} tip={}", hex32(&hd.actor_id), hd.seq, hex32(&hd.tip));
    }
    for x in &out.checked {
        let (word, why) = describe(&x.verdict);
        println!("anchor kind={} over={} time_ms={} names_record={} -> {word}: {why}", x.anchor.kind, hex32(&x.anchor.over), x.anchor.time_ms, x.names_record);
    }
    println!("what the proof says (R-MAINGOAL-A §2d):");
    for s in SENTENCES {
        println!("  {s}");
    }
    println!("{}", if out.holds() { "HOLDS" } else { "DOES NOT HOLD" });
    std::process::exit(if out.holds() { 0 } else { 3 });
}

#[cfg(not(feature = "anchor-live"))]
fn offline() -> ! {
    die(5, "built without feature `anchor-live`: no network, nothing written. Build with `cargo build --bin anchor --features anchor-live`")
}

#[cfg(not(feature = "anchor-live"))]
fn stamp(_: &Args) {
    offline()
}
#[cfg(not(feature = "anchor-live"))]
fn upgrade(_: &Args) {
    offline()
}
#[cfg(not(feature = "anchor-live"))]
fn fetch_headers(_: &Args) {
    offline()
}

#[cfg(feature = "anchor-live")]
mod live {
    use super::*;
    use bebop_store::nodekey::sha256;
    use dowiz_hub::doc_seal::live::curl::{fetch_block_json, CurlCalendar, EXPLORER};
    use dowiz_hub::doc_seal::SealedHub;
    use dowiz_hub::doc_seal::live::stamp::{stamp_image, stamp_multi, upgrade_file, upgrade_image, Stamped, Upgraded};
    use dowiz_hub::doc_seal::live::header_line_from_explorer;
    use dowiz_hub::HubError;

    fn now_ms() -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
    }

    fn report(st: &Stamped, out: &str) {
        for (c, r) in &st.replies {
            match r {
                Ok(uri) => println!("calendar {c}: accepted; pending at {uri}"),
                Err(e) => println!("calendar {c}: FAILED: {e}"),
            }
        }
        println!("saved {out} ({} bytes) -- PENDING: no Bitcoin attestation yet; run `anchor upgrade {out}` in a few hours", st.ots.len());
    }

    pub(super) fn stamp(a: &Args) {
        let path = a.path();
        let bytes = read(path);
        let cals: Vec<String> = a.opt("calendars").map(|c| c.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_else(|| ots::CALENDARS.iter().map(|s| s.to_string()).collect());
        let cals: Vec<&str> = cals.iter().map(String::as_str).collect();
        let out = a.opt("out").map(str::to_string).unwrap_or_else(|| format!("{path}.ots"));
        let cal = CurlCalendar::default();
        match SealedHub::load(&bytes) {
            Ok(sh) => {
                println!("sealed image: anchoring tip {}", hex32(&sh.tip().unwrap_or([0; 32])));
                let (img, st) = stamp_image(&bytes, &cal, &cals, now_ms()).unwrap_or_else(|e| die(5, &format!("{e} -- nothing written")));
                write(path, &img);
                write(&out, &st.ots);
                println!("one ANCH (kind 2) written into {path}");
                report(&st, &out);
            }
            Err(HubError::NotAHub) => {
                let d = sha256(&bytes);
                println!("plain file: anchoring sha256 {}", hex32(&d));
                let st = stamp_multi(&cal, &cals, &d).unwrap_or_else(|e| die(5, &format!("{e} -- nothing written")));
                write(&out, &st.ots);
                report(&st, &out);
            }
            Err(e) => die(2, &format!("{path}: refused: {e:?}")),
        }
    }

    pub(super) fn upgrade(a: &Args) {
        let path = a.path();
        let bytes = read(path);
        let cal = CurlCalendar::default();
        if SealedHub::load(&bytes).is_ok() {
            let (img, lines) = upgrade_image(&bytes, &cal).unwrap_or_else(|e| die(5, &format!("{e} -- nothing written")));
            for l in &lines {
                println!("{l}");
            }
            match img {
                Some(img) => {
                    write(path, &img);
                    println!("{path} written; verify with `anchor verify {path} --headers <file>`");
                }
                None => {
                    println!("nothing new; {path} unchanged");
                    std::process::exit(4);
                }
            }
            return;
        }
        if ots::parse(&bytes).is_err() {
            die(2, &format!("{path}: neither a sealed image nor an .ots proof"));
        }
        match upgrade_file(&cal, &bytes).unwrap_or_else(|e| die(5, &format!("{e} -- nothing written"))) {
            Upgraded::Attested { ots: new, heights, still_pending } => {
                if new != bytes {
                    write(path, &new);
                }
                println!("attestation PRESENT for Bitcoin block(s) {heights:?}; still pending at [{}]", still_pending.join(","));
                println!("not verified until `anchor verify {path} --headers <file>` (fetch-headers {})", heights.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(" "));
            }
            Upgraded::Pending { uris } => {
                println!("still pending at [{}]; {path} unchanged", uris.join(","));
                std::process::exit(4);
            }
        }
    }

    pub(super) fn fetch_headers(a: &Args) {
        let out = a.opt("out").unwrap_or_else(|| die(2, "fetch-headers wants --out <file>"));
        let explorer = a.opt("explorer").unwrap_or(EXPLORER);
        if a.pos.is_empty() {
            die(2, "fetch-headers wants at least one block height");
        }
        let mut text = std::fs::read_to_string(out).unwrap_or_else(|_| format!("# <height> <merkle_root hex, raw header byte order> <time_ms> -- written by `anchor fetch-headers` from {explorer}\n"));
        for h in &a.pos {
            let height: u64 = h.parse().unwrap_or_else(|_| die(2, &format!("{h}: not a block height")));
            let json = fetch_block_json(explorer, height, 30).unwrap_or_else(|e| die(5, &format!("{e} -- nothing written")));
            let line = header_line_from_explorer(height, &json).unwrap_or_else(|e| die(5, &format!("{e} -- nothing written")));
            println!("{line}");
            text.push_str(&line);
            text.push('\n');
        }
        write(out, text.as_bytes());
        println!("saved {out}");
    }
}

#[cfg(feature = "anchor-live")]
use live::{fetch_headers, stamp, upgrade};

fn die(code: i32, msg: &str) -> ! {
    eprintln!("anchor: {msg}");
    std::process::exit(code)
}
