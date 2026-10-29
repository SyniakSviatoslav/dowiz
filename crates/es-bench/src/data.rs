//! Deterministic data, statistics, timing and the small file helpers every adapter shares.
//! Lifted from the store-vs-SQLite harness (R-STORE-VS-SQL-2026-10-27, `common.rs`): the same
//! splitmix64 stream, the same `key{:012}` keys, the same median/p95.
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::time::Instant;

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// 15 ASCII bytes, sorting in numeric order.
pub fn key(i: u64) -> Vec<u8> {
    format!("key{:012}", i).into_bytes()
}

pub fn val(i: u64, vs: usize, salt: u64) -> Vec<u8> {
    let mut r = Rng(i.wrapping_mul(0x1000_0000_01B3) ^ salt.wrapping_mul(0xDEAD_BEEF));
    let mut v = Vec::with_capacity(vs);
    while v.len() < vs {
        let x = r.next().to_le_bytes();
        let take = (vs - v.len()).min(8);
        v.extend_from_slice(&x[..take]);
    }
    v
}

/// 0..n in a fixed random order.
pub fn perm(n: u64, seed: u64) -> Vec<u64> {
    let mut p: Vec<u64> = (0..n).collect();
    let mut r = Rng(seed);
    for i in (1..p.len()).rev() {
        let j = r.below(i as u64 + 1) as usize;
        p.swap(i, j);
    }
    p
}

/// The data set, SORTED by key (every engine's bulk load gets the same order).
pub fn entries(n: u64, vs: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    (0..n).map(|i| (key(i), val(i, vs, 0))).collect()
}

/// `m` uniformly random existing keys.
pub fn lookups(n: u64, m: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut r = Rng(seed);
    (0..m).map(|_| key(r.below(n))).collect()
}

/// `q` prefixes of 13 bytes: `key` + the top 10 digits, i.e. 100 keys each when n >= 100.
pub fn prefixes(n: u64, q: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut r = Rng(seed);
    (0..q).map(|_| key(r.below(n))[..13].to_vec()).collect()
}

/// What every read folds a value into: a checksum all engines must agree on.
pub fn acc_of(v: &[u8]) -> u64 {
    v.len() as u64 + v[0] as u64
}

#[derive(Clone, Copy, Debug)]
pub struct St {
    pub med: f64,
    pub p95: f64,
    pub min: f64,
    pub n: usize,
}

pub fn stats(mut v: Vec<f64>) -> St {
    assert!(!v.is_empty(), "stats of no samples");
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    let p95i = ((0.95 * n as f64).ceil() as usize).clamp(1, n) - 1;
    St { med: v[n / 2], p95: v[p95i], min: v[0], n }
}

/// One machine-readable result line: `RES|row|engine|med|p95|min|reps|unit|note`.
pub fn res(row: &str, engine: &str, unit: &str, s: St, note: &str) {
    println!("RES|{row}|{engine}|med={:.3}|p95={:.3}|min={:.3}|reps={}|{unit}|{note}", s.med, s.p95, s.min, s.n);
}
pub fn one(row: &str, engine: &str, unit: &str, x: f64, note: &str) {
    println!("RES|{row}|{engine}|value={x:.3}|{unit}|{note}");
}

pub fn t() -> Instant {
    Instant::now()
}
pub fn us(t0: Instant) -> f64 {
    t0.elapsed().as_nanos() as f64 / 1000.0
}

pub fn dir_sync(p: &Path) {
    let d = p.parent().unwrap_or(Path::new("."));
    File::open(d).unwrap().sync_all().unwrap();
}

/// Write a whole file crash-safely: tmp, fsync, rename, fsync the directory.
pub fn write_atomic(path: &Path, bytes: &[u8], sync: bool) {
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp).unwrap();
        f.write_all(bytes).unwrap();
        if sync {
            f.sync_data().unwrap();
        }
    }
    fs::rename(&tmp, path).unwrap();
    if sync {
        dir_sync(path);
    }
}

/// Bytes of every file under `dir` (one level; every engine here keeps a flat directory).
pub fn dir_bytes(dir: &Path) -> u64 {
    fs::read_dir(dir).map(|r| r.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
}

/// Drop every file under `dir` from the page cache (clean pages only). False if any refused.
pub fn evict_dir(dir: &Path) -> bool {
    let mut ok = true;
    for e in fs::read_dir(dir).unwrap().flatten() {
        if let Ok(f) = File::open(e.path()) {
            let _ = f.sync_all();
            ok &= unsafe { libc::posix_fadvise(f.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) } == 0;
        }
    }
    ok
}

/// A read-only MAP_SHARED view of a whole file: writes through any fd of the SAME inode show.
pub struct Mmap {
    ptr: *mut libc::c_void,
    len: usize,
}
impl Mmap {
    pub fn open(p: &Path) -> Mmap {
        let f = File::open(p).unwrap();
        let len = f.metadata().unwrap().len() as usize;
        assert!(len > 0, "mmap of an empty file {}", p.display());
        let ptr = unsafe { libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ, libc::MAP_SHARED, f.as_raw_fd(), 0) };
        assert!(ptr != libc::MAP_FAILED, "mmap {}", p.display());
        Mmap { ptr, len }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }
}
impl Drop for Mmap {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr, self.len);
        }
    }
}
