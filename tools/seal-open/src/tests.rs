use super::*;
use backup_seal::{SealEntropy, SealPublic};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// A fresh directory per test, under the target's tmp dir.
fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("seal-open-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// A REAL hub image (the bebop store the nightly copies), a few orders on it.
fn hub_image() -> (Vec<u8>, Option<String>) {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    for (i, id) in ["a", "b", "c", "d"].iter().enumerate() {
        h.append(dowiz_hub::EventKind::Placed, id, "{\"total\":1500}", i as u64 + 1, [0u8; 32]).unwrap();
    }
    (h.to_bytes(), h.tip())
}

fn sealed_to(pk_text: &str, plain: &[u8]) -> Vec<u8> {
    let pk = SealPublic::parse(pk_text).unwrap();
    let e = SealEntropy { m: [9u8; 32], eph: [8u8; 32], nonce: [7u8; 12] };
    backup_seal::seal(&pk, 1, plain, &e).unwrap()
}

#[test]
fn keygen_writes_a_0600_secret_and_refuses_to_overwrite() {
    let d = dir("keygen");
    let sk = d.join("backup.sk");
    let pk = keygen_to(&sk, &[4u8; 64]).unwrap();
    assert!(pk.starts_with(backup_seal::PK_PREFIX));
    assert_eq!(fs::metadata(&sk).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(pubkey_of(&sk).unwrap(), pk, "pubkey re-derives the same text");
    assert!(keygen_to(&sk, &[5u8; 64]).is_err(), "overwrote a secret key");
    assert_eq!(pubkey_of(&sk).unwrap(), pk, "the refused keygen left the key alone");
    assert_eq!(os_seed().unwrap().len(), 64);
}

#[test]
fn open_roundtrips_a_hub_image_byte_equal() {
    let d = dir("open");
    let sk = d.join("k");
    let pk = keygen_to(&sk, &[1u8; 64]).unwrap();
    let (image, tip) = hub_image();
    fs::write(d.join("x.sealed"), sealed_to(&pk, &image)).unwrap();
    assert_eq!(open_file(&sk, &d.join("x.sealed"), &d.join("x.out")), Ok(1));
    let opened = fs::read(d.join("x.out")).unwrap();
    assert_eq!(opened, image);
    assert_eq!(dowiz_hub::Hub::load(&opened).unwrap().tip(), tip, "witness tip survives");
    assert!(open_file(&sk, &d.join("x.sealed"), &d.join("x.out")).is_err(), "overwrote the output");
}

#[test]
fn a_tampered_truncated_or_foreign_file_refuses_and_writes_nothing() {
    let d = dir("refuse");
    let sk = d.join("k");
    let pk = keygen_to(&sk, &[1u8; 64]).unwrap();
    let other = d.join("other");
    keygen_to(&other, &[2u8; 64]).unwrap();
    let sealed = sealed_to(&pk, &hub_image().0);
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    for (name, at) in [("magic", 0), ("kem-ct", 50), ("x25519", 1100), ("tag", 1140), ("nonce", 1165), ("body", 2000)] {
        let mut t = sealed.clone();
        t[at] ^= 0x10;
        cases.push((name, t));
    }
    let mut t = sealed.clone();
    *t.last_mut().unwrap() ^= 1;
    cases.push(("gcm-tag", t));
    cases.push(("truncated", sealed[..sealed.len() - 5].to_vec()));
    cases.push(("header-only", sealed[..1173].to_vec()));
    for (name, bytes) in cases {
        let (inp, out) = (d.join(format!("{name}.in")), d.join(format!("{name}.out")));
        fs::write(&inp, bytes).unwrap();
        assert!(open_file(&sk, &inp, &out).is_err(), "{name} opened");
        assert!(!out.exists(), "{name} wrote an output");
    }
    // The wrong secret key, and a corrupt secret-key file.
    fs::write(d.join("good.in"), &sealed).unwrap();
    assert!(open_file(&other, &d.join("good.in"), &d.join("o1")).is_err());
    fs::write(d.join("bad.sk"), b"DWZSSK1\nshort").unwrap();
    assert!(open_file(&d.join("bad.sk"), &d.join("good.in"), &d.join("o2")).is_err());
    // Positive twin of every case above: the untouched file opens.
    assert_eq!(open_file(&sk, &d.join("good.in"), &d.join("o3")), Ok(1));
}
