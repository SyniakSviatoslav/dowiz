//! The limits of what `holds` covers, measured: anchors, SPEC, DOC flags, whole files.
use super::*;
use einstore::evlog::doc::spec::SpecIn;
use einstore::evlog::doc::{Doc, Signer, FLAG_SELF_DESCRIBING, FLAG_SHA256_IDS, KIND_PLATFORM_TSA};
use einstore::Store;
use dowiz_hub::doc_seal::live::verify_image;
use dowiz_hub::doc_seal::tsa;

/// LIMITS of what `holds` covers, measured.
#[test]
fn anchors_spec_flags_and_whole_files() {
    let (a, _b, bytes) = base(1);
    let platform = key(0x42);
    let roots = [a.id()];
    let keys = [platform.public_key().to_vec()];
    let mut sh = SealedHub::load(&bytes).unwrap();
    let tip = sh.tip().unwrap();
    sh.anchor(KIND_PLATFORM_TSA, &tip, 1_730_000_000_000, &tsa::mark(&platform, &tip, 1_730_000_000_000)).unwrap();
    let anchored = sh.to_bytes_trimmed();
    let o = verify_image(&anchored, &keys, None).unwrap();
    assert_eq!((o.counts.ok, o.counts.bad), (1, 0));

    let mut x = read(&anchored);
    x.anchors.clear();
    let o = verify_image(&write(&x), &keys, None).unwrap();
    println!("ATK anch-remove LIMIT: anchors={} outcome_holds={}", o.counts.anchors, o.holds());
    assert!(o.holds() && o.counts.anchors == 0);

    // An anchor from ANOTHER file: bad in an uncut file, only `unchecked` in a cut one.
    let mut x = read(&anchored);
    x.anchors[0].over = [7; 32];
    let f = verify_image(&write(&x), &keys, None).unwrap();
    println!("ATK anch-over-unknown: bad={} ok={}", f.counts.bad, f.counts.ok);
    assert_eq!(f.counts.bad, 1);
    let mut other = SealedHub::create(&a).unwrap();
    other.set_entropy([9; 32]);
    orders(&mut other, &a, "q", 3);
    other.cut(2, &a, 5).unwrap();
    let mut x = read(&other.to_bytes_trimmed());
    x.anchors.push(einstore::evlog::doc::Anchor { at: 0, kind: KIND_PLATFORM_TSA, over: [7; 32], time_ms: 5, prev: None, proof: tsa::mark(&platform, &[7; 32], 5) });
    let o = verify_image(&write(&x), &keys, None).unwrap();
    println!("ATK anch-moved-into-cut-file LIMIT: ok={} bad={} unchecked={} cut={} outcome_holds={}", o.counts.ok, o.counts.bad, o.counts.unchecked, o.counts.cut, o.holds());
    assert!(o.holds() && o.counts.bad == 0 && o.counts.unchecked == 1);

    // SPEC: the embedded reader swapped, its sha claim with it: `verify` never reads it.
    let wasm = b"\0asm evil";
    let spec = SpecIn { wasm, wasm_sha256: einstore::nodekey::sha256(wasm), text: b"lies", abi: 2 };
    let mut arch = Store::create_bytes(1 << 20).unwrap();
    Doc::archive_bytes(&Store::from_bytes(&bytes), &mut arch, &spec).unwrap();
    let r = only("spec-swapped LIMIT", &arch.to_bytes_trimmed(), &roots).unwrap();
    assert!(r.holds());

    // DOC flags: bit 0 and bit 2 flipped, nothing reads them.
    let mut x = read(&bytes);
    x.flags &= !FLAG_SHA256_IDS;
    x.flags |= FLAG_SELF_DESCRIBING;
    stays("doc-flags-flipped INERT", &x, &roots);

    // SWAP a whole file: no seal names a file. Two hubs of one owner both hold for that owner.
    let (_, _, other_file) = base(2);
    let r = only("file-swapped LIMIT", &other_file, &roots).unwrap();
    assert!(r.holds());
    // SWAP one device's chain between them: its first record chains onto the other file.
    let (mut s, o) = (read(&bytes), read(&other_file));
    let b_id = key(0xB2).id();
    let keep = |r: &&Record| r.actor_pubkey != b_id;
    let graft: Vec<Record> = o.recs.iter().filter(|r| r.actor_pubkey == b_id).cloned().collect();
    s.recs = s.recs.iter().filter(keep).cloned().chain(graft).collect();
    let i = s.chains.iter().position(|c| c[0].actor_id == b_id).unwrap();
    s.chains[i] = o.chains.iter().find(|c| c[0].actor_id == b_id).unwrap().clone();
    caught("chain-swap-between-files", &s, &roots, |r| r.counters.unsealed >= 1);
    let _ = a.alg();
}
