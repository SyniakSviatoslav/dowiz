//! LV-COL-E's named tests: the `sales` block, the four kernels, the plan, the image.
//! Each refusal has its positive twin; the kernels are checked against a naive per-row fold
//! written here as plain loops over `Sale`s, never against themselves.

use super::pack::{days_in, split, Pack};
use super::*;
use crate::block::decode::{check, decode};
use crate::block::encode::encode;

fn rows(days: i32, per_day: i32) -> Vec<Sale> {
    sample(20260101, days, per_day).into_iter().map(|(s, _)| s).collect()
}

fn fixture(name: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/blocks/{name}.dwb"));
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The naive fold: per dish, the sum of `pick` over the rows `keep` holds for.
fn naive(rs: &[Sale], keep: fn(&Sale) -> bool, pick: fn(&Sale) -> i64) -> BTreeMap<i64, i64> {
    let mut m = BTreeMap::new();
    for s in rs.iter().filter(|s| keep(s)) {
        *m.entry(s.dish).or_insert(0) += pick(s);
    }
    m
}

#[test]
fn sales_block_round_trips() {
    let rs = rows(3, 7);
    let bytes = encode(&sales_block(&rs)).unwrap();
    let b = decode(&bytes).unwrap();
    assert_eq!(b.n, 21);
    assert_eq!(sales_of(&b).unwrap(), rs);
    assert_eq!(encode(&b).unwrap(), bytes, "encode(decode(b)) == b");
}

#[test]
fn sales_of_refuses_another_schema() {
    let names = decode(&fixture("names_small")).unwrap();
    assert_eq!(sales_of(&names), Err(Refusal::Mismatch("not a sales block")));
    assert!(sales_of(&decode(&fixture("sales_small")).unwrap()).is_ok(), "twin: a sales block reads");
}

#[test]
fn sum_by_and_mask_equal_the_naive_fold() {
    let rs = rows(30, 9);
    let b = sales_block(&rs);
    let (dish, gross, cqty, cost) = (widen(&b.cols[1]), widen(&b.cols[3]), widen(&b.cols[4]), widen(&b.cols[6]));
    assert_eq!(sum_by(&dish, &gross, None, "gross").unwrap(), naive(&rs, |_| true, |s| s.gross));
    let m = mask_filter(&cqty, Op::Gt, 0);
    assert!(m.iter().any(|x| !*x) && m.iter().any(|x| *x), "the sample has costed and uncosted rows");
    assert_eq!(sum_by(&dish, &cost, Some(&m), "cost").unwrap(), naive(&rs, |s| s.cqty > 0, |s| s.cost));
    let qty = widen(&b.cols[2]);
    assert_eq!(sum_by(&dish, &qty, None, "qty").unwrap(), naive(&rs, |_| true, |s| i64::from(s.qty)));
}

#[test]
fn sum_by_refuses_an_overflow_by_name() {
    let keys = [7, 7, 9];
    assert_eq!(sum_by(&keys, &[i64::MAX, 1, 0], None, "gross"), Err(Refusal::Overflow { col: "gross" }));
    let ok = sum_by(&keys, &[i64::MAX - 1, 1, 5], None, "gross").unwrap();
    assert_eq!(ok, BTreeMap::from([(7, i64::MAX), (9, 5)]), "twin: one below the edge sums");
}

#[test]
fn topk_is_value_desc_then_key_asc() {
    let g = BTreeMap::from([(5, 10), (-3, 10), (1, 50), (9, -2)]);
    assert_eq!(topk(&g, 3), vec![(1, 50), (-3, 10), (5, 10)]);
    assert_eq!(topk(&g, 9).len(), 4);
    assert!(topk(&BTreeMap::new(), 3).is_empty());
}

#[test]
fn csr_join_maps_and_refuses_a_double_row() {
    let got = csr_join(&[1, 2, 3], &[1, 3], &[100, 300]).unwrap();
    assert_eq!(got, vec![Some(100), None, Some(300)]);
    assert_eq!(csr_join(&[1], &[1, 1], &[100, 200]), Err(Refusal::Mismatch("csr_join: a key with two rows")));
}

#[test]
fn run_plan_is_the_kernels_over_a_span() {
    let rs = rows(10, 6);
    let blocks: Vec<Block> = rs.chunk_by(|a, b| a.day == b.day).map(sales_block).collect();
    assert_eq!(blocks.len(), 10, "one block per day");
    // food cost by dish over the costed rows: FILTER cqty > 0, GROUP dish, AGG cost.
    let plan: [PlanRow; 3] = [[FILTER, 4, Op::Gt as i64, 0, 0, 0, 0, 0], [GROUP, 1, 0, 0, 0, 0, 0, 0], [AGG, 6, 0, 0, 0, 0, 0, 0]];
    let want: Vec<(i64, i64)> = naive(&rs, |s| s.cqty > 0, |s| s.cost).into_iter().collect();
    assert_eq!(run_plan(&blocks, &plan).unwrap(), want);
    let top: [PlanRow; 3] = [[GROUP, 1, 0, 0, 0, 0, 0, 0], [AGG, 3, 0, 0, 0, 0, 0, 0], [TOPK, 2, 0, 0, 0, 0, 0, 0]];
    let all = naive(&rs, |_| true, |s| s.gross);
    assert_eq!(run_plan(&blocks, &top).unwrap(), topk(&all, 2));
    let no_agg: [PlanRow; 1] = [[GROUP, 1, 0, 0, 0, 0, 0, 0]];
    assert_eq!(run_plan(&blocks, &no_agg), Err(Refusal::Mismatch("plan: GROUP and AGG are required")));
}

#[test]
fn pack_round_trips_and_absorbs_once() {
    let mut p = Pack::default();
    let a = sample(20261020, 3, 4);
    assert_eq!(p.absorb("log@1", &a), Ok(true));
    assert_eq!(p.absorb("log@1", &a), Ok(false), "an archive folded twice is refused");
    assert_eq!(p.absorb("log@2", &sample(20261022, 2, 3)), Ok(true));
    let bytes = p.encode().unwrap();
    let q = Pack::decode(&bytes).unwrap();
    assert_eq!(q, p);
    assert_eq!(q.days.len(), 4, "20..23: day 22 holds both archives' rows");
    // Day 22 is one row per dish over BOTH archives' lines, each the sum of its lines.
    let lines: Vec<Sale> = a.iter().chain(sample(20261022, 2, 3).iter()).map(|(s, _)| *s).filter(|s| s.day == 20261022).collect();
    assert_eq!(q.days[&20261022], aggregate(&lines).unwrap());
    assert!(q.days[&20261022].len() < lines.len(), "a dish sold in both archives is one row");
    assert_eq!(split(&bytes).unwrap().len(), 2 + 4);
    assert_eq!(Pack::decode(&[]).unwrap(), Pack::default());
}

#[test]
fn days_in_reads_only_the_span() {
    let mut p = Pack::default();
    p.absorb("log@1", &sample(20261001, 20, 3)).unwrap();
    let bytes = p.encode().unwrap();
    let got = days_in(&bytes, 20261005, 20261007).unwrap();
    let days: Vec<i32> = got.iter().map(|b| sales_of(b).unwrap()[0].day).collect();
    assert_eq!(days, vec![20261005, 20261006, 20261007]);
}

#[test]
fn a_corrupted_day_byte_refuses_the_image() {
    let mut p = Pack::default();
    p.absorb("log@1", &sample(20261001, 2, 3)).unwrap();
    let bytes = p.encode().unwrap();
    assert!(Pack::decode(&bytes).is_ok(), "twin: the clean image reads");
    // The last byte before the final crc: inside the last day block's `cost` column.
    let mut bad = bytes.clone();
    let at = bad.len() - CRC_LEN - 1;
    bad[at] ^= 0x01;
    assert!(matches!(Pack::decode(&bad), Err(Refusal::BadCrc { .. })), "{:?}", Pack::decode(&bad));
    assert!(matches!(days_in(&bad, 0, i32::MAX), Err(Refusal::BadCrc { .. })));
}

const CRC_LEN: usize = crate::block::CRC;

#[test]
fn fixtures_are_the_encoder_output() {
    for (name, rs) in [("sales_empty", vec![]), ("sales_small", rows(1, 7)), ("sales", rows(365, 5))] {
        let want = encode(&sales_block(&rs)).unwrap();
        assert_eq!(fixture(name), want, "{name}: rerun `cargo run --example generate_fixtures fixtures fixtures/blocks`");
        check(&want).unwrap();
    }
}

#[test]
fn the_kernel_line_of_the_small_fixture() {
    let rs = rows(1, 7);
    let g = naive(&rs, |_| true, |s| s.gross);
    let line = line(&fixture("sales_small"));
    assert!(line.starts_with(&format!("kernel sales rows=7 groups={} ", g.len())), "{line}");
    let costed = rs.iter().filter(|s| s.cqty > 0).count();
    assert!(line.contains(&format!(" costed={costed} ")), "{line}");
    assert_eq!(super::line(&fixture("names_small")), "kernel skip=names");
    assert_eq!(super::line(b"DWB1"), "kernel refused=too_short col=-");
}

#[test]
fn a_dish_id_clash_refuses_the_whole_archive() {
    let mut p = Pack::default();
    let mut rows = sample(20261001, 1, 2);
    p.absorb("log@1", &rows).unwrap();
    let before = p.clone();
    // The same K64 named by another id: in the image already, and within one archive.
    rows[0].1 = "not-the-same-dish".to_string();
    assert!(matches!(p.absorb("log@2", &rows), Err(Refusal::KeyCollision { .. })));
    assert_eq!(p, before, "nothing of the refused archive was folded");
    let mut fresh = Pack::default();
    let mut twin = sample(20261001, 1, 2);
    twin[1] = (Sale { dish: twin[0].0.dish, ..twin[1].0 }, "other".to_string());
    assert!(matches!(fresh.absorb("log@1", &twin), Err(Refusal::KeyCollision { .. })));
    assert_eq!(fresh.absorb("log@1", &sample(20261001, 1, 2)), Ok(true), "twin: distinct ids fold");
}

/// AC-LV-COL-05 at the kernel level, MEASURED in release: a busy venue's year -- 365 days x 90
/// order lines over 40 dishes, summed by `absorb` into 365 day blocks of one row per dish (14,600
/// rows) -- checked, decoded and folded (dish -> gross; the costed cost and sales) beside a CONTROL
/// in the same process: the same year as the cube's JSON day rows (`{"d":..,"m":{dish:[4]}}`).
/// `cargo test --release --lib year_kernels -- --ignored --nocapture`.
#[test]
#[ignore]
fn year_kernels_us() {
    let mut p = Pack::default();
    let start = crate::stock::meta::day_number(20251001);
    let mut lines = Vec::new();
    for k in 0..365i64 {
        let day = crate::stock::meta::day_of_number(start + k) as i32;
        for i in 0..90i64 {
            let id = format!("dish-{:03}", (k * 7 + i) % 40);
            let qty = 1 + (i % 3) as i32;
            let gross = 450 * i64::from(qty);
            let (cqty, cgross) = if i % 4 == 0 { (0, 0) } else { (qty, gross) };
            lines.push((Sale { day, dish: schema::k64(id.as_bytes()) as i64, qty, gross, cqty, cgross, cost: cgross / 3 }, id));
        }
    }
    p.absorb("log@1", &lines).unwrap();
    let img = p.encode().unwrap();
    let mut json = String::new();
    for (d, rs) in &p.days {
        let m: BTreeMap<String, [i64; 4]> = rs.iter().map(|s| (p.dishes[&s.dish].clone(), [i64::from(s.qty), s.gross, i64::from(s.cqty), s.cost])).collect();
        json.push_str(&serde_json::json!({ "d": d, "m": m }).to_string());
        json.push('\n');
    }
    let t0 = std::time::Instant::now();
    let blocks = days_in(&img, 20251001, 20260930).unwrap();
    let read = t0.elapsed();
    let gross = full_sum_by(&blocks, 1, 3, None).unwrap();
    let cost = full_sum_by(&blocks, 1, 6, Some((4, Op::Gt, 0))).unwrap();
    let sales = full_sum_by(&blocks, 1, 5, Some((4, Op::Gt, 0))).unwrap();
    let scan = t0.elapsed();
    let t1 = std::time::Instant::now();
    let mut m: BTreeMap<String, [i64; 4]> = BTreeMap::new();
    for l in json.lines() {
        let v: serde_json::Value = serde_json::from_str(l).unwrap();
        for (id, x) in v["m"].as_object().unwrap() {
            let e = m.entry(id.clone()).or_default();
            for i in 0..4 {
                e[i] += x[i].as_i64().unwrap();
            }
        }
    }
    let control = t1.elapsed();
    let by_id: BTreeMap<String, i64> = gross.iter().map(|(k, v)| (p.dishes[k].clone(), *v)).collect();
    assert_eq!(by_id, m.iter().map(|(k, v)| (k.clone(), v[1])).collect(), "blocks and JSON agree");
    let rows: usize = p.days.values().map(Vec::len).sum();
    assert_eq!((lines.len(), rows), (32_850, 14_600));
    // W-BLOCKVIEW: the decode path (CONTROL) against the image read in place, each the median of
    // 11 runs in this process. `open` is the full check (once per image generation); `view` is
    // `Index::view` + the same three sums, what a holder of the generation's `Index` pays per query.
    let med = |f: &mut dyn FnMut()| -> u128 {
        let mut t: Vec<u128> = (0..11)
            .map(|_| {
                let s = std::time::Instant::now();
                f();
                s.elapsed().as_micros()
            })
            .collect();
        t.sort_unstable();
        t[5]
    };
    let (from, to) = (20251001, 20260930);
    let decode_us = med(&mut || {
        std::hint::black_box(three(&days_in(&img, from, to).unwrap()).unwrap());
    });
    let open_us = med(&mut || {
        std::hint::black_box(Index::open(&img).unwrap());
    });
    let idx = Index::open(&img).unwrap();
    let view_us = med(&mut || {
        std::hint::black_box(three_days(&idx.view(&img).unwrap().days(from, to)).unwrap());
    });
    assert_eq!(three_days(&idx.view(&img).unwrap().days(from, to)).unwrap(), [gross.clone(), cost.clone(), sales.clone()], "in place == decode");
    println!("YEAR-VIEW rows=14600 blocks={} control decode_path_us={decode_us} | open_us={open_us} (full check) view_scan_us={view_us} (reopen + 3 sums) open+scan={}", idx.days(), open_us + view_us);
    println!("YEAR-KERNELS lines={} rows={} image={}B year_scan_us={} (check+decode {}) groups={}/{}/{} | control json={}B json_parse_us={}", lines.len(), rows, img.len(), scan.as_micros(), read.as_micros(), gross.len(), cost.len(), sales.len(), json.len(), control.as_micros());
}

#[test]
fn aggregate_is_one_row_per_dish_in_any_order_and_refuses_an_overflow() {
    let lines = rows(3, 12);
    let mut shuffled = lines.clone();
    shuffled.reverse();
    let dup: Vec<Sale> = lines.iter().chain(lines.iter()).copied().collect();
    let a = aggregate(&lines).unwrap();
    assert_eq!(aggregate(&shuffled).unwrap(), a, "the order of the lines does not move a row");
    let twice = aggregate(&dup).unwrap();
    assert_eq!(twice.len(), a.len());
    assert!(twice.iter().zip(&a).all(|(t, o)| t.qty == 2 * o.qty && t.cost == 2 * o.cost && t.cgross == 2 * o.cgross));
    assert!(a.windows(2).all(|w| (w[0].day, w[0].dish) < (w[1].day, w[1].dish)), "day, then dish, ascending");
    let big = Sale { gross: i64::MAX, ..lines[0] };
    assert_eq!(aggregate(&[big, lines[0]]), Err(Refusal::Overflow { col: "gross" }));
}

// ---- W-BLOCKVIEW: the image read in place (`kernels::image`) against the decode path. ----

use super::image::{full_sum_by_days, run_plan_days, sum_into_cols, ColRef, Day, Groups, Index};

/// The decode path's three year sums of `blocks`: gross by dish, costed cost and costed sales.
fn three(blocks: &[Block]) -> Result<[BTreeMap<i64, i64>; 3], Refusal> {
    let f = Some((4, Op::Gt, 0));
    Ok([full_sum_by(blocks, 1, 3, None)?, full_sum_by(blocks, 1, 6, f)?, full_sum_by(blocks, 1, 5, f)?])
}

fn three_days(days: &[Day]) -> Result<[BTreeMap<i64, i64>; 3], Refusal> {
    let f = Some((4, Op::Gt, 0));
    Ok([full_sum_by_days(days, 1, 3, None)?, full_sum_by_days(days, 1, 6, f)?, full_sum_by_days(days, 1, 5, f)?])
}

/// What the in-place reader must say of one loose block: the decode path's sums, or its refusal.
fn same_as_decode(bytes: &[u8]) -> Result<(), String> {
    let want = match decode(bytes) {
        Ok(b) if b.schema == &schema::SALES && b.n > 0 => three(&[b]),
        Ok(_) => Err(Refusal::Mismatch("day blocks are one day each, ascending")),
        Err(r) => Err(r),
    };
    let got = Day::new(bytes).and_then(|d| three_days(&[d]));
    if got == want { Ok(()) } else { Err(format!("view {got:?} != decode {want:?}")) }
}

#[test]
fn views_equal_the_decode_path_on_every_fixture_and_every_flipped_byte() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/blocks");
    let mut seen = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        let bytes = std::fs::read(&p).unwrap();
        same_as_decode(&bytes).unwrap_or_else(|m| panic!("{}: {m}", p.display()));
        seen += 1;
    }
    assert!(seen >= 15, "every fixture: {seen}");
    let small = fixture("sales_small");
    assert!(Day::new(&small).is_ok(), "twin: the clean sales block reads in place");
    for at in 0..small.len() {
        let mut bad = small.clone();
        bad[at] ^= 0x01;
        same_as_decode(&bad).unwrap_or_else(|m| panic!("byte {at}: {m}"));
    }
}

