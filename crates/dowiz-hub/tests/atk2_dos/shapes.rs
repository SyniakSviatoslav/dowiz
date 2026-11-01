//! The crafted files of `atk2_dos`: no key, no valid signature, every crc right.
use bebop_store::evlog::doc::anch::{ANCH_VERSION, DIGEST_ANCH};
use bebop_store::evlog::doc::keys::{DIGEST_KEY, DIGEST_KEYS};
use bebop_store::evlog::doc::seal::{read_heads, DIGEST_HEADS, DIGEST_SEAL, SEAL_VERSION};
use bebop_store::evlog::doc::seg::{DIGEST_SEG, SEG_VERSION};
use bebop_store::evlog::doc::{id_cells, Doc, DIGEST_DOC, DOC_CELLS, ROOT_VERSION_DOC};
use bebop_store::evlog::Record;
use bebop_store::{Store, Tx};
use dowiz_hub::doc_seal::DeviceKey;

pub fn recs(n: usize) -> Vec<Record> {
    let mut prev = [0u8; 32];
    (0..n)
        .map(|i| {
            let mut id = [0u8; 32];
            id[..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
            let r = Record { id, prev, actor_pubkey: [0; 32], actor_seq: i as u64 + 1, flags: 0, payload: Vec::new() };
            prev = id;
            r
        })
        .collect()
}

pub fn base(cells: usize, n: usize) -> Store {
    let mut st = Store::create_bytes((cells + n * 40 + 16384) * 8).unwrap();
    let k = DeviceKey::from_seed(&[0xA1; 32]);
    Doc::init_over_bytes(&mut st, 1, &[k.key()], &recs(n)).unwrap();
    st
}

#[derive(Default)]
pub struct Refs {
    pub heads: Option<usize>,
    pub keys: Option<usize>,
    pub seg: Option<usize>,
    pub anch: Option<usize>,
}
pub fn commit(st: &mut Store, tx: &mut Tx, r: Refs) -> Vec<u8> {
    let v = Doc::open(st).unwrap();
    let d = st.alloc(tx, DOC_CELLS, DIGEST_DOC).unwrap();
    (0..DOC_CELLS as usize).for_each(|i| st.put_cell(d, i, 0));
    st.put_cell(d, 0, 1);
    st.put_cell(d, 5, ROOT_VERSION_DOC);
    st.put_cell(d, 7, ROOT_VERSION_DOC);
    st.link(d, 1, v.log.unwrap());
    for (c, o) in [(3, r.heads.or(v.heads)), (4, r.keys.or(v.keys)), (8, r.anch), (9, r.seg)] {
        if let Some(o) = o {
            st.link(d, c, o);
        }
    }
    st.seal(d);
    st.commit_bytes(tx, d);
    st.to_bytes_trimmed()
}

/// Objects `head + 2` cells apart inside one big one, each claiming the rest of it, each naming the
/// next through cell `next`. `fill(st, payload_at, len)` writes one's cells. Sealed high to low,
/// so every crc is right though each object holds all the later ones.
pub fn nest(st: &mut Store, tx: &mut Tx, cells: usize, digest: i64, head: usize, row: usize, next: usize, fill: &dyn Fn(&mut Store, usize, usize)) -> usize {
    let o = st.alloc(tx, cells as i64, 1).unwrap();
    let (first, end, step) = (o + 2, o + 2 + cells, head + 2);
    let k = (cells - head - 2 - row) / step;
    for i in (0..k).rev() {
        let p = first + i * step;
        let len = head + (end - p - 2 - head) / row * row;
        st.cells[p] = (digest << 32) | len as i64;
        st.cells[p + 1] = 0;
        (0..head).for_each(|c| st.cells[p + 2 + c] = 0);
        fill(st, p, len);
        if i + 1 < k {
            st.cells[p + 2 + next] = step as i64;
        }
        st.seal(p);
    }
    first
}

/// Two SEALs naming each other, `h` HEADS rows on one of them, `r` records (the walk's cap).
pub fn seal_ring(h: usize, r: usize, sig_cells: usize) -> Vec<u8> {
    let mut st = base(10 * h + 2 * sig_cells + 256, r);
    let mut tx = st.begin().unwrap();
    let mut one = |st: &mut Store| {
        let o = st.alloc(&mut tx, 15 + sig_cells as i64, DIGEST_SEAL).unwrap();
        (0..15 + sig_cells).for_each(|i| st.put_cell(o, i, 0));
        st.put_cell(o, 0, SEAL_VERSION);
        st.put_cell(o, 14, sig_cells as i64 * 8);
        o
    };
    let (a, b) = (one(&mut st), one(&mut st));
    st.link(a, 11, b);
    st.link(b, 11, a);
    st.seal(a);
    st.seal(b);
    let t = st.alloc(&mut tx, 1 + 10 * h as i64, DIGEST_HEADS).unwrap();
    (0..1 + 10 * h).for_each(|i| st.put_cell(t, i, 0));
    st.put_cell(t, 0, h as i64);
    (0..h).for_each(|i| st.link(t, 1 + i * 10 + 9, a));
    st.seal(t);
    commit(&mut st, &mut tx, Refs { heads: Some(t), ..Refs::default() })
}

pub fn nested_segs(cells: usize) -> Vec<u8> {
    let mut st = base(cells + 256, 1);
    let mut tx = st.begin().unwrap();
    let fill = |st: &mut Store, p: usize, len: usize| {
        st.cells[p + 2] = SEG_VERSION;
        st.cells[p + 2 + 11] = ((len - 12) / 10) as i64;
    };
    let s = nest(&mut st, &mut tx, cells, DIGEST_SEG, 12, 10, 7, &fill);
    commit(&mut st, &mut tx, Refs { seg: Some(s), ..Refs::default() })
}

pub fn nested_anchors(cells: usize) -> Vec<u8> {
    let mut st = base(cells + 256, 1);
    let mut tx = st.begin().unwrap();
    let fill = |st: &mut Store, p: usize, len: usize| {
        st.cells[p + 2] = ANCH_VERSION;
        st.cells[p + 2 + 8] = (len as i64 - 9) * 8;
    };
    let s = nest(&mut st, &mut tx, cells, DIGEST_ANCH, 9, 1, 7, &fill);
    commit(&mut st, &mut tx, Refs { anch: Some(s), ..Refs::default() })
}

/// The dearest file that is READ, not refused: every record its own actor, so each one costs a
/// chain, a map node per index, and its `Record`, against the 18 cells it takes in the file.
pub fn many_actors(n: usize) -> Vec<u8> {
    let mut st = Store::create_bytes((n * 40 + 16384) * 8).unwrap();
    let k = DeviceKey::from_seed(&[0xA1; 32]);
    let mut all = recs(n);
    for (i, r) in all.iter_mut().enumerate() {
        r.actor_pubkey[..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
        r.actor_seq = 1;
    }
    Doc::init_over_bytes(&mut st, 1, &[k.key()], &all).unwrap();
    st.to_bytes_trimmed()
}

/// `rows` KEYS rows on one KEY of 64 KiB: `read_keys` clones the pk per row.
pub fn one_key_many_rows(rows: usize) -> Vec<u8> {
    let mut st = base(5 * rows + 8300, 1);
    let mut tx = st.begin().unwrap();
    let k = st.alloc(&mut tx, 9 + 8192, DIGEST_KEY).unwrap();
    (0..9 + 8192).for_each(|i| st.put_cell(k, i, 0));
    st.put_cell(k, 4, 65);
    st.put_cell(k, 8, 65536);
    st.seal(k);
    let t = st.alloc(&mut tx, 1 + 5 * rows as i64, DIGEST_KEYS).unwrap();
    (0..1 + 5 * rows).for_each(|i| st.put_cell(t, i, 0));
    st.put_cell(t, 0, rows as i64);
    (0..rows).for_each(|i| st.link(t, 1 + i * 5 + 4, k));
    st.seal(t);
    commit(&mut st, &mut tx, Refs { keys: Some(t), ..Refs::default() })
}


/// `n` records, then one big KEY on `rows` rows: the rows alone fit the image, the rows and the
/// records do not. What says the walks share ONE budget.
pub fn records_then_keys(n: usize, rows: usize) -> Vec<u8> {
    let mut st = base(5 * rows + 8300, n);
    let mut tx = st.begin().unwrap();
    let k = st.alloc(&mut tx, 9 + 8192, DIGEST_KEY).unwrap();
    (0..9 + 8192).for_each(|i| st.put_cell(k, i, 0));
    st.put_cell(k, 4, 65);
    st.put_cell(k, 8, 65536);
    st.seal(k);
    let t = st.alloc(&mut tx, 1 + 5 * rows as i64, DIGEST_KEYS).unwrap();
    (0..1 + 5 * rows).for_each(|i| st.put_cell(t, i, 0));
    st.put_cell(t, 0, rows as i64);
    (0..rows).for_each(|i| st.link(t, 1 + i * 5 + 4, k));
    st.seal(t);
    commit(&mut st, &mut tx, Refs { keys: Some(t), ..Refs::default() })
}

/// SEALs laid over each other, each naming the next: no ring, so only the budget stops the copies.
pub fn nested_seals(cells: usize) -> Vec<u8> {
    let mut st = base(10 + cells + 256, 1);
    let mut tx = st.begin().unwrap();
    let fill = |st: &mut Store, p: usize, len: usize| {
        let len = len.min(15 + 8192);
        st.cells[p] = (DIGEST_SEAL << 32) | len as i64;
        st.cells[p + 2] = SEAL_VERSION;
        st.cells[p + 2 + 14] = (len as i64 - 15) * 8;
    };
    let s = nest(&mut st, &mut tx, cells, DIGEST_SEAL, 15, 1, 11, &fill);
    let t = st.alloc(&mut tx, 11, DIGEST_HEADS).unwrap();
    (0..11).for_each(|i| st.put_cell(t, i, 0));
    st.put_cell(t, 0, 1);
    st.link(t, 10, s);
    st.seal(t);
    commit(&mut st, &mut tx, Refs { heads: Some(t), ..Refs::default() })
}

/// A real file with a second HEADS row on the first row's seal, naming another seq.
pub fn second_row(bytes: &[u8]) -> Vec<u8> {
    let src = Store::from_bytes(bytes);
    let mut st = Store::create_bytes(1 << 22).unwrap();
    Doc::copy_bytes(&src, &mut st).unwrap();
    let v = Doc::open(&st).unwrap();
    let h = read_heads(&st, v.heads)[0];
    let mut tx = st.begin().unwrap();
    let t = st.alloc(&mut tx, 21, DIGEST_HEADS).unwrap();
    (0..21).for_each(|i| st.put_cell(t, i, 0));
    st.put_cell(t, 0, 2);
    for (row, seq) in [(0usize, h.seq), (1, h.seq + 1)] {
        let b = 1 + row * 10;
        id_cells(&h.actor_id).iter().enumerate().for_each(|(i, c)| st.put_cell(t, b + i, *c));
        st.put_cell(t, b + 4, seq as i64);
        id_cells(&h.tip).iter().enumerate().for_each(|(i, c)| st.put_cell(t, b + 5 + i, *c));
        st.link(t, b + 9, h.seal.unwrap());
    }
    st.seal(t);
    commit(&mut st, &mut tx, Refs { heads: Some(t), ..Refs::default() })
}
