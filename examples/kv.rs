// SPDX-License-Identifier: MIT OR Apache-2.0
//! The bebop KV database in fifteen lines: create a file, commit, reopen, read, scan, and
//! name a refusal. Run: `cd crates/bebop-ffi && cargo run --release --example kv [path]`.

use bebop_io::{Db, Durability, Options, Refusal, Verify};

fn main() -> Result<(), Refusal> {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        let p = std::env::temp_dir().join(format!("bebop-example-{}.bebop", std::process::id()));
        p.to_string_lossy().into_owned()
    });
    let _ = std::fs::remove_file(&path);

    {
        let db = Db::create_path(&path, Options::default())?;
        let mut w = db.begin_write()?;
        w.put(b"order/0001", b"new")?;
        w.put(b"order/0002", b"paid")?;
        w.put(b"zone/north", b"open")?;
        let g = w.commit(Durability::Data)?;
        println!("committed generation {}", g.get());
    } // the Db drops here; the file and its lock are released

    let db = Db::open_path(&path, Options::default().verify(Verify::Reachable))?;
    let r = db.begin_read();
    println!("reopened: {} entries, generation {}", r.len(), r.generation().get());
    let v = r.get(b"order/0002")?.expect("written above");
    println!("order/0002 = {}", String::from_utf8_lossy(&v));
    for e in r.prefix(b"order/")? {
        let (k, v) = e?;
        println!("  {} = {}", String::from_utf8_lossy(&k), String::from_utf8_lossy(&v));
    }
    println!("fold root {}", r.fold_root()? as i64);

    // A refusal is a value with a name, never a panic.
    let mut w = db.begin_write()?;
    match w.put(&vec![b'k'; 70_000], b"v") {
        Err(e @ Refusal::KeyTooLong { .. }) => println!("refused as expected: {e}"),
        other => panic!("expected KeyTooLong, got {other:?}"),
    }
    w.abort();
    drop(r);
    drop(db);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.lock"));
    Ok(())
}