/// An image of `days` days from 2026-10-01, three dishes a day.
fn image(days: i32) -> Vec<u8> {
    let mut p = Pack::default();
    p.absorb("log@1", &sample(20261001, days, 3)).unwrap();
    p.encode().unwrap()
}

#[test]
fn an_opened_image_reads_the_span_as_days_in_does() {
    let bytes = image(20);
    let idx = Index::open(&bytes).unwrap();
    assert_eq!(idx.days(), 20);
    let img = idx.view(&bytes).unwrap();
    for (from, to) in [(20261005, 20261007), (0, i32::MAX), (20261020, 20261020), (20261008, 20261003), (20261101, 20261231)] {
        let (blocks, days) = (days_in(&bytes, from, to).unwrap(), img.days(from, to));
        let want: Vec<i32> = blocks.iter().map(|b| sales_of(b).unwrap()[0].day).collect();
        assert_eq!(days.iter().map(Day::day).collect::<Vec<_>>(), want, "{from}..={to}");
        assert_eq!(three_days(&days), three(&blocks));
        for plan in [
            vec![[FILTER, 4, Op::Gt as i64, 0, 0, 0, 0, 0], [GROUP, 1, 0, 0, 0, 0, 0, 0], [AGG, 6, 0, 0, 0, 0, 0, 0]],
            vec![[GROUP, 1, 0, 0, 0, 0, 0, 0], [AGG, 3, 0, 0, 0, 0, 0, 0], [TOPK, 2, 0, 0, 0, 0, 0, 0]],
            vec![[GROUP, 0, 0, 0, 0, 0, 0, 0], [AGG, 2, 0, 0, 0, 0, 0, 0]],
            vec![[GROUP, 1, 0, 0, 0, 0, 0, 0]],
        ] {
            assert_eq!(run_plan_days(&days, &plan), run_plan(&blocks, &plan), "{plan:?}");
        }
    }
    assert_eq!(Index::open(&[]).map(|i| i.days()), Ok(0), "no bytes is an empty image");
}

