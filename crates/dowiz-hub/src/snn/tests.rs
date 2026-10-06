//! W-SNN rows 1, 3 and 5: the integers agree with a second implementation, a broken blob is refused
//! by name, the shadow never changes what the guest sees, and `off` runs nothing.

use std::collections::BTreeMap;

use serde_json::Value;

use super::shadow::{decide, Mode, Outcome, Tally};
use super::*;

fn fixture(name: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn ints(v: &Value) -> Vec<i64> {
    v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect()
}

fn pairs(v: &Value) -> Vec<(usize, i64)> {
    v.as_array().unwrap().iter().map(|p| (p[0].as_u64().unwrap() as usize, p[1].as_i64().unwrap())).collect()
}

/// The golden's inputs as the network's: dishes, group counts, the guest.
fn golden_case(c: &Value) -> (Menu, Guest) {
    let ng = ints(&c["ngroups"]);
    let dishes = c["dishes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| Dish {
            id: d["id"].as_str().unwrap().to_string(),
            v: ints(&d["v"]).try_into().unwrap(),
            groups: d["groups"].as_array().unwrap().iter().map(|g| (g[0].as_u64().unwrap() as u8, g[1].as_u64().unwrap() as u32)).collect(),
        })
        .collect();
    let menu = Menu { dishes, n_groups: [ng[0] as usize, ng[1] as usize, ng[2] as usize], ..Menu::default() };
    let g = &c["guest"];
    let cs = g["c"].as_array().unwrap();
    let guest = Guest { u: ints(&g["u"]).try_into().unwrap(), w: pairs(&g["w"]), c: [pairs(&cs[0]), pairs(&cs[1]), pairs(&cs[2])] };
    (menu, guest)
}

#[test]
fn the_integers_equal_the_python_reference_on_every_golden_case() {
    let m = shipped().expect("the shipped weights decode");
    let f: Value = serde_json::from_str(&fixture("snn/golden.json")).unwrap();
    let cases = f["cases"].as_array().unwrap();
    let (mut same, mut scores, mut first) = (0, 0, None);
    for (i, c) in cases.iter().enumerate() {
        let (menu, g) = golden_case(c);
        let st = infer::stalks(&m, &menu);
        let got = infer::scores(&m, &menu, &st, &g);
        scores += got.len();
        let want_x: Vec<Vec<i64>> = c["stalks"].as_array().unwrap().iter().map(ints).collect();
        if got == ints(&c["want"]) && st.x == want_x {
            same += 1;
        } else if first.is_none() {
            first = Some(format!("case {i}: got {:?} want {}", &got[..got.len().min(6)], c["want"]));
        }
    }
    println!("MEASURED snn golden: {same}/{} cases bit-identical to the python-int reference ({scores} scores and every dish stalk)", cases.len());
    assert_eq!(same, cases.len(), "{}", first.unwrap_or_default());
}

#[test]
fn the_inference_source_names_no_float() {
    for (name, src) in [("snn.rs", include_str!("../snn.rs")), ("infer.rs", include_str!("infer.rs")), ("blob.rs", include_str!("blob.rs")), ("shadow.rs", include_str!("shadow.rs")), ("quality.rs", include_str!("quality.rs"))] {
        let code: String = src.lines().map(|l| l.split("//").next().unwrap_or("")).collect::<Vec<_>>().join("\n");
        for word in ["f32", "f64", ".sqrt(", "powf", "powi", ".exp(", ".ln("] {
            assert!(!code.contains(word), "{name} names `{word}` outside a comment: the inference must stay integer");
        }
    }
}

#[test]
fn the_blob_round_trips_and_says_what_it_is() {
    let m = shipped().unwrap();
    assert_eq!(blob::encode(&m), WEIGHTS.to_vec());
    assert_eq!((m.id, m.d, m.layers, m.lam.len()), (20261006, 8, 2, AXES));
    assert_eq!(WEIGHTS.len(), blob::HEADER + 4 * blob::expected_params(8, 2) + 4);
    println!("MEASURED snn weights: {} bytes, {} params, beta {} / {Q}", WEIGHTS.len(), blob::expected_params(8, 2), m.beta);
}

#[test]
fn a_corrupted_blob_is_refused_by_name_and_the_guest_keeps_the_current_list() {
    let mut bad = WEIGHTS.to_vec();
    bad[blob::HEADER + 40] ^= 0x10;
    let err = blob::decode(&bad).unwrap_err();
    assert!(matches!(err, Refusal::BadCrc { .. }), "{err:?}");
    assert!(err.to_string().starts_with("snn_blob_crc"), "{err}");
    let current = vec![("a".to_string(), 900), ("b".to_string(), 700)];
    for mode in [Mode::Shadow, Mode::On] {
        let (shown, o) = decide(mode, current.clone(), 3, || blob::decode(&bad).map(|_| Vec::new()));
        assert_eq!(shown, current, "{mode:?}: the venue keeps serving the current ranker");
        assert!(matches!(&o, Outcome::Unusable(why) if why.starts_with("snn_blob_crc")), "{o:?}");
    }
    // The positive twin: the untouched blob is used.
    assert!(blob::decode(WEIGHTS).is_ok());
}

#[test]
fn every_other_broken_blob_is_refused_by_its_own_name() {
    assert!(matches!(blob::decode(&WEIGHTS[..20]), Err(Refusal::TooShort { len: 20 })));
    let reseal = |mut b: Vec<u8>| {
        let n = b.len() - 4;
        let crc = crate::block::crc32(&b[..n]);
        b[n..].copy_from_slice(&crc.to_le_bytes());
        b
    };
    let mut magic = WEIGHTS.to_vec();
    magic[0] = b'X';
    assert_eq!(blob::decode(&reseal(magic)), Err(Refusal::BadMagic));
    let mut fmt = WEIGHTS.to_vec();
    fmt[8] = 2;
    assert_eq!(blob::decode(&reseal(fmt)), Err(Refusal::BadFormat(2)));
    let mut dim = WEIGHTS.to_vec();
    dim[10] = 9;
    assert_eq!(blob::decode(&reseal(dim)), Err(Refusal::BadShape { what: "stalk dim", got: 9 }));
    let mut count = WEIGHTS.to_vec();
    count[24] ^= 1;
    assert!(matches!(blob::decode(&reseal(count)), Err(Refusal::BadCount { .. })));
    let mut short = WEIGHTS[..WEIGHTS.len() - 8].to_vec();
    short.extend_from_slice(&[0; 4]);
    assert!(matches!(blob::decode(&reseal(short)), Err(Refusal::BadLength { .. })));
}

/// The shared fixture's guests (W-TASTE `strip.json`): the phone's profile read as the network's
/// guest, the current list the phone's strip. 1000 guests x 4 menus.
fn strip_guests() -> Vec<(Vec<(String, String)>, Guest, Menu, Vec<(String, i64)>)> {
    let f: Value = serde_json::from_str(&fixture("rank/strip.json")).unwrap();
    let menus: Vec<Vec<Value>> = f["menus"].as_array().unwrap().iter().map(|m| m.as_array().unwrap().clone()).collect();
    let mut out = Vec::new();
    for c in f["cases"].as_array().unwrap() {
        let menu = &menus[c["menu"].as_u64().unwrap() as usize];
        let day = c["day"].as_i64().unwrap();
        let pairs: Vec<(String, String)> = menu.iter().filter_map(|p| Some((p.get("id")?.as_str()?.to_string(), p.to_string()))).collect();
        let net = Menu::from_products(&pairs, 0);
        let on: Vec<&Value> = menu.iter().filter(|p| p.get("id").is_some()).collect();
        let w = crate::rank::strip::weights(&c["profile"], &on, day);
        let sense = crate::rank::strip::sense_vec(&c["profile"], &on, day, None);
        let guest = Guest::from_maps(&net, &sense, &w.cat, &w.tag, &w.dish);
        let current: Vec<(String, i64)> = crate::rank::strip::strip(menu, &c["profile"], day, &Default::default()).into_iter().map(|x| (x.id, x.s)).collect();
        out.push((pairs, guest, net, current));
    }
    out
}

#[test]
fn shadow_never_changes_what_the_guest_sees_on_1000_fixture_guests() {
    let m = shipped().unwrap();
    let mut tally = Tally::default();
    let mut ran = 0;
    for (_, guest, net, current) in strip_guests() {
        let (shown, o) = decide(Mode::Shadow, current.clone(), 3, || {
            ran += 1;
            Ok(rank(&m, &net, &guest, 6, &[]))
        });
        assert_eq!(shown, current, "shadow showed something other than the current ranker");
        tally.add(&o, 20_000, m.id);
    }
    assert_eq!(ran, 1000, "the network ran beside every guest");
    println!(
        "MEASURED snn shadow on strip.json: compared {} refused {}, top-1 agreement {} pm, top-3 overlap {} pm",
        tally.compared, tally.unusable, tally.top1_pm(), tally.overlap_pm()
    );
    // The positive twin: `on` shows the network's list.
    let (_, guest, net, current) = strip_guests().into_iter().find(|x| !x.1.is_empty()).unwrap();
    let snn = rank(&m, &net, &guest, 6, &[]);
    assert!(!snn.is_empty());
    assert_eq!(decide(Mode::On, current, 3, || Ok(snn.clone())).0, snn);
}

#[test]
fn off_computes_nothing() {
    let current = vec![("a".to_string(), 1)];
    let (shown, o) = decide(Mode::Off, current.clone(), 3, || -> Result<Vec<(String, i64)>, Refusal> { panic!("off ran the network") });
    assert_eq!((shown, o), (current, Outcome::Off));
    let mut t = Tally::default();
    t.add(&Outcome::Off, 5, 1);
    assert_eq!(t, Tally { model: 1, ..Tally::default() }, "off is not even counted");
}

#[test]
fn the_switch_reads_shadow_unless_told_otherwise() {
    assert_eq!(Mode::parse(None), Mode::Shadow);
    assert_eq!(Mode::parse(Some("garbage")), Mode::Shadow);
    assert_eq!(Mode::parse(Some(" off ")), Mode::Off);
    assert_eq!(Mode::parse(Some("on")), Mode::On);
    assert!(Mode::ALL.iter().all(|s| Mode::parse(Some(s)).as_str() == *s));
}

#[test]
fn the_tally_holds_counts_only_and_restarts_for_a_new_model() {
    let mut t = Tally::default();
    t.add(&Outcome::Compared { top1_same: true, shared: 2, k: 3 }, 10, 7);
    t.add(&Outcome::Compared { top1_same: false, shared: 1, k: 3 }, 11, 7);
    t.add(&Outcome::Unusable("snn_blob_crc".into()), 12, 7);
    assert_eq!((t.compared, t.top1_pm(), t.overlap_pm(), t.unusable, t.since_day, t.last_day), (2, 500, 500, 1, 10, 12));
    let keys: Vec<String> = serde_json::to_value(&t).unwrap().as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["compared", "current_hit", "last_day", "model", "settled", "shared", "since_day", "slots", "snn_hit", "top1_same", "unusable"]);
    t.add(&Outcome::Compared { top1_same: true, shared: 3, k: 3 }, 13, 8);
    assert_eq!((t.compared, t.model, t.since_day), (1, 8, 13));
}

#[test]
fn the_network_ranks_the_set_the_current_ranker_ranks() {
    let menu = r#"[{"id":"a","sense":{"v":1,"taste":{"spicy":4},"texture":{},"aroma":{}},"categoryId":"rolls","tags":["Hot"],"ingredients":["Rice"," salmon"],"allergens":["gluten"]},
        {"id":"b","sense":{"v":1,"taste":{"spicy":3},"texture":{},"aroma":{}},"available":false},
        {"id":"c","sense":{"v":1,"taste":{"spicy":5},"texture":{},"aroma":{}},"allergens":["fish"]},
        {"id":"d","taste":{"spicy":2},"allergens":[]},{"id":"e","taste":{"spicy":2}}]"#;
    let pairs: Vec<(String, String)> = serde_json::from_str::<Vec<Value>>(menu).unwrap().iter().map(|p| (p["id"].as_str().unwrap().into(), p.to_string())).collect();
    let net = Menu::from_products(&pairs, crate::block::taste::avoid_mask(&["fish"]));
    let ids: Vec<&str> = net.dishes.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["a", "d"], "off sale, the avoided allergen and the undeclared dish are out");
    let spicy: BTreeMap<String, i64> = [("t:spicy".to_string(), 1000)].into_iter().collect();
    let mut cur: Vec<String> = crate::block::taste::top_k_json(&pairs, &spicy, crate::block::taste::avoid_mask(&["fish"]), 9).into_iter().map(|x| x.0).collect();
    cur.sort();
    assert_eq!(cur, ids, "the same set top_k_json ranks");
    assert_eq!(net.dishes[0].v[5], 800, "t:spicy 4/5");
    assert_eq!(net.names[INGREDIENT as usize].keys().collect::<Vec<_>>(), ["rice", "salmon"]);
    let sense: BTreeMap<String, i64> = [("t:spicy".to_string(), 900)].into_iter().collect();
    let g = Guest::from_maps(&net, &sense, &BTreeMap::new(), &[("hot".to_string(), 1000)].into_iter().collect(), &BTreeMap::new());
    assert_eq!(g.c[TAG as usize], vec![(0, 1000)]);
    let got = rank(&shipped().unwrap(), &net, &g, 5, &[]);
    assert!(got.iter().all(|(id, s)| (id == "a" || id == "d") && *s > 0), "{got:?}");
    assert!(rank(&shipped().unwrap(), &net, &Guest::default(), 5, &[]).is_empty(), "an empty profile ranks nothing");
}

