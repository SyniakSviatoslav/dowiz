// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE: a changed byte is refused by name. It is never read as data.
//! Run: `cargo run --example refuse_damage`

use einstore::{Db, Durability, Options, Refusal, Verify};

fn main() -> Result<(), Refusal> {
    let db = Db::create_detached(Options::detached())?;
    let mut w = db.begin_write()?;
    w.put(b"invoice/7", b"total=1500 lek")?;
    w.commit(Durability::None)?; // the caller persists these bytes
    let image = db.snapshot_bytes();

    // The control: the same bytes, untouched, read back.
    let clean = Db::from_bytes(&image, Options::detached().verify(Verify::Reachable))?;
    let r = clean.begin_read();
    let v = r.get(b"invoice/7")?.expect("written above");
    assert_eq!(&v[..], b"total=1500 lek");
    println!("clean image: invoice/7 = {}", String::from_utf8_lossy(&v));

    // One bit of the value, which sits in the file as plain bytes: 1500 becomes 1400.
    let mut changed = image.clone();
    let at = changed.windows(10).position(|w| w == b"total=1500").expect("the value is in the file");
    changed[at + 7] ^= 1;

    match Db::from_bytes(&changed, Options::detached().verify(Verify::Reachable)) {
        Err(e @ Refusal::BadCrc(_)) => println!("changed image refused: {e}"),
        Err(other) => panic!("refused, but not as BadCrc: {other:?}"),
        Ok(_) => panic!("a changed image was read as data"),
    }
    Ok(())
}