#[test]
fn the_safe_open_refuses_what_days_in_refuses() {
    let bytes = image(3);
    assert!(Index::open(&bytes).is_ok(), "twin: the clean image opens");
    let mut bad = bytes.clone();
    let at = bad.len() - CRC_LEN - 1;
    bad[at] ^= 0x01;
    assert!(matches!(Index::open(&bad), Err(Refusal::BadCrc { .. })), "{:?}", Index::open(&bad));
    assert!(matches!(days_in(&bad, 0, i32::MAX), Err(Refusal::BadCrc { .. })));
    assert_eq!(Index::open(&bytes[..bytes.len() - 1]).err().map(|r| r.code()), Some("too_short"));
}

#[test]
fn an_index_reads_only_the_bytes_it_was_opened_on() {
    let (a, b) = (image(4), image(5));
    let idx = Index::open(&a).unwrap();
    assert!(idx.view(&a).is_ok(), "twin: its own bytes");
    assert_eq!(idx.view(&b).err(), Some(Refusal::Mismatch("an index of other bytes")), "another length");
    // Same length, one crc trailer changed (the last block's): another generation, refused.
    let mut c = a.clone();
    let end = c.len() - 1;
    c[end] ^= 0x80;
    assert_eq!(idx.view(&c).err(), Some(Refusal::Mismatch("an index of other bytes")));
}

