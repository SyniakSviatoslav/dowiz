//! `doc_verify` (W-MG-M1B): the auditor's reading of a sealed hub image, from the file plus
//! the public keys and block headers the auditor brings -- never a live service.
//!
//!     doc_verify <image> [--platform-pk <hex 1952 B>]... [--headers <file>] [--trust <actor hex>]...
//!
//! `--headers`: lines of `<height> <merkle_root_hex 32 B, raw header byte order> <time_ms>`, the
//! file `anchor fetch-headers` writes (`#` comments included).
//! Prints the counters, the heads, every anchor with its verdict and `under_cut`, every MERGE with
//! its conflicts in words and whether they re-derive, then the three sentences of R-MAINGOAL-A
//! §2d VERBATIM and what the proof does not say. Exit 0 when the report holds, no anchor is bad
//! and every conflict list re-derives; 3 when it does not hold or one differs; 4 when all of that
//! stands and a conflict list could not be asked, because the file was cut (`NOT CHECKED`: never
//! read as clean, a cut of one record would launder a lie); 2 when the file is not a sealed image
//! (the refusal named).

use einstore::evlog::doc::{AnchorCounts, Trust};
use dowiz_hub::doc_seal::live::{anchor_line, doc_line, refusal_words, Adapters, FileHeaders};
use dowiz_hub::doc_seal::merge::merge_report;
use dowiz_hub::doc_seal::ots::BlockHeaders;
use dowiz_hub::doc_seal::{hex32, SealedHub, SENTENCES};
use dowiz_hub::HubError;

fn unhex32(s: &str) -> Option<[u8; 32]> {
    let v = dowiz_hub::crypto::unhex(s.trim())?;
    v.try_into().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut image = None;
    let mut pks: Vec<Vec<u8>> = Vec::new();
    let mut headers: Option<FileHeaders> = None;
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
                headers = Some(FileHeaders::parse(&text).unwrap_or_else(|e| die(2, &format!("--headers {path}: {e}"))));
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
        Err(e) => die(2, &refusal_words(&e)),
    };
    let rep = if trust.is_empty() { sh.verify() } else { sh.verify_trusting(Trust::Only(&trust)) };
    let rep = rep.unwrap_or_else(|e| die(2, &refusal_words(&e)));
    let hd = headers.as_ref().map(|h| h as &dyn BlockHeaders);
    let checked = sh.anchors(&Adapters { keys: &pks, headers: hd }).unwrap_or_else(|e| die(2, &refusal_words(&e)));
    let ac = AnchorCounts::of(&checked);
    println!("{}", doc_line(&rep, &ac));
    for h in &rep.heads {
        println!("head actor={} seq={} tip={}", hex32(&h.actor_id), h.seq, hex32(&h.tip));
    }
    for x in &checked {
        println!("{}", anchor_line(x));
    }
    let (merges, recheck_clean) = merge_report(&sh, rep.holds());
    for l in &merges {
        println!("{l}");
    }
    println!("what the proof says (R-MAINGOAL-A §2d):");
    for s in SENTENCES {
        println!("  {s}");
    }
    let said = |word: &str| merges.iter().any(|l| l.starts_with("merge ") && l.ends_with(word));
    let (unasked, differs) = (said("recheck=unchecked"), said("recheck=DIFFERS"));
    if rep.holds() && ac.bad == 0 && unasked && !differs {
        println!("NOT CHECKED");
        std::process::exit(4);
    }
    let holds = rep.holds() && ac.bad == 0 && recheck_clean;
    println!("{}", if holds { "HOLDS" } else { "DOES NOT HOLD" });
    std::process::exit(if holds { 0 } else { 3 });
}

fn die(code: i32, msg: &str) -> ! {
    eprintln!("doc_verify: {msg}");
    std::process::exit(code)
}
