//! PROJ kinds 3 and 4 read (LV-INC-A): `fold` is what THIS reader derives from the input, `memo` the
//! OUT's value, and a value that is not the digest of its own OUT is `BAD_MEMO`.
use super::{ProjView, BAD_MEMO, STALE_MEMO};
use bebop_store::evlog::EvLog;
use bebop_store::kv::Kv;
use bebop_store::proj::kinds::{blob_of, blob_value, zset_of};
use bebop_store::proj::{fn_key, is_current, BlobFold, Identity, LogSummary, Proj, ZLinear};
use bebop_store::zset::{digest, ZSet};
use bebop_store::{get_in, root_in, Store, View};

/// Kind 4: `fold` = the Z-set digest THIS reader derives from the KV's entries (checked load).
pub(super) fn zset_view(bytes: &[u8], v: &View, p: &Proj) -> Result<ProjView, i32> {
    let kv = Kv::load_checked(&Store::from_bytes(bytes)).map_err(|_| crate::abi::NOT_A_KV)?;
    if digest(&zset_of(v, p.out)) != p.value {
        return Err(BAD_MEMO);
    }
    if !is_current(v, p) {
        return Err(STALE_MEMO);
    }
    let rows = ZSet::from_pairs(kv.entries.into_iter().map(|(k, val)| ((k.into_bytes(), val), 1)).collect());
    let n = root_in(v).map_or(0, |r| get_in(v, r, 0));
    Ok(ProjView { n, fold: digest(&Identity.apply(&rows)), memo: p.value })
}

/// Kind 3: `fold` = FNV of the LogSummary THIS reader derives from the log (fn "log_summary"),
/// else of the memo's own bytes.
pub(super) fn blob_view(bytes: &[u8], v: &View, p: &Proj) -> Result<ProjView, i32> {
    let stored = blob_of(v, p.out);
    if blob_value(&stored) != p.value {
        return Err(BAD_MEMO);
    }
    if !is_current(v, p) {
        return Err(STALE_MEMO);
    }
    let mut recs = EvLog::walk(&Store::from_bytes(bytes));
    recs.reverse();
    let mine = if p.fn_key == fn_key("log_summary") { LogSummary.full(&recs) } else { stored };
    Ok(ProjView { n: recs.len() as i64, fold: blob_value(&mine), memo: p.value })
}