#[test]
fn days_out_of_order_are_refused_at_open() {
    let mut p = Pack::default();
    p.absorb("log@1", &sample(20261001, 2, 3)).unwrap();
    let ok = p.encode().unwrap();
    assert!(Index::open(&ok).is_ok(), "twin: ascending days open");
    let blocks: Vec<Vec<u8>> = split(&ok).unwrap().iter().map(|b| encode(b).unwrap()).collect();
    let swapped: Vec<u8> = [&blocks[0], &blocks[1], &blocks[3], &blocks[2]].into_iter().flatten().copied().collect();
    assert_eq!(Index::open(&swapped).err(), Some(Refusal::Mismatch("day blocks are one day each, ascending, one row per dish")));
    assert_eq!(Index::open(&swapped).err(), Pack::decode(&swapped).err());
    let one_names: Vec<u8> = blocks[0].clone();
    assert_eq!(Index::open(&one_names).err(), Some(Refusal::Mismatch("a sales image opens with two names blocks")));
}

#[test]
fn groups_past_the_run_spill_and_keep_every_sum_and_refusal() {
    let keys: Vec<i64> = (0..6000).map(|i| (i * 7919) % 3000 - 1500).collect();
    let vals: Vec<i64> = (0..6000).map(|i| i * 3 - 9000).collect();
    let le = |v: &[i64]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let (kb, vb) = (le(&keys), le(&vals));
    let mut acc = Groups::default();
    sum_into_cols(&mut acc, ColRef::I64(&kb), ColRef::I64(&vb), None, "gross").unwrap();
    let got = acc.into_map();
    assert_eq!(got.len(), 3000, "past RUN_MAX: spilled");
    assert_eq!(got, sum_by(&keys, &vals, None, "gross").unwrap());
    let mask: Vec<u8> = (0..6000i32).flat_map(|i| (i % 5).to_le_bytes()).collect();
    let mut acc = Groups::default();
    sum_into_cols(&mut acc, ColRef::I64(&kb), ColRef::I64(&vb), Some((ColRef::I32(&mask), Op::Gt, 2)), "cost").unwrap();
    let m: Vec<bool> = (0..6000).map(|i| i % 5 > 2).collect();
    assert_eq!(acc.into_map(), sum_by(&keys, &vals, Some(&m), "cost").unwrap());
    let (k, v) = (le(&[7, 7, 9]), le(&[i64::MAX, 1, 0]));
    let mut acc = Groups::default();
    assert_eq!(sum_into_cols(&mut acc, ColRef::I64(&k), ColRef::I64(&v), None, "gross"), Err(Refusal::Overflow { col: "gross" }));
}

