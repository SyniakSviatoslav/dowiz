// SPDX-License-Identifier: MIT OR Apache-2.0
//! PROMISE: a commit that did not finish is not there. The owner of the file gets the commit
//! before it. Bytes handed over whole are not the owner's crash, so the same damage there is a
//! refusal, never a quiet step back to older data.
//! Run: `cargo run --example crash_fallback`
//!
//! This damages one commit by hand. The crash harness in `einfile_io::sim`, run by the tests,
//! enumerates the crash points of the commit protocol. Neither one is a power cut on real hardware.

use einstore::{Db, Durability, Options, Refusal, Verify};

fn main() -> Result<(), Refusal> {
    let path = std::env::temp_dir().join(format!("ein-crash-{}.ein", std::process::id()));
    let path = path.to_str().expect("a UTF-8 temp path").to_owned();

    {
        let db = Db::create_path(&path, Options::default())?;
        let mut w = db.begin_write()?;
        w.put(b"first", b"committed")?;
        w.commit(Durability::Data)?;
        let mut w = db.begin_write()?;
        w.put(b"second", b"will be torn")?;
        w.commit(Durability::Data)?;
    }

    // Tear the last commit: change one byte of the value it wrote.
    let mut bytes = std::fs::read(&path).expect("the file was just written");
    let at = bytes.windows(12).position(|w| w == b"will be torn").expect("the value is in the file");
    bytes[at] ^= 1;
    std::fs::write(&path, &bytes).expect("the file is writable");

    // Handed over whole: refused.
    let refused = Db::from_bytes(&bytes, Options::detached());
    assert!(refused.is_err(), "bytes handed over whole never fall back");
    println!("bytes handed over whole: {}", refused.err().expect("checked above"));

    // Opened by its owner: the commit before it.
    {
        let db = Db::open_path(&path, Options::default().verify(Verify::Reachable))?;
        let r = db.begin_read();
        assert!(r.get(b"first")?.is_some(), "the finished commit is there");
        assert!(r.get(b"second")?.is_none(), "the torn commit is not");
        println!("opened by its owner: generation {}, 'second' absent", r.generation().get());
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.lock"));
    Ok(())
}
