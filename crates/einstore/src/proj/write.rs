//! The memo WRITER, for every kind (moved out of `proj.rs` by LV-INC-A for the 300-line
//! ratchet; kinds 1/2 write byte for byte what they wrote before -- `proj/tests.rs`
//! `kinds_1_and_2_write_the_same_bytes` was pinned before the move).
use super::{projtab, root_in, Store, StoreError, DIGEST_OUT, DIGEST_PROJ, DIGEST_PROJTAB, FMT, PROJ_CELLS};

/// What a memo's OUT object holds.
pub(super) enum Out<'a> {
    /// Kinds 1/2: OUT `{value, count}`.
    Pair(i64, i64),
    /// Kinds 3/4: an object of layout `digest` whose payload is `cells` (value, count first).
    Cells(i64, &'a [i64]),
}

/// Write the memo: OUT, PROJ, and a new sorted table, in ONE commit that keeps the data root.
#[allow(clippy::too_many_arguments)]
pub(super) fn put(st: &mut Store, key: i64, kind: i64, code: i64, gen: i64, tip: [i64; 2], fn_key: i64, out: Out) -> Result<(), StoreError> {
    let root = root_in(st).ok_or(StoreError::NoSuperblock)?;
    let old = projtab(st);
    let mut rows: Vec<(i64, usize, i64)> = Vec::new();
    if let Some(t) = old {
        let m = st.get(t, 0).max(0) as usize;
        for i in 0..m.min(st.obj_cells(t).saturating_sub(1) / 3) {
            let k = st.get(t, 1 + 3 * i);
            if let Some(p) = st.follow(t, 2 + 3 * i) {
                // A row of the same fn and kind under another code is a previous deploy's.
                let same = st.get(p, 8) == fn_key && st.get(p, 1) == kind;
                if k != key && !same {
                    rows.push((k, p, st.get(t, 3 + 3 * i)));
                }
            }
        }
    }
    let mut tx = st.begin()?;
    let (dig, cells): (i64, Vec<i64>) = match out {
        Out::Pair(value, count) => (DIGEST_OUT, vec![value, count]),
        Out::Cells(d, c) => (d, c.to_vec()),
    };
    let out = st.alloc(&mut tx, cells.len() as i64, dig)?;
    for (i, v) in cells.into_iter().enumerate() {
        st.put_cell(out, i, v);
    }
    st.seal(out);
    let p = st.alloc(&mut tx, PROJ_CELLS, DIGEST_PROJ)?;
    for (i, v) in [key, kind, code, FMT, gen, tip[0], tip[1], 0, fn_key].into_iter().enumerate() {
        st.put_cell(p, i, v);
    }
    st.link(p, 7, out);
    st.seal(p);
    rows.push((key, p, kind));
    rows.sort_by_key(|r| r.0);
    let t = st.alloc(&mut tx, 1 + 3 * rows.len() as i64, DIGEST_PROJTAB)?;
    st.put_cell(t, 0, rows.len() as i64);
    for (i, &(k, at, kd)) in rows.iter().enumerate() {
        st.put_cell(t, 1 + 3 * i, k);
        st.link(t, 2 + 3 * i, at);
        st.put_cell(t, 3 + 3 * i, kd);
    }
    st.seal(t);
    if let Some(ot) = old {
        tx.sup_delta += 2 + st.obj_len(ot);
    }
    st.commit_bytes_projtab(&tx, root, t as i64);
    Ok(())
}
