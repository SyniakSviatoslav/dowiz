//! THE SALES FACT BLOCK AND ITS FOUR SCALAR KERNELS (R-BEBOP-LEVERS §5.4, lane LV-COL-E).
//!
//! `schema::SALES` is one venue-day's sales, one row per dish (`aggregate`). Over its columns:
//!   `mask_filter(col, op, rhs)` -- a row mask, the only predicate form (no expression language);
//!   `sum_by(keys, vals, mask)`  -- CHECKED group sums (an overflow is a named refusal, never a wrap);
//!   `topk(groups, k)`           -- value descending, key ascending: one total order, so every reader
//!                                  picks the same k;
//!   `csr_join(keys, from, to)`  -- each row's key mapped through a two-column table (dish -> category
//!                                  out of `names`/CSR rows), `None` where the table has no row.
//! A PLAN IS DATA (§5.4, as the A.3 rules): `[op, a, b, c, 0, 0, 0, 0]` rows -- FILTER, GROUP, AGG,
//! TOPK -- run by `run_plan`, one function. Not a query parser, not an optimiser.
//!
//! FULL ONLY. Each kernel is `full(blocks)`: a fold over every day block of a span. The
//! incremental `step(acc, zset Δ)` form waits for LV-INC-A's `ZSet` (§2.4); until it lands a
//! hot tail is folded `full` beside the blocks (`services::analytics`), never stepped.
//!
//! `line` is what `crates/bebop-wasm/gate.sh` compares across four readers (`block_read --kernel`,
//! `bw_kernel`, `oracle.py --kernel`, `block.bp bk_kernel_line`):
//!   kernel sales rows=<n> groups=<g> gross=<fnv64> costed=<m> cost=<fnv64> top=<key:val,...|->
//!   kernel refused=<code> col=<column|->
//! `gross` = sum_by(dish, gross) over every row; `cost` = sum_by(dish, cost) over the rows whose
//! `cqty > 0` (`costed` of them); each fnv is FNV-1a 64 over the groups in ascending key order,
//! key then sum, 8 LE bytes each; `top` = topk(gross groups, 3), both halves 16-hex.

use std::collections::BTreeMap;

use super::decode::decode;
use super::{schema, Block, Col, Refusal};

#[path = "kernels/pack.rs"]
pub mod pack;
/// W-BLOCKVIEW: the image read in place (`Index`, `Day`) and the kernels over borrowed columns.
#[path = "kernels/image.rs"]
pub mod image;
#[path = "kernels/sample.rs"]
mod sample;
pub use sample::sample;

/// One `sales` row: a dish's day (or, before `aggregate`, one order line the venue took).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sale {
    /// The venue's local day, `yyyymmdd`.
    pub day: i32,
    /// `K64` of the product id.
    pub dish: i64,
    /// Portions, every line.
    pub qty: i32,
    /// Unit price x qty, every line, minor units.
    pub gross: i64,
    /// Portions of the lines that carry a stamped cost.
    pub cqty: i32,
    /// Their sales, minor units.
    pub cgross: i64,
    /// Their stamped cost, minor units.
    pub cost: i64,
}

impl Sale {
    /// `self + o`, every field CHECKED (an overflow names its column).
    pub fn plus(&self, o: &Sale) -> Result<Sale, Refusal> {
        let i = |a: i32, b: i32, col| a.checked_add(b).ok_or(Refusal::Overflow { col });
        let l = |a: i64, b: i64, col| a.checked_add(b).ok_or(Refusal::Overflow { col });
        Ok(Sale {
            day: self.day,
            dish: self.dish,
            qty: i(self.qty, o.qty, "qty")?,
            gross: l(self.gross, o.gross, "gross")?,
            cqty: i(self.cqty, o.cqty, "cqty")?,
            cgross: l(self.cgross, o.cgross, "cgross")?,
            cost: l(self.cost, o.cost, "cost")?,
        })
    }
}

/// THE CANONICAL DAY ROWS: lines summed per `(day, dish)`, ordered by day then dish (signed `K64`),
/// so the same lines in any order -- one archive or three, at the catch-up or in the trace's
/// refold -- give the same rows and the same block bytes.
pub fn aggregate(lines: &[Sale]) -> Result<Vec<Sale>, Refusal> {
    let mut m: BTreeMap<(i32, i64), Sale> = BTreeMap::new();
    for l in lines {
        let e = m.entry((l.day, l.dish)).or_insert(Sale { day: l.day, dish: l.dish, ..Sale::default() });
        *e = e.plus(l)?;
    }
    Ok(m.into_values().collect())
}

