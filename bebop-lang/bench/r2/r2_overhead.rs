// R2 Per-Object Overhead — Rust equivalent: sizeof comparisons + alloc overhead.
// Compile: rustc -O -o bench file.rs

use std::alloc::{alloc, dealloc, Layout};
use std::mem;

#[repr(align(64))]
struct Hypervector {
    words: [u64; 16],
}

struct Complex {
    re: f64,
    im: f64,
}

struct Arena {
    mem: *mut u8,
    cap: usize,
    used: usize,
}

const ARENA_ALIGN: usize = 64;

impl Arena {
    fn init(&mut self, mem: *mut u8, cap: usize) {
        let base = mem as usize;
        let aligned = (base + ARENA_ALIGN - 1) & !(ARENA_ALIGN - 1);
        self.mem = aligned as *mut u8;
        self.cap = cap - (aligned - base);
        self.used = 0;
    }

    fn alloc(&mut self, n: usize) -> Option<*mut u8> {
        if self.used + n > self.cap {
            return None;
        }
        let off = (self.used + ARENA_ALIGN - 1) & !(ARENA_ALIGN - 1);
        if off + n > self.cap {
            return None;
        }
        let p = unsafe { self.mem.add(off) };
        self.used = off + n;
        Some(p)
    }

    fn reset(&mut self) {
        self.used = 0;
    }
}

fn main() {
    println!("=== R2 Per-Object Memory Overhead (Rust) ===\n");

    println!("--- sizeof(struct) ---");
    println!("sizeof(Complex)      = {} bytes", mem::size_of::<Complex>());
    println!("sizeof(Hypervector)  = {} bytes (align(64))", mem::size_of::<Hypervector>());

    // Rust global allocator overhead
    println!("\n--- Rust global alloc overhead ---");
    let layout1 = Layout::new::<u8>();
    let a1 = unsafe { alloc(layout1) };
    let a2 = unsafe { alloc(layout1) };
    let spacing = (a2 as isize - a1 as isize).abs();
    println!("alloc(1B), alloc(1B) spacing: {} bytes", spacing);
    println!("  => per-allocation overhead: ~{} bytes", spacing.saturating_sub(1));
    unsafe {
        dealloc(a1, layout1);
        dealloc(a2, layout1);
    }

    // Arena overhead
    println!("\n--- Arena overhead (64B alignment per alloc) ---");
    let buf_size = 65536 * 4;
    let buf = unsafe { alloc(Layout::from_size_align(buf_size, 64).unwrap()) };
    let mut ar = Arena { mem: std::ptr::null_mut(), cap: 0, used: 0 };
    ar.init(buf, buf_size);

    let sizes: [usize; 12] = [1, 4, 8, 16, 32, 64, 128, 256, 512, 1024,
                               mem::size_of::<Complex>(), mem::size_of::<Hypervector>()];
    for &sz in &sizes {
        ar.reset();
        let _first = ar.alloc(sz);
        let used = ar.used;
        let waste = used - sz;
        let waste_pct = (waste as f64) / (sz as f64) * 100.0;
        println!("arena_alloc({:4}): used={:5}, waste={:5} bytes, overhead={:.1}%",
                 sz, used, waste, waste_pct);
    }

    // Mixed alloc efficiency
    println!("\n--- Arena mixed alloc efficiency (1000 allocs) ---");
    ar.reset();
    let mut total_requested = 0usize;
    for i in 0..1000 {
        let sz = sizes[i % sizes.len()];
        total_requested += sz;
        if ar.alloc(sz).is_none() {
            println!("arena OOM at {}", i);
            break;
        }
    }
    let total_used = ar.used;
    let overall_pct = (total_used - total_requested) as f64 / total_requested as f64 * 100.0;
    println!("total_requested: {} bytes", total_requested);
    println!("total_arena_used: {} bytes", total_used);
    println!("overall overhead:  {:.1}%", overall_pct);

    println!("\n=== Summary ===");
    println!("sizeof(Hypervector): {} B (align 64)", mem::size_of::<Hypervector>());
    println!("sizeof(Complex):     {} B", mem::size_of::<Complex>());
    println!("Rust alloc overhead: ~{} bytes per alloc (jemalloc/allocator-dependent)", spacing.saturating_sub(1));

    unsafe { dealloc(buf, Layout::from_size_align(buf_size, 64).unwrap()); }
}