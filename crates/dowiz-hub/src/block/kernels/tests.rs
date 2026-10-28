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
