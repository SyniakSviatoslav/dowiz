//! `maingoal_device` (W-MG-M7A): the DEVICE of `tools/gates/maingoal.sh` -- a real file, written by
//! the real library, with real ML-DSA-65 keys. Nothing here is a test double: a `Hub` is written
//! the way the order log writes it (one legacy record, then sha256), sealed, and carries on as a
//! `SealedHub`: orders, a platform mark, the person forgotten, the oldest records cut, a mark over
//! the new tip. The keys are derived from `--seed`, so one seed is one set of bytes: the gate's
//! fixture `crates/bebop-wasm/fixtures/anch.store` is this program at seed 0^32, and the gate's
//! scenarios run on a fresh random seed. The platform key here is the run's own: NO platform mark
//! is issued by anyone in production (`doc_seal/live.rs`).
//!
//!     maingoal_device <out dir> --seed <hex 32 B> --reader <reader.wasm> --format <FORMAT.md>
//!
//! Writes into <out dir>:
//!   before-forget.sealed   the image before the erasure (the gate's control: the phone IS there)
//!   history.sealed         what `cut` gave back: the whole image as it stood before the cut
//!   hub.sealed             the working file: 4 records, 1 segment, 2 anchors
//!   archive.store          hub.sealed with SPEC first: the reader and the format text inside
//!   revoke.sealed          a second device admitted, one seal, REVOKEd after it: holds, and the
//!                          gate zeroes `revoked_at` in it
//!   legacy-forget.sealed   a REAL tombstone over a LEGACY record: `Hub`'s legacy id, `forget`'s kind bit
//!   device.pk device.actor platform.pk      hex; the public half, all an auditor is given
//!
//! Exit 0 written; 2 bad arguments; 1 the library refused (named).

use bebop_store::evlog::doc::{Doc, KeyOp, SpecIn, KIND_PLATFORM_TSA};
use bebop_store::nodekey::sha256;
use bebop_store::Store;
use dowiz_hub::crypto::{hex, unhex};
use dowiz_hub::doc_seal::{hex32, tsa, DeviceKey, SealedHub};
use dowiz_hub::{Event, EventKind, Hub};

/// The person the gate forgets; `tools/gates/maingoal.sh` greps the files for this exact text.
const PHONE: &str = "+355691111111";
const PLAIN: &str = r#"{"status":"new","total":{"amount":1500,"currency":"ALL"}}"#;
/// `claimed_ms` of the first commit: the writer's claim, which no proof vouches for (§2d).
const T0: i64 = 1_761_900_000_000;
/// The reader ABI `bebop-verify.mjs` asks for (`bw_version`).
const READER_ABI: i64 = 2;

fn order(id: &str) -> String {
    format!(r#"{{"id":"{id}","status":"new","total":1800,"contact":{{"name":"N {id}","phone":"{PHONE}"}}}}"#)
}

fn redact(e: &Event) -> Option<String> {
    e.order_json.contains(PHONE).then(|| e.order_json.replace(PHONE, ""))
}

fn tagged(tag: &str, seed: &[u8; 32]) -> [u8; 32] {
    let mut m = tag.as_bytes().to_vec();
    m.extend_from_slice(seed);
    sha256(&m)
}

struct Args {
    out: String,
    seed: [u8; 32],
    wasm: Vec<u8>,
    text: Vec<u8>,
}

fn args() -> Result<Args, String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: maingoal_device <out dir> --seed <hex 32 B> --reader <reader.wasm> --format <FORMAT.md>";
    let (mut out, mut seed, mut wasm, mut text) = (None, None, None, None);
    let mut i = 0;
    while i < argv.len() {
        let v = argv.get(i + 1);
        match argv[i].as_str() {
            "--seed" => seed = v.and_then(|h| unhex(h)).and_then(|b| <[u8; 32]>::try_from(b).ok()),
            "--reader" => wasm = v.map(|p| std::fs::read(p).map_err(|e| format!("{p}: {e}"))).transpose()?,
            "--format" => text = v.map(|p| std::fs::read(p).map_err(|e| format!("{p}: {e}"))).transpose()?,
            p if out.is_none() => {
                out = Some(p.to_string());
                i += 1;
                continue;
            }
            other => return Err(format!("unknown argument {other}; {usage}")),
        }
        i += 2;
    }
    match (out, seed, wasm, text) {
        (Some(out), Some(seed), Some(wasm), Some(text)) => Ok(Args { out, seed, wasm, text }),
        _ => Err(usage.to_string()),
    }
}

/// A hub as the order log writes it: one record under the legacy scheme, five under sha256.
fn real_hub(seed: [u8; 32]) -> Result<Vec<u8>, String> {
    let mut hub = Hub::create_sized(64 * 1024).map_err(|e| format!("{e:?}"))?;
    hub.set_entropy(seed);
    hub.set_sha256(false);
    hub.append(EventKind::Placed, "old", PLAIN, 1, [0u8; 32]).map_err(|e| format!("{e:?}"))?;
    hub.set_sha256(true);
    for i in 1..=5u64 {
        hub.append(EventKind::Placed, &format!("o{i}"), PLAIN, i + 1, [0u8; 32]).map_err(|e| format!("{e:?}"))?;
    }
    Ok(hub.to_bytes_trimmed())
}

