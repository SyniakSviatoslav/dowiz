//! RED/GREEN for the integer rate.
//!
//! The worked examples are ported VERBATIM from
//! `docs/design/BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md` §3.2-§3.4, in lek,
//! including the tie case. Lek has no minor unit (`money_text(1500, "ALL")` is
//! `"1500 ALL"`), so every rounding here is to a whole lek and the ties are
//! common rather than academic.

use super::*;

const VAT20: RatePpm = RatePpm(200_000);
const VAT6: RatePpm = RatePpm(60_000);

fn basket(lines: &[TaxLine], discount: i64, inclusive: bool) -> TaxInput<'_> {
    TaxInput {
        lines,
        discount,
        inclusive,
        fee: None,
        tip: 0,
    }
}

// ── The rate is an integer, and a float on the wire is refused ──────────────

#[test]
fn green_a_rate_is_an_integer_in_parts_per_million() {
    assert_eq!(RatePpm::parse("200000").unwrap(), VAT20);
    assert_eq!(RatePpm::parse("60000").unwrap(), VAT6);
    assert_eq!(RatePpm::parse("88750").unwrap(), RatePpm(88_750)); // NYC 8.875%
    assert_eq!(RatePpm::parse("0").unwrap(), RatePpm(0));
    assert_eq!(RatePpm::parse("1000000").unwrap(), RatePpm(RatePpm::MAX));
}

#[test]
fn red_a_float_on_the_wire_is_refused() {
    // `"tax_rate": 0.20` is the shape this refuses. Every spelling of it.
    for s in ["0.20", "0.2", ".2", "20.0", "0,20", "200000.0"] {
        let e = RatePpm::parse(s).expect_err("a float must be refused");
        assert!(
            e.contains("not an integer"),
            "{s}: refusal must name the reason, got {e}"
        );
    }
    // And the integer the same rate is spelled as IS accepted.
    assert_eq!(RatePpm::parse("200000").unwrap().0, 200_000);
}

#[test]
fn red_a_sign_exponent_separator_or_space_is_refused() {
    assert!(RatePpm::parse("-1").unwrap_err().contains("negative"));
    assert!(RatePpm::parse("-200000").unwrap_err().contains("negative"));
    for s in ["+200000", "2e5", "2E5", "200_000", " 200000", "200000 ", "abc"] {
        assert!(
            RatePpm::parse(s).is_err(),
            "{s} must be refused, not silently coerced"
        );
    }
    assert!(RatePpm::parse("").unwrap_err().contains("empty"));
}

#[test]
fn red_a_rate_above_one_hundred_percent_is_refused() {
    assert!(RatePpm::parse("1000001").unwrap_err().contains("above 100%"));
    // The typo this exists for: 20 % typed as `20000000` ppm (2000 %).
    assert!(RatePpm::parse("20000000").unwrap_err().contains("above 100%"));
    assert!(RatePpm::parse("4294967296") // one past u32
        .unwrap_err()
        .contains("too large"));
}

// ── tax_of IS the generated organ (the authority flip) ──────────────────────

#[test]
fn green_tax_of_is_the_generated_organ_not_a_second_law() {
    assert_eq!(tax_of(1000, VAT20, false).unwrap(), 200);
    assert_eq!(tax_of(1200, VAT20, true).unwrap(), 200);
    // Bit-for-bit the organ, over a grid: any divergence means a second law
    // was written by accident.
    for base in [0i64, 1, 3, 9, 250, 675, 750, 999, 1_000_000, i64::MAX / 4] {
        for ppm in [0u32, 1, 60_000, 88_750, 200_000, 999_999, 1_000_000] {
            for incl in [false, true] {
                let want = if incl {
                    crate::eqc_gen::apply_tax_inclusive_int(base, i64::from(ppm))
                } else {
                    crate::eqc_gen::apply_tax_exclusive_int(base, i64::from(ppm))
                };
                let got = tax_of(base, RatePpm(ppm), incl);
                match want {
                    Ok(v) => assert_eq!(got.unwrap(), v, "base={base} ppm={ppm} incl={incl}"),
                    Err(_) => assert!(got.is_err(), "base={base} ppm={ppm} incl={incl}"),
                }
            }
        }
    }
}

// ── Blueprint §3.2 example 1: per line vs per group ─────────────────────────