// ---- W-BLOCKVIEW: `Index::open` refuses a malformed image exactly as `Pack::decode` does. ----

/// `sample(20261001, 3, 4)`'s image with day block `k`'s rows changed by `f`, resealed (crc good).
fn with_day(k: usize, f: impl FnOnce(&mut Vec<Sale>)) -> Vec<u8> {
    let mut p = Pack::default();
    p.absorb("log@1", &sample(20261001, 3, 4)).unwrap();
    let mut blocks = split(&p.encode().unwrap()).unwrap();
    let mut rows = sales_of(&blocks[2 + k]).unwrap();
    f(&mut rows);
    blocks[2 + k] = sales_block(&rows);
    blocks.iter().flat_map(|b| encode(b).unwrap()).collect()
}

fn refused_alike(bytes: &[u8], want: Refusal) {
    assert_eq!(Pack::decode(bytes).err(), Some(want.clone()), "Pack::decode");
    assert_eq!(Index::open(bytes).err(), Some(want), "Index::open");
}

const ROWS: Refusal = Refusal::Mismatch("day blocks are one day each, ascending, one row per dish");

#[test]
fn a_row_with_another_day_is_refused_as_pack_decode_refuses_it() {
    assert!(Index::open(&with_day(1, |_| ())).is_ok() && Pack::decode(&with_day(1, |_| ())).is_ok(), "twin: resealed unchanged");
    // Day block 1 (2026-10-02), row 2's `day` cell set to 2026-10-09.
    refused_alike(&with_day(1, |r| r[2].day = 20261009), ROWS);
}