/// The `sales` block of `rows`, in the order given.
pub fn sales_block(rows: &[Sale]) -> Block {
    let i32s = |f: fn(&Sale) -> i32| Col::I32(rows.iter().map(f).collect());
    let i64s = |f: fn(&Sale) -> i64| Col::I64(rows.iter().map(f).collect());
    Block {
        schema: &schema::SALES,
        n: rows.len() as u32,
        nnz: 0,
        cols: vec![i32s(|s| s.day), i64s(|s| s.dish), i32s(|s| s.qty), i64s(|s| s.gross), i32s(|s| s.cqty), i64s(|s| s.cgross), i64s(|s| s.cost)],
    }
}

/// The rows of a `sales` block; any other schema is refused.
pub fn sales_of(b: &Block) -> Result<Vec<Sale>, Refusal> {
    if b.schema != &schema::SALES {
        return Err(Refusal::Mismatch("not a sales block"));
    }
    let c: Vec<Vec<i64>> = (0..7).map(|i| widen(&b.cols[i])).collect();
    Ok((0..b.n as usize)
        .map(|r| Sale { day: c[0][r] as i32, dish: c[1][r], qty: c[2][r] as i32, gross: c[3][r], cqty: c[4][r] as i32, cgross: c[5][r], cost: c[6][r] })
        .collect())
}

/// A column as i64 values (i32 sign-extended, u32 and bytes zero-extended).
pub fn widen(c: &Col) -> Vec<i64> {
    match c {
        Col::I64(v) => v.clone(),
        Col::I32(v) => v.iter().map(|x| i64::from(*x)).collect(),
        Col::Bytes(v) => v.iter().map(|x| i64::from(*x)).collect(),
        Col::U32(v) => v.iter().map(|x| i64::from(*x)).collect(),
    }
}

/// A row predicate's comparison. Codes are the plan's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq = 1,
    Ne = 2,
    Lt = 3,
    Le = 4,
    Gt = 5,
    Ge = 6,
}

impl Op {
    pub fn of(code: i64) -> Option<Op> {
        [Op::Eq, Op::Ne, Op::Lt, Op::Le, Op::Gt, Op::Ge].into_iter().find(|o| *o as i64 == code)
    }
    fn holds(self, x: i64, rhs: i64) -> bool {
        match self {
            Op::Eq => x == rhs,
            Op::Ne => x != rhs,
            Op::Lt => x < rhs,
            Op::Le => x <= rhs,
            Op::Gt => x > rhs,
            Op::Ge => x >= rhs,
        }
    }
}

/// KERNEL 1: the rows where `col op rhs`.
pub fn mask_filter(col: &[i64], op: Op, rhs: i64) -> Vec<bool> {
    col.iter().map(|x| op.holds(*x, rhs)).collect()
}

/// KERNEL 2: `vals` summed by `keys` over the masked rows, in row order, every add CHECKED.
pub fn sum_by(keys: &[i64], vals: &[i64], mask: Option<&[bool]>, col: &'static str) -> Result<BTreeMap<i64, i64>, Refusal> {
    let mut out = BTreeMap::new();
    sum_into(&mut out, keys, vals, mask, col)?;
    Ok(out)
}

/// `sum_by` into an accumulator: the fold `full` runs over a span's blocks.
pub fn sum_into(acc: &mut BTreeMap<i64, i64>, keys: &[i64], vals: &[i64], mask: Option<&[bool]>, col: &'static str) -> Result<(), Refusal> {
    for (i, (k, v)) in keys.iter().zip(vals).enumerate() {
        if mask.is_some_and(|m| !m.get(i).copied().unwrap_or(false)) {
            continue;
        }
        let e = acc.entry(*k).or_insert(0);
        *e = e.checked_add(*v).ok_or(Refusal::Overflow { col })?;
    }
    Ok(())
}

/// KERNEL 3: the `k` largest groups, value descending then key ascending.
pub fn topk(groups: &BTreeMap<i64, i64>, k: usize) -> Vec<(i64, i64)> {
    let mut all: Vec<(i64, i64)> = groups.iter().map(|(a, b)| (*a, *b)).collect();
    all.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    all.truncate(k);
    all
}

/// KERNEL 4: each key mapped through the table `from[i] -> to[i]`; `None` where no row has it.
/// A key the table names twice is refused: a join that picks one of two rows is a guess.
pub fn csr_join(keys: &[i64], from: &[i64], to: &[i64]) -> Result<Vec<Option<i64>>, Refusal> {
    let mut t = BTreeMap::new();
    for (f, v) in from.iter().zip(to) {
        if t.insert(*f, *v).is_some() {
            return Err(Refusal::Mismatch("csr_join: a key with two rows"));
        }
    }
    Ok(keys.iter().map(|k| t.get(k).copied()).collect())
}