#[test]
fn green_example_1_three_coffees_are_125_lek_of_vat_not_126() {
    // Three coffees at 250 lek, 20 % inclusive.
    let lines = [
        TaxLine { amount: 250, rate: VAT20 },
        TaxLine { amount: 250, rate: VAT20 },
        TaxLine { amount: 250, rate: VAT20 },
    ];
    let s = summarise(&basket(&lines, 0, true)).unwrap();
    assert_eq!(s.groups.len(), 1);
    assert_eq!(s.groups[0].base, 750, "B = 750");
    assert_eq!(s.groups[0].tax, 125, "net = 625, tax = 125");
    assert_eq!(s.groups[0].lines, 3);
    assert_eq!(s.tax_total, 125);
    assert_eq!(s.order_total, 750, "inclusive: the gross is the total");

    // The naive per-line answer, computed here so the difference is a MEASURED
    // 1 lek and not a claim: net(250) = 208 (208.33 rounds down), tax = 42,
    // three lines = 126. A fiscal message has no field for that 1 lek.
    let per_line: i64 = lines.iter().map(|l| tax_of(l.amount, l.rate, true).unwrap()).sum();
    assert_eq!(per_line, 126, "per-line rounding drifts");
    assert_ne!(per_line, s.tax_total, "and the group is the authority");
}

// ── Blueprint §3.2 example 2: which quantity you round decides a tie ────────

#[test]
fn green_example_2_the_tie_rounds_the_net_so_the_tax_is_112_not_113() {
    // The same basket with a 10 % promo: `Promo::discount` floors 750*10/100 = 75.
    let lines = [
        TaxLine { amount: 250, rate: VAT20 },
        TaxLine { amount: 250, rate: VAT20 },
        TaxLine { amount: 250, rate: VAT20 },
    ];
    let s = summarise(&basket(&lines, 75, true)).unwrap();
    assert_eq!(s.groups[0].base, 675, "B = 750 - 75");
    assert_eq!(s.discount_allocated, alloc::vec![75]);
    assert_eq!(s.groups[0].tax, 112, "exact tax is 112.5 — a TIE");
    assert_eq!(s.order_total, 675);

    // The two rules that also sound like "round half up" and give 113. The
    // equation names the rounded quantity so nobody "fixes" this. NOTE the
    // denominator: on an INCLUSIVE gross the tax fraction is r/(10^6 + r), not
    // r/10^6 — getting that wrong is its own way to be off by 22 lek.
    let round_the_tax = (675i128 * 200_000 + 1_200_000 / 2) / 1_200_000; // 112.5 -> 113
    assert_eq!(round_the_tax, 113);
    let truncate_the_net = 675i128 - (675i128 * 1_000_000) / 1_200_000; // 562 -> 113
    assert_eq!(truncate_the_net, 113);
    assert_ne!(i128::from(s.groups[0].tax), round_the_tax);
}

#[test]
fn green_at_twenty_percent_every_whole_lek_gross_that_is_three_mod_six_is_a_tie() {
    // The blueprint's claim, checked rather than repeated.
    for g in [3i64, 9, 15, 21, 675, 999] {
        assert_eq!(g % 6, 3);
        let exact_tax_times_two = g * 2 * 200_000 / 1_200_000; // 2 * G/6
        assert_eq!(exact_tax_times_two % 2, 1, "G={g} is a tie (x.5)");
        let net_half_up = tax_of(g, VAT20, true).unwrap();
        let tax_half_up = (g * 200_000 + 1_200_000 / 2) / 1_200_000;
        assert_eq!(net_half_up, tax_half_up - 1, "G={g}: the two rules differ by 1");
    }
}

// ── Blueprint §3.4: the tax block, reproduced field for field ───────────────

#[test]
fn green_the_tax_block_of_section_3_4() {
    // "tax": { inclusive true, groups [{200000, base 675, tax 112, lines 1}],
    //          fee {200000, base 300, tax 50}, total 162, discount_allocated [75] }
    let lines = [TaxLine { amount: 750, rate: VAT20 }];
    let s = summarise(&TaxInput {
        lines: &lines,
        discount: 75,
        inclusive: true,
        fee: Some(TaxLine { amount: 300, rate: VAT20 }),
        tip: 0,
    })
    .unwrap();
    assert!(s.inclusive);
    assert_eq!(
        s.groups,
        alloc::vec![TaxGroup { rate_ppm: 200_000, base: 675, tax: 112, lines: 1 }]
    );
    assert_eq!(
        s.fee,
        Some(TaxGroup { rate_ppm: 200_000, base: 300, tax: 50, lines: 1 })
    );
    assert_eq!(s.tax_total, 162, "tax.total = 112 + 50");
    assert_eq!(s.discount_allocated, alloc::vec![75]);
    assert_eq!(s.order_total, 975, "675 + 300 + 0 tip");
}