#[test]
fn one_guest_over_165_dishes_is_under_a_millisecond_in_release() {
    let m = shipped().unwrap();
    let f: Value = serde_json::from_str(&fixture("rank/strip.json")).unwrap();
    let menu: Vec<Value> = f["menus"][3].as_array().unwrap().clone();
    assert_eq!(menu.len(), 165);
    let pairs: Vec<(String, String)> = menu.iter().map(|p| (p["id"].as_str().unwrap().to_string(), p.to_string())).collect();
    let sense: BTreeMap<String, i64> = [("t:umami", 900), ("x:crispy", 400), ("a:smoky", 700)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
    let (mut rank_ns, mut full_ns) = (Vec::new(), Vec::new());
    for _ in 0..41 {
        let t = std::time::Instant::now();
        let net = Menu::from_products(&pairs, 0);
        let built = t.elapsed().as_nanos();
        let g = Guest::from_maps(&net, &sense, &BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new());
        let t2 = std::time::Instant::now();
        let top = rank(&m, &net, &g, 6, &[]);
        rank_ns.push(t2.elapsed().as_nanos());
        full_ns.push(built + t2.elapsed().as_nanos());
        assert_eq!(top.len(), 6);
    }
    rank_ns.sort_unstable();
    full_ns.sort_unstable();
    let (r, f) = (rank_ns[20] / 1000, full_ns[20] / 1000);
    println!("MEASURED snn one guest x 165 dishes: rank {r} us median, with the menu read from JSON {f} us (debug_assertions={})", cfg!(debug_assertions));
    // The heavier shape: 165 dishes in 16 categories with tags AND ingredients (the A/B's venue menu).
    let mut lcg = super::synth::Lcg(7);
    let shape = super::synth::shape(&mut lcg);
    let rows = super::synth::menu(&mut lcg, 165, Some(&shape));
    let vpairs: Vec<(String, String)> = rows.iter().enumerate().map(|(i, d)| (format!("d{i:03}"), super::synth::product(&format!("d{i:03}"), d).to_string())).collect();
    let vnet = Menu::from_products(&vpairs, 0);
    let vg = Guest::from_maps(&vnet, &sense, &BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new());
    let mut vns: Vec<u128> = (0..41).map(|_| { let t = std::time::Instant::now(); let _ = rank(&m, &vnet, &vg, 6, &[]); t.elapsed().as_nanos() }).collect();
    vns.sort_unstable();
    let v = vns[20] / 1000;
    let groups: usize = vnet.n_groups.iter().sum();
    println!("MEASURED snn one guest x 165 venue-shaped dishes ({groups} groups): rank {v} us median");
    if !cfg!(debug_assertions) {
        assert!(r < 1000 && v < 1000, "rank took {r} / {v} us; the budget is 1 ms");
    }
}

#[test]
fn a_hold_is_checked_once_against_the_next_order_and_counted_per_venue_only() {
    use super::quality::{settle, Held};
    let cur = vec![("a".to_string(), 9), ("b".to_string(), 8), ("c".to_string(), 7), ("d".to_string(), 6)];
    let net = vec![("c".to_string(), 9), ("x".to_string(), 8)];
    let h = Held::of(&cur, &net, 3, 7, 100);
    assert_eq!((h.current.len(), h.snn.len()), (3, 2), "the first k of each, no more");
    let mut slot = Some(h);
    assert_eq!(settle(&mut slot, &["x".to_string()]), Some((false, true, 7)));
    assert_eq!(slot, None, "cleared once checked");
    assert_eq!(settle(&mut slot, &["x".to_string()]), None, "a second order checks nothing");
    let mut empty = Some(Held::of(&cur, &net, 3, 7, 100));
    assert_eq!(settle(&mut empty, &[]), None);
    assert_eq!(empty, None, "an order with no dish still clears the hold");
    let mut t = Tally::default();
    t.add(&Outcome::Compared { top1_same: false, shared: 1, k: 3 }, 100, 7);
    t.settle(false, true, 101, 7);
    t.settle(true, true, 102, 7);
    t.settle(true, false, 103, 6);
    assert_eq!((t.settled, t.hit_pm()), (2, (500, 1000)), "a check about another model is not counted");
    let old: Tally = serde_json::from_str(r#"{"compared":3,"top1_same":1,"shared":2,"slots":9,"unusable":0,"since_day":1,"last_day":2,"model":7}"#).unwrap();
    assert_eq!((old.compared, old.settled), (3, 0), "a tally written before quality reads with zero checks");
}