/// FULL: `sum_by(key, val)` over every block of a span, masked by `filter` when given.
pub fn full_sum_by(blocks: &[Block], key: usize, val: usize, filter: Option<(usize, Op, i64)>) -> Result<BTreeMap<i64, i64>, Refusal> {
    let mut acc = BTreeMap::new();
    for b in blocks {
        let name = schema::of(b.schema).and_then(|k| k.cols.get(val)).map_or("-", |c| c.name);
        let (Some(kc), Some(vc)) = (b.cols.get(key), b.cols.get(val)) else { return Err(Refusal::Mismatch("plan names no such column")) };
        let mask = match filter {
            Some((c, op, rhs)) => Some(mask_filter(&widen(b.cols.get(c).ok_or(Refusal::Mismatch("plan names no such column"))?), op, rhs)),
            None => None,
        };
        sum_into(&mut acc, &widen(kc), &widen(vc), mask.as_deref(), name)?;
    }
    Ok(acc)
}

/// A plan row: `[op, a, b, c, 0, 0, 0, 0]`.
pub type PlanRow = [i64; 8];
/// `[FILTER, col, op, rhs]`, `[GROUP, key_col]`, `[AGG, val_col]` (a sum), `[TOPK, k]`.
pub const FILTER: i64 = 1;
pub const GROUP: i64 = 2;
pub const AGG: i64 = 3;
pub const TOPK: i64 = 4;

/// A plan read: the filter, the key and value columns, `k`.
pub(crate) type Plan = (Option<(usize, Op, i64)>, usize, usize, Option<usize>);

/// A plan's rows: at most one of each, GROUP and AGG required.
pub(crate) fn plan_of(plan: &[PlanRow]) -> Result<Plan, Refusal> {
    let (mut filter, mut key, mut val, mut k) = (None, None, None, None);
    for r in plan {
        let col = || usize::try_from(r[1]).map_err(|_| Refusal::Mismatch("plan: a negative column"));
        match r[0] {
            FILTER => filter = Some((col()?, Op::of(r[2]).ok_or(Refusal::Mismatch("plan: no such comparison"))?, r[3])),
            GROUP => key = Some(col()?),
            AGG => val = Some(col()?),
            TOPK => k = Some(usize::try_from(r[1]).map_err(|_| Refusal::Mismatch("plan: a negative k"))?),
            _ => return Err(Refusal::Mismatch("plan: no such row")),
        }
    }
    let (Some(key), Some(val)) = (key, val) else { return Err(Refusal::Mismatch("plan: GROUP and AGG are required")) };
    Ok((filter, key, val, k))
}

/// Run a plan over a span's blocks.
pub fn run_plan(blocks: &[Block], plan: &[PlanRow]) -> Result<Vec<(i64, i64)>, Refusal> {
    let (filter, key, val, k) = plan_of(plan)?;
    let groups = full_sum_by(blocks, key, val, filter)?;
    Ok(match k {
        Some(k) => topk(&groups, k),
        None => groups.into_iter().collect(),
    })
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv(mut h: u64, v: i64) -> u64 {
    for b in v.to_le_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn fold(groups: &BTreeMap<i64, i64>) -> u64 {
    groups.iter().fold(FNV_OFFSET, |h, (k, v)| fnv(fnv(h, *k), *v))
}

/// The gate's kernel line for one block file (see the module).
pub fn line(bytes: &[u8]) -> String {
    let refused = |r: &Refusal| {
        let col = match r {
            Refusal::Overflow { col } => *col,
            Refusal::BadType { col } | Refusal::BadUnit { col } | Refusal::BadReserved { col } | Refusal::BadAlignment { col } => *col,
            Refusal::BadOffsets { col } | Refusal::BadLength { col } | Refusal::BadPadding { col } | Refusal::NotUtf8 { col } => *col,
            _ => "-",
        };
        format!("kernel refused={} col={col}", r.code())
    };
    let b = match decode(bytes) {
        Ok(b) => b,
        Err(r) => return refused(&r),
    };
    if b.schema != &schema::SALES {
        return format!("kernel skip={}", b.schema.name);
    }
    let (dish, gross, cqty, cost) = (widen(&b.cols[1]), widen(&b.cols[3]), widen(&b.cols[4]), widen(&b.cols[6]));
    let g = match sum_by(&dish, &gross, None, "gross") {
        Ok(g) => g,
        Err(r) => return refused(&r),
    };
    let m = mask_filter(&cqty, Op::Gt, 0);
    let c = match sum_by(&dish, &cost, Some(&m), "cost") {
        Ok(c) => c,
        Err(r) => return refused(&r),
    };
    let top: Vec<String> = topk(&g, 3).iter().map(|(k, v)| format!("{k:016x}:{v:016x}")).collect();
    let top = if top.is_empty() { "-".to_string() } else { top.join(",") };
    format!(
        "kernel sales rows={} groups={} gross={:016x} costed={} cost={:016x} top={top}",
        b.n,
        g.len(),
        fold(&g),
        m.iter().filter(|x| **x).count(),
        fold(&c)
    )
}

#[cfg(test)]
#[path = "kernels/tests.rs"]
mod tests;