// ── Allocation, grouping, exclusive, and the refusals ───────────────────────

#[test]
fn green_largest_remainder_allocates_the_discount_exactly() {
    // 100 lek at 6 %, 200 lek at 20 %, a 7-lek cut that divides neither.
    let lines = [
        TaxLine { amount: 200, rate: VAT20 },
        TaxLine { amount: 100, rate: VAT6 },
    ];
    let s = summarise(&basket(&lines, 7, true)).unwrap();
    // Groups come back sorted by rate ASCENDING — a canonical order.
    assert_eq!(s.groups[0].rate_ppm, 60_000);
    assert_eq!(s.groups[1].rate_ppm, 200_000);
    // floor: 2 and 4; the 1 left over goes to the larger remainder (the 20 % group).
    assert_eq!(s.discount_allocated, alloc::vec![2, 5]);
    assert_eq!(
        s.discount_allocated.iter().sum::<i64>(),
        7,
        "Sigma d_r == D EXACTLY, which is the whole point of largest remainder"
    );
    assert_eq!(s.groups[0].base, 98);
    assert_eq!(s.groups[1].base, 195);
    assert_eq!(s.groups[0].tax, 6);
    assert_eq!(s.groups[1].tax, 32);
    assert_eq!(s.tax_total, 38);
    assert_eq!(s.order_total, 293, "300 - 7");
}

#[test]
fn green_a_remainder_tie_goes_to_the_lower_rate_deterministically() {
    let lines = [
        TaxLine { amount: 100, rate: VAT20 },
        TaxLine { amount: 100, rate: VAT6 },
    ];
    let s = summarise(&basket(&lines, 1, true)).unwrap();
    assert_eq!(s.discount_allocated, alloc::vec![1, 0], "6 % group first");
    // Replay: the same input gives the same bytes, every time, on every node.
    for _ in 0..8 {
        assert_eq!(summarise(&basket(&lines, 1, true)).unwrap(), s);
    }
}

#[test]
fn green_exclusive_prices_add_the_tax_to_the_total() {
    let lines = [TaxLine { amount: 1000, rate: VAT20 }];
    let s = summarise(&TaxInput {
        lines: &lines,
        discount: 0,
        inclusive: false,
        fee: Some(TaxLine { amount: 300, rate: VAT20 }),
        tip: 50,
    })
    .unwrap();
    assert_eq!(s.groups[0].tax, 200);
    assert_eq!(s.fee.unwrap().tax, 60);
    assert_eq!(s.tax_total, 260);
    assert_eq!(s.order_total, 1000 + 260 + 300 + 50);
}

#[test]
fn green_the_tip_is_outside_the_taxable_base() {
    let lines = [TaxLine { amount: 1000, rate: VAT20 }];
    let with_tip = summarise(&TaxInput {
        lines: &lines,
        discount: 0,
        inclusive: true,
        fee: None,
        tip: 500,
    })
    .unwrap();
    let without = summarise(&basket(&lines, 0, true)).unwrap();
    assert_eq!(with_tip.tax_total, without.tax_total, "a gratuity is not taxed");
    assert_eq!(with_tip.order_total, without.order_total + 500);
}

#[test]
fn green_an_empty_basket_is_a_zero_summary_not_a_refusal() {
    let s = summarise(&basket(&[], 0, true)).unwrap();
    assert!(s.groups.is_empty());
    assert_eq!(s.tax_total, 0);
    assert_eq!(s.order_total, 0);
}

#[test]
fn red_a_discount_larger_than_the_basket_is_refused_with_both_numbers() {
    let lines = [TaxLine { amount: 100, rate: VAT20 }];
    let e = summarise(&basket(&lines, 101, true)).unwrap_err();
    assert!(e.contains("101") && e.contains("100"), "got: {e}");
    // And a discount on an empty basket is the same refusal, not a silent zero.
    assert!(summarise(&basket(&[], 1, true)).is_err());
}

