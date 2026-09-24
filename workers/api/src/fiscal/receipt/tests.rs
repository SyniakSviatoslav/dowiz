//! TAX §3.8 / art. 29: "pa NIVF" before registration, the codes after.

use super::*;
use crate::fiscal::document::document;
use crate::services::ordering::tax_block::stamp;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::RatePpm;
use serde_json::json;

const NOW: i64 = 1_790_000_000_000; // 2026-09-21T14:13:20Z
const V20: VenueTax = VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

fn doc() -> Document {
    let mut o = json!({
        "id": "o1", "location_id": "v1", "status": "DELIVERED",
        "items": [
            { "name": "Kafe", "quantity": 3, "unit_price": 250 },
            { "name": "Uje", "quantity": 1, "unit_price": 100, "vat_ppm": 60000 },
        ],
        "total": 1150, "fulfilment": { "kind": "dine_in", "table": "7" }, "channel": "room",
        "payments": [{ "method": "cash", "amount": 1150, "currency": "ALL" }],
    });
    stamp(&mut o, &V20, 300, 0, 0).unwrap();
    document(&o, "ALL", NOW).unwrap()
}

fn codes() -> Codes {
    Codes { iic: "IIC0001".into(), fic: "FIC-9f2c".into(), inv_ord_num: "42".into() }
}

#[test]
fn before_registration_it_says_pa_nivf_and_carries_every_other_field() {
    let r = receipt_text(&doc(), None);
    assert!(r.contains("pa NIVF"), "{r}");
    assert!(!r.contains("FIC-9f2c") && !r.contains("NIVF (FIC)"), "no codes yet:\n{r}");
    for want in [
        "Porosia: o1",
        "Tavolina: 7",
        "Data: 20260921T141320Z",
        "3 x Kafe @ 250 ALL = 750 ALL (TVSH 20 %)",
        "1 x Uje @ 100 ALL = 100 ALL (TVSH 6 %)",
        "TVSH 6 %: base 100 ALL, TVSH 6 ALL",
        "TVSH 20 %: base 750 ALL, TVSH 125 ALL",
        "Dergesa, TVSH 20 %: base 300 ALL, TVSH 50 ALL",
        "TOTALI: 1150 ALL",
        "Pagesa: kesh",
        "cash 1150 ALL",
        "Cmimet perfshijne TVSH",
    ] {
        assert!(r.contains(want), "missing {want:?} in\n{r}");
    }
    assert!(r.contains(&format!("Nr. {}", uuid_text(&doc().uuid))));
}

#[test]
fn after_registration_it_carries_the_codes_and_no_pa_nivf() {
    let r = receipt_text(&doc(), Some(&codes()));
    assert!(!r.contains("pa NIVF"), "{r}");
    assert!(r.contains("NIVF (FIC): FIC-9f2c"));
    assert!(r.contains("NSLF (IIC): IIC0001"));
    assert!(r.contains("Nr. i fatures: 42"));
    assert!(r.contains("invoice-check/#/verify?iic=IIC0001"));
    assert!(r.contains("TOTALI: 1150 ALL"), "the body is the same document");
}

#[test]
fn lek_is_never_drawn_as_hundredths_and_euros_are() {
    let r = receipt_text(&doc(), None);
    assert!(!r.contains("11.50"), "1150 lek is not 11.50:\n{r}");
    let mut d = doc();
    d.currency = "EUR".into();
    d.total = 981;
    assert!(receipt_text(&d, None).contains("TOTALI: 9.81 EUR"));
}

#[test]
fn a_rate_prints_from_its_ppm_without_a_float() {
    assert_eq!(percent(200_000), "20");
    assert_eq!(percent(88_750), "8.875");
    assert_eq!(percent(60_000), "6");
    assert_eq!(percent(0), "0");
}

#[test]
fn a_corrective_says_what_it_corrects() {
    let d = doc();
    let c = crate::fiscal::document::corrective(&d, NOW + 1);
    let r = receipt_text(&c, None);
    assert!(r.starts_with("FATURE KORRIGJUESE\n"));
    assert!(r.contains(&format!("Korrigjon: {}", uuid_text(&d.uuid))));
    assert!(r.contains("TOTALI: -1150 ALL"));
}