#[test]
fn a_dish_out_of_order_is_refused_as_pack_decode_refuses_it() {
    // Day block 0, rows 0 and 1 swapped: the `dish` cells descend.
    refused_alike(&with_day(0, |r| r.swap(0, 1)), ROWS);
}

#[test]
fn a_dish_the_names_do_not_hold_is_refused_as_pack_decode_refuses_it() {
    // Day block 2, the last row's `dish` cell set to i64::MAX: still ascending, named nowhere.
    refused_alike(&with_day(2, |r| r.last_mut().unwrap().dish = i64::MAX), Refusal::Mismatch("a row names a dish the image does not"));
}

#[test]
fn an_empty_or_foreign_day_block_is_refused_as_pack_decode_refuses_it() {
    refused_alike(&with_day(1, Vec::clear), Refusal::Mismatch("an empty day block"));
    let ok = with_day(0, |_| ());
    let blocks = split(&ok).unwrap();
    let foreign: Vec<u8> = [&blocks[0], &blocks[1], &blocks[0]].iter().flat_map(|b| encode(b).unwrap()).collect();
    refused_alike(&foreign, Refusal::Mismatch("not a sales block"));
}

#[test]
fn every_block_is_checked_before_any_row_is_read() {
    // A bad row in day block 0 AND a bad crc in the last block: `split` reports the crc first.
    let mut bad = with_day(0, |r| r.swap(0, 1));
    let at = bad.len() - CRC_LEN - 1;
    bad[at] ^= 0x01;
    assert!(matches!(Pack::decode(&bad), Err(Refusal::BadCrc { .. })));
    assert_eq!(Index::open(&bad).err(), Pack::decode(&bad).err());
}

#[test]
fn no_bytes_and_every_short_prefix_open_as_pack_decode_reads_them() {
    // The empty image once underflowed `len - CRC` at open (an end offset taken with no block).
    assert_eq!(Index::open(&[]).map(|i| i.days()), Ok(0));
    assert_eq!(Pack::decode(&[]), Ok(Pack::default()));
    let img = with_day(0, |_| ());
    for n in 1..64.min(img.len()) {
        assert_eq!(Index::open(&img[..n]).err(), Pack::decode(&img[..n]).err(), "prefix {n}");
        assert!(Index::open(&img[..n]).is_err(), "prefix {n}");
    }
}