#[test]
fn red_negative_money_is_refused_and_says_which() {
    let neg = [TaxLine { amount: -1, rate: VAT20 }];
    assert!(summarise(&basket(&neg, 0, true))
        .unwrap_err()
        .contains("negative line amount"));
    let ok = [TaxLine { amount: 100, rate: VAT20 }];
    assert!(summarise(&basket(&ok, -1, true))
        .unwrap_err()
        .contains("negative discount"));
    assert!(summarise(&TaxInput {
        lines: &ok,
        discount: 0,
        inclusive: true,
        fee: None,
        tip: -1
    })
    .unwrap_err()
    .contains("negative tip"));
    assert!(summarise(&TaxInput {
        lines: &ok,
        discount: 0,
        inclusive: true,
        fee: Some(TaxLine { amount: -1, rate: VAT20 }),
        tip: 0
    })
    .unwrap_err()
    .contains("negative fee"));
}

#[test]
fn red_overflow_is_refused_never_wrapped() {
    let huge = [
        TaxLine { amount: i64::MAX, rate: VAT20 },
        TaxLine { amount: i64::MAX, rate: VAT20 },
    ];
    assert!(summarise(&basket(&huge, 0, true))
        .unwrap_err()
        .contains("overflow"));
    // Exclusive at i64::MAX: the tax is in range but base + tax is not.
    let one = [TaxLine { amount: i64::MAX, rate: RatePpm(1_000_000) }];
    assert!(summarise(&basket(&one, 0, false))
        .unwrap_err()
        .contains("overflow"));
}

// ── Item 2: the rate in force at `now_ms` (no clock; `now_ms` is passed in) ──

const LAW_DAY_MS: i64 = 1_767_225_600_000; // 2026-01-01T00:00:00Z

#[test]
fn green_in_force_is_the_default_one_ms_before_and_the_scheduled_rate_after() {
    let sched = schedule::parse(r#"[{"since_ms":1767225600000,"ppm":70000}]"#).unwrap();
    assert_eq!(in_force(&sched, Some(VAT20), LAW_DAY_MS - 1), Some(VAT20));
    assert_eq!(in_force(&sched, Some(VAT20), LAW_DAY_MS), Some(RatePpm(70_000)));
    assert_eq!(in_force(&sched, Some(VAT20), LAW_DAY_MS + 1), Some(RatePpm(70_000)));
}

#[test]
fn green_in_force_takes_the_latest_change_that_has_happened() {
    // Out of order on purpose: the owner's list is not trusted to be sorted.
    let sched = schedule::parse(
        r#"[{"since_ms":300,"ppm":60000},{"since_ms":100,"ppm":100000},{"since_ms":200,"ppm":0}]"#,
    )
    .unwrap();
    assert_eq!(in_force(&sched, None, 99), None, "no default and nothing in force yet");
    assert_eq!(in_force(&sched, None, 150), Some(RatePpm(100_000)));
    assert_eq!(in_force(&sched, None, 250), Some(RatePpm(0)), "a zero rate is a rate");
    assert_eq!(in_force(&sched, None, 301), Some(VAT6));
}

#[test]
fn green_an_empty_schedule_is_the_default() {
    assert_eq!(schedule::parse("").unwrap(), alloc::vec![]);
    assert_eq!(schedule::parse("[]").unwrap(), alloc::vec![]);
    assert_eq!(in_force(&[], Some(VAT6), 0), Some(VAT6));
    assert_eq!(in_force(&[], None, 0), None);
}

#[test]
fn red_a_schedule_with_a_float_or_a_bad_shape_is_refused_with_a_reason() {
    for bad in [
        r#"[{"since_ms":1,"ppm":0.2}]"#,
        r#"[{"since_ms":1,"ppm":"200000"}]"#,
        r#"[{"since_ms":1.5,"ppm":200000}]"#,
        r#"[{"since_ms":1}]"#,
        r#"[{"ppm":200000}]"#,
        r#"[{"since_ms":1,"ppm":2000000}]"#,
        r#"[{"since_ms":1,"ppm":-1}]"#,
        r#"[{"since_ms":1,"ppm":1,"extra":2}]"#,
        r#"{"since_ms":1,"ppm":1}"#,
        "[1]",
        "not json",
    ] {
        let e = schedule::parse(bad).expect_err(bad);
        assert!(e.starts_with("tax.schedule"), "{bad}: {e}");
    }
    // At most a handful: the future only (blueprint §2.6), never a history.
    let many: alloc::vec::Vec<alloc::string::String> =
        (0..9).map(|i| alloc::format!(r#"{{"since_ms":{i},"ppm":1}}"#)).collect();
    let s = alloc::format!("[{}]", many.join(","));
    assert!(schedule::parse(&s).is_err(), "nine entries is a history, not a schedule");
}
