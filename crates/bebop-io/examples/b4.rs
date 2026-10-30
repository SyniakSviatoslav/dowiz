// SPDX-License-Identifier: MIT OR Apache-2.0
//! B4 (R-EMBED-STORE §4.6, §12 P10): open + verify after N commits, each depth measured apart.
//!
//!   cargo run --release --offline --features tree --example b4 -- <dir> <commits> [reps]
//!
//! Writes a v4 (B+tree) store of `commits` single-put commits into `<dir>/b4.bebop` (Durability::None:
//! the writes are not what is measured), then times, `reps` times each, the median and minimum printed:
//!
//!   open_anchor     Db::open_path at Verify::Anchor (the default): pick + PartTab crc + the anchor's crc
//!                   over the last commit + KvIn::open_lazy -- and, today, the image read and copied
//!   open_reachable  the same at Verify::Reachable (every reachable object's crc)
//!   open_whole      the same at Verify::WholeArena (every object of the arena)
//!   read_file       std::fs::read of the image: the floor under every open above
//!   recover         sim::recover over a View of the bytes: the anchor's crc alone (O(last commit))
//!   open_lazy       KvIn::open_lazy over the View: the shape walk, no crc
//!   kvin_open       KvIn::open over the View: what es-bench's `bebop` engine calls open (= verify)
//!   walk            sim::walk of the whole arena: WholeArena's extra
//!   store_copy      Store::from_bytes + to_bytes_trimmed: the two copies Db::open pays after the read
//!
//! Every line is `b4 commits=<n> image_bytes=<b> row=<row> median_us=<m> min_us=<min> reps=<r>`.
//! Memory: run under `ulimit -v 2500000`; the image is a few hundred bytes per commit.

use std::time::Instant;

use bebop_image::{Store, View, ARENA};
use bebop_io::sim::{recover, walk};
use bebop_io::{Db, Durability, Options, Verify};
use bebop_kv::zc::KvIn;

fn us<T>(f: impl FnOnce() -> T) -> (f64, T) {
    let t = Instant::now();
    let r = f();
    (t.elapsed().as_secs_f64() * 1e6, r)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = a.get(1).expect("dir");
    let n: usize = a.get(2).expect("commits").parse().expect("commits");
    let reps: usize = a.get(3).map(|s| s.parse().expect("reps")).unwrap_or(5);
    std::fs::create_dir_all(dir).expect("dir");
    let path = format!("{dir}/b4.bebop");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path}.lock"));

    let opts = Options::default().create_tree(true);
    let t0 = Instant::now();
    {
        let db = Db::create_path(&path, opts).expect("create");
        for i in 0..n {
            let mut w = db.begin_write().expect("begin");
            w.put(format!("k/{:010}", (i as u64).wrapping_mul(2654435761) % (n as u64 * 4)).as_bytes(), &[(i % 251) as u8; 48]).expect("put");
            w.commit(Durability::None).expect("commit");
        }
        println!("b4 commits={n} wrote in {:.1} s, compactions={}", t0.elapsed().as_secs_f64(), db.compactions());
    }
    let bytes = std::fs::read(&path).expect("read");
    let image_bytes = bytes.len();
    // The anchor's range IS the last commit: after a copy compaction that is the whole live set,
    // so "O(last commit)" is only small when the last commit was small. Said here, per run.
    let rec = recover(&View::new(&bytes)).expect("recover");
    println!("b4 commits={n} image_bytes={image_bytes} generation={} via={:?} arena_cells={}", rec.sb.generation, rec.via, rec.sb.arena_used - ARENA as i64);
    let say = |row: &str, v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("b4 commits={n} image_bytes={image_bytes} row={row} median_us={:.0} min_us={:.0} reps={reps}", v[v.len() / 2], v[0]);
    };
    let time = |row: &str, f: &mut dyn FnMut()| {
        let mut v = Vec::with_capacity(reps);
        for _ in 0..reps {
            v.push(us(&mut *f).0);
        }
        say(row, &mut v);
    };

    for (row, depth) in [("open_anchor", Verify::Anchor), ("open_reachable", Verify::Reachable), ("open_whole", Verify::WholeArena)] {
        time(row, &mut || {
            let db = Db::open_path(&path, opts.verify(depth)).expect("open");
            std::hint::black_box(db.begin_read().generation());
        });
    }
    time("read_file", &mut || {
        std::hint::black_box(std::fs::read(&path).expect("read"));
    });
    time("recover", &mut || {
        std::hint::black_box(recover(&View::new(&bytes)).expect("recover"));
    });
    time("open_lazy", &mut || {
        std::hint::black_box(KvIn::open_lazy(View::new(&bytes)).expect("open_lazy"));
    });
    time("kvin_open", &mut || {
        std::hint::black_box(KvIn::open(View::new(&bytes)).expect("open"));
    });
    let used = recover(&View::new(&bytes)).unwrap().sb.arena_used as usize;
    time("walk", &mut || {
        std::hint::black_box(walk(&View::new(&bytes), ARENA, used).expect("walk"));
    });
    time("store_copy", &mut || {
        std::hint::black_box(Store::from_bytes(&bytes).to_bytes_trimmed());
    });
}
