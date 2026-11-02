//! The wasm32 reader's twin of `dowiz_hub::block::kernels::line` (LV-COL-E, R-BEBOP-LEVERS §5.4):
//!   kernel sales rows=<n> groups=<g> gross=<fnv64> costed=<m> cost=<fnv64> top=<key:val,...|->
//!   kernel refused=<code> col=<column|->      kernel skip=<schema>
//! Written from the spec text over THIS crate's decode, not the hub's kernels (that would count
//! one reader twice). Sums are checked in row order; groups fold in ascending signed key order.

use super::{decode, table, Refused};
use std::collections::BTreeMap;

fn sum(keys: &[i64], vals: &[i64], keep: &[bool], col: &'static str) -> Result<BTreeMap<i64, i64>, Refused> {
    let mut g = BTreeMap::new();
    for ((k, v), ok) in keys.iter().zip(vals).zip(keep) {
        if *ok {
            let e = g.entry(*k).or_insert(0i64);
            *e = e.checked_add(*v).ok_or(Refused { code: "overflow", col })?;
        }
    }
    Ok(g)
}

fn fold(g: &BTreeMap<i64, i64>) -> u64 {
    let step = |h: u64, v: i64| v.to_le_bytes().iter().fold(h, |h, x| (h ^ u64::from(*x)).wrapping_mul(0x0000_0100_0000_01b3));
    g.iter().fold(0xcbf2_9ce4_8422_2325, |h, (k, v)| step(step(h, *k), *v))
}

/// The kernel line of one block file.
pub fn line(b: &[u8]) -> String {
    let t = table();
    let run = || -> Result<String, Refused> {
        let d = decode(b, &t)?;
        if t[d.schema].name != "sales" {
            return Ok(format!("kernel skip={}", t[d.schema].name));
        }
        let (dish, gross, cqty, cost) = (&d.cols[1], &d.cols[3], &d.cols[4], &d.cols[6]);
        let g = sum(dish, gross, &vec![true; dish.len()], "gross")?;
        let keep: Vec<bool> = cqty.iter().map(|q| *q > 0).collect();
        let c = sum(dish, cost, &keep, "cost")?;
        let mut top: Vec<(i64, i64)> = g.iter().map(|(k, v)| (*k, *v)).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let top: Vec<String> = top.iter().take(3).map(|(k, v)| format!("{k:016x}:{v:016x}")).collect();
        let top = if top.is_empty() { "-".to_string() } else { top.join(",") };
        let costed = keep.iter().filter(|x| **x).count();
        Ok(format!("kernel sales rows={} groups={} gross={:016x} costed={costed} cost={:016x} top={top}", d.n, g.len(), fold(&g), fold(&c)))
    };
    run().unwrap_or_else(|r| format!("kernel refused={} col={}", r.code, r.col))
}

/// Write `line(ptr..ptr+len)` into `out` (at most `cap` bytes); `bw_block`'s contract.
///
/// # Safety
/// `ptr..ptr+len` is readable (or `len` is 0) and `out..out+cap` is writable.
#[cfg_attr(bw_block, no_mangle)]
pub unsafe extern "C" fn bw_kernel(ptr: *const u8, len: usize, out: *mut u8, cap: usize) -> i32 {
    if out.is_null() || (ptr.is_null() && len != 0) {
        return -crate::abi::NULL_ARG;
    }
    let bytes = if len == 0 { &[][..] } else { core::slice::from_raw_parts(ptr, len) };
    let s = line(bytes);
    if s.len() > cap {
        return -1;
    }
    core::ptr::copy_nonoverlapping(s.as_ptr(), out, s.len());
    s.len() as i32
}
