//! THE KERNELS' INNER LOOPS over borrowed columns (lane W-BLOCKVIEW; `kernels::image`): one
//! monomorphic fold per (key, value, mask) width, summed into `Groups`, every add CHECKED.

use std::collections::BTreeMap;
use super::super::super::Refusal;
use super::super::Op;
use super::{le_i32, ColRef};

/// One element width, monomorphised into the folds below.
trait Lane {
    const W: usize;
    fn get(w: &[u8]) -> i64;
}
macro_rules! lane {
    ($t:ident, $w:expr, |$x:ident| $e:expr) => {
        struct $t;
        impl Lane for $t {
            const W: usize = $w;
            #[inline(always)]
            fn get($x: &[u8]) -> i64 {
                $e
            }
        }
    };
}
lane!(L64, 8, |w| i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]));
lane!(L32, 4, |w| le_i32(w));
lane!(U32, 4, |w| i64::from(u32::from_le_bytes([w[0], w[1], w[2], w[3]])));
lane!(L8, 1, |w| i64::from(w[0]));

/// Group sums: a sorted run with a next-slot hint, spilled to a `BTreeMap` past `RUN_MAX` groups so
/// a key column with many values stays O(log g) per row. Every add CHECKED.
#[derive(Debug, Default)]
pub struct Groups {
    run: Vec<(i64, i64)>,
    hint: usize,
    tree: Option<BTreeMap<i64, i64>>,
}
const RUN_MAX: usize = 1024;

impl Groups {
    /// The hot path: the hinted slot (ascending-key rows), inlined into the fold.
    #[inline(always)]
    fn add(&mut self, k: i64, v: i64, col: &'static str) -> Result<(), Refusal> {
        match self.run.get_mut(self.hint) {
            Some((h, e)) if *h == k && self.tree.is_none() => {
                *e = e.checked_add(v).ok_or(Refusal::Overflow { col })?;
                self.hint += 1;
                Ok(())
            }
            _ => self.add_slow(k, v, col),
        }
    }

    #[cold]
    fn add_slow(&mut self, k: i64, v: i64, col: &'static str) -> Result<(), Refusal> {
        let at = self.run.binary_search_by_key(&k, |e| e.0);
        if self.tree.is_none() && (at.is_ok() || self.run.len() < RUN_MAX) {
            let i = at.unwrap_or_else(|i| {
                self.run.insert(i, (k, 0));
                i
            });
            self.hint = i + 1;
            let e = &mut self.run[i].1;
            *e = e.checked_add(v).ok_or(Refusal::Overflow { col })?;
            return Ok(());
        }
        let run = std::mem::take(&mut self.run);
        let e = self.tree.get_or_insert_with(|| run.into_iter().collect()).entry(k).or_insert(0);
        *e = e.checked_add(v).ok_or(Refusal::Overflow { col })?;
        Ok(())
    }

    /// The sums by key.
    pub fn into_map(self) -> BTreeMap<i64, i64> {
        self.tree.unwrap_or_else(|| self.run.into_iter().collect())
    }
}

/// One block's fold, every width known; `M` is the mask column's. Rows past the shortest column are
/// not read (`zip`), as `sum_into` drops rows past its mask.
fn fold<K: Lane, V: Lane, M: Lane>(acc: &mut Groups, k: &[u8], v: &[u8], mask: Option<(&[u8], Op, i64)>, col: &'static str) -> Result<(), Refusal> {
    let rows = k.chunks_exact(K::W).zip(v.chunks_exact(V::W));
    match mask {
        None => rows.map(|(a, b)| acc.add(K::get(a), V::get(b), col)).collect(),
        Some((m, op, rhs)) => rows.zip(m.chunks_exact(M::W)).filter(|(_, x)| op.holds(M::get(x), rhs)).map(|((a, b), _)| acc.add(K::get(a), V::get(b), col)).collect(),
    }
}

/// `sum_into` over borrowed columns: one monomorphic loop per (key, value, mask) width.
pub fn sum_into_cols(acc: &mut Groups, key: ColRef, val: ColRef, mask: Option<(ColRef, Op, i64)>, col: &'static str) -> Result<(), Refusal> {
    macro_rules! m {
        ($K:ty, $V:ty, $k:expr, $v:expr) => {
            match mask {
                None => fold::<$K, $V, L8>(acc, $k, $v, None, col),
                Some((ColRef::I64(m), op, r)) => fold::<$K, $V, L64>(acc, $k, $v, Some((m, op, r)), col),
                Some((ColRef::I32(m), op, r)) => fold::<$K, $V, L32>(acc, $k, $v, Some((m, op, r)), col),
                Some((ColRef::U32(m), op, r)) => fold::<$K, $V, U32>(acc, $k, $v, Some((m, op, r)), col),
                Some((ColRef::U8(m), op, r)) => fold::<$K, $V, L8>(acc, $k, $v, Some((m, op, r)), col),
            }
        };
    }
    macro_rules! v {
        ($K:ty, $k:expr) => {
            match val {
                ColRef::I64(v) => m!($K, L64, $k, v),
                ColRef::I32(v) => m!($K, L32, $k, v),
                ColRef::U32(v) => m!($K, U32, $k, v),
                ColRef::U8(v) => m!($K, L8, $k, v),
            }
        };
    }
    match key {
        ColRef::I64(k) => v!(L64, k),
        ColRef::I32(k) => v!(L32, k),
        ColRef::U32(k) => v!(U32, k),
        ColRef::U8(k) => v!(L8, k),
    }
}