/// One owner, one admitted device with one seal, and the owner's REVOKE after that seal.
fn revoked_device(a: &Args, owner: &DeviceKey) -> Result<Vec<u8>, String> {
    let er = |e: dowiz_hub::HubError| format!("{e:?}");
    let second = DeviceKey::from_seed(&tagged("maingoal/second-device", &a.seed));
    let mut sh = SealedHub::create(owner).map_err(er)?;
    sh.set_entropy(tagged("maingoal/revoke-entropy", &a.seed));
    sh.commit_keyop(&KeyOp::Admit(second.key()), owner, T0).map_err(er)?;
    sh.append(EventKind::Placed, "r1", PLAIN, &second, T0 + 1000).map_err(er)?;
    sh.commit_keyop(&KeyOp::Revoke { actor_id: second.id(), after_seq: 1 }, owner, T0 + 2000).map_err(er)?;
    Ok(sh.to_bytes_trimmed())
}

/// A legacy record holding the person, one sha256 record, sealed, the person forgotten.
fn legacy_forgotten(a: &Args, device: &DeviceKey) -> Result<Vec<u8>, String> {
    let er = |e: dowiz_hub::HubError| format!("{e:?}");
    let mut hub = Hub::create_sized(64 * 1024).map_err(er)?;
    hub.set_entropy(tagged("maingoal/legacy-entropy", &a.seed));
    hub.set_sha256(false);
    hub.append(EventKind::Placed, "old", &order("old"), 1, [0u8; 32]).map_err(er)?;
    hub.set_sha256(true);
    hub.append(EventKind::Placed, "o1", PLAIN, 2, [0u8; 32]).map_err(er)?;
    let mut sh = SealedHub::seal_hub(&hub.to_bytes_trimmed(), device).map_err(er)?;
    sh.set_entropy(tagged("maingoal/legacy-sealed", &a.seed));
    let n = sh.forget(redact, &sha256(PHONE.as_bytes()), device, T0).map_err(er)?;
    if n != 1 {
        return Err(format!("forget redacted {n} records, the legacy scenario has 1"));
    }
    Ok(sh.to_bytes_trimmed())
}

fn write(a: &Args, name: &str, bytes: &[u8]) -> Result<(), String> {
    let path = format!("{}/{name}", a.out);
    std::fs::write(&path, bytes).map_err(|e| format!("{path}: {e}"))?;
    println!("device: wrote {name} ({} bytes)", bytes.len());
    Ok(())
}

fn run(a: &Args) -> Result<(), String> {
    let er = |e: dowiz_hub::HubError| format!("{e:?}");
    let device = DeviceKey::from_seed(&tagged("maingoal/device", &a.seed));
    let platform = DeviceKey::from_seed(&tagged("maingoal/platform", &a.seed));

    // (a) THE DEVICE WRITES AND SEALS: real order records, one seal per commit.
    let mut sh = SealedHub::seal_hub(&real_hub(tagged("maingoal/hub-entropy", &a.seed))?, &device).map_err(er)?;
    sh.set_entropy(tagged("maingoal/entropy", &a.seed));
    sh.append(EventKind::Placed, "p1", &order("p1"), &device, T0).map_err(er)?;
    sh.append(EventKind::Placed, "p2", &order("p2"), &device, T0 + 1000).map_err(er)?;
    sh.append(EventKind::Placed, "o7", PLAIN, &device, T0 + 2000).map_err(er)?;
    let early = sh.tip().ok_or("no tip after three commits")?;
    sh.anchor(KIND_PLATFORM_TSA, &early, T0 + 2500, &tsa::mark(&platform, &early, T0 + 2500)).map_err(er)?;
    sh.append(EventKind::Placed, "o8", PLAIN, &device, T0 + 3000).map_err(er)?;
    sh.append(EventKind::Placed, "o9", PLAIN, &device, T0 + 4000).map_err(er)?;
    write(a, "before-forget.sealed", &sh.to_bytes_trimmed())?;

    // FORGET the person, CUT all but the newest three records, ANCHOR the new tip.
    let n = sh.forget(redact, &sha256(PHONE.as_bytes()), &device, T0 + 5000).map_err(er)?;
    if n != 2 {
        return Err(format!("forget redacted {n} records, the scenario has 2"));
    }
    let history = sh.cut(3, &device, T0 + 6000).map_err(er)?;
    let tip = sh.tip().ok_or("no tip after the cut")?;
    sh.anchor(KIND_PLATFORM_TSA, &tip, T0 + 6500, &tsa::mark(&platform, &tip, T0 + 6500)).map_err(er)?;
    let sealed = sh.to_bytes_trimmed();
    write(a, "history.sealed", &history)?;
    write(a, "hub.sealed", &sealed)?;

    // THE ARCHIVE: the same DOC with SPEC first, so a host with no library can read it.
    let spec = SpecIn { wasm: &a.wasm, wasm_sha256: sha256(&a.wasm), text: &a.text, abi: READER_ABI };
    let mut dst = Store::create_bytes((sealed.len() + a.wasm.len() + a.text.len()).saturating_mul(2).max(1 << 20)).map_err(|e| format!("{e:?}"))?;
    Doc::archive_bytes(&Store::from_bytes(&sealed), &mut dst, &spec).map_err(|e| format!("archive: {e:?}"))?;
    write(a, "archive.store", &dst.to_bytes_trimmed())?;

    write(a, "revoke.sealed", &revoked_device(a, &device)?)?;
    write(a, "legacy-forget.sealed", &legacy_forgotten(a, &device)?)?;
    write(a, "device.pk", hex(device.public_key()).as_bytes())?;
    write(a, "device.actor", hex32(&device.id()).as_bytes())?;
    write(a, "platform.pk", hex(platform.public_key()).as_bytes())
}

fn main() {
    let a = args().unwrap_or_else(|e| {
        eprintln!("maingoal_device: {e}");
        std::process::exit(2)
    });
    if let Err(e) = run(&a) {
        eprintln!("maingoal_device: refused: {e}");
        std::process::exit(1);
    }
}
