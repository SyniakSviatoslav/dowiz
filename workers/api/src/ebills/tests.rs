//! The wire shapes here are the measured ones from the blueprint (§1.5,
//! §1.6), with every identifier synthetic: the field NAMES and TYPES are the
//! platform's, the values are not anyone's.

use super::map::*;
use super::time::epoch_ms;
use super::*;
use serde_json::{json, Value};

const UUID_COURSE: &str = "00000000-0000-4000-8000-000000008602";
const UUID_BILL: &str = "00000000-0000-4000-8000-000000008601";
const IIC: &str = "0123456789ABCDEF0123456789ABCDEF";
const FIC: &str = "11111111-2222-4333-8444-555555555555";

/// Raw JSON, not `json!`: the macro recurses once per token and a 40-key
/// sale overran the crate's recursion limit, which is `lib.rs`'s to raise.
fn raw(text: &str) -> Value {
    serde_json::from_str(text).expect("fixture is valid JSON")
}

/// A course rung up at table 12: `summaryInvoice:false` + `saleUnitOrder`,
/// one line of 2 × 250 lek. Exactly the record of §1.5.
fn course() -> Value {
    raw(&format!(
        r#"{{
        "id": 8602, "invOrdNum": 8598, "uuid": "{UUID_COURSE}", "fic": "{FIC}",
        "timestamp": "2026-09-21T09:01:25.857Z",
        "status": "CLOSED", "fiscalSatus": "FINISHED", "draft": 0, "changedStatus": null,
        "summaryInvoice": false, "exchange": false, "buying": false, "selfIssue": "SELFISSUE_0",
        "paymentMethod": "CASH", "salePaymentMethods": [], "totalValue": 500.0,
        "totalVatAmount": 83.3333333333, "currencyRate": 1.0,
        "currency": {{"id": 1, "currency": "Albania Lek", "currencyCode": "ALL", "sellRate": 1.0}},
        "delivery": null, "notes": null,
        "client": {{"id": 1, "name": "Default Client", "defaultClient": true}},
        "extraUser": {{"id": 1, "operatorCode": "xx000xx000"}},
        "pointOfSale": {{"id": 1, "posName": "pos1"}},
        "saleUnit": {{"id": 12, "identifier": "12", "type": "TABLE", "status": "ACTIVE", "posX": 5.0, "posY": 3.0}},
        "saleUnitOrder": {{"id": 5614, "status": "COMPLETED"}},
        "saleRecords": [{{
            "id": 18490, "saleNo": 1, "itemName": "korca", "amount": 2.0, "price": 250.0,
            "totalValue": 500.0, "vat": "VAT_20", "totalVatAmount": 83.3333333333,
            "discount": 0.0, "discountReduceBase": false, "refundType": null,
            "itemInSale": {{"id": 74, "itemCode": "56", "item": "korca", "price": 250.0, "vat": "VAT_20",
                           "isService": true, "unit": {{"id": 1839, "unit": "Copë", "unitCode": "XPP"}}}}
        }}],
        "logCis": [{{"id": 8602, "iic": "{IIC}", "iicReference": "{IIC}", "fic": "{FIC}", "status": "SUCCESS",
                    "sentTime": "2026-09-21T09:01:25Z", "faultString": null}}],
        "qrCode": "https://efiskalizimi-app.tatime.gov.al/invoice-check/#/verify?iic=...&tin=..."
        }}"#
    ))
}

/// The bill that closed table 14: `summaryInvoice:true`, no `saleUnitOrder`,
/// two lines of 80 lek. As the LIST returns it: `saleRecords: null`.
fn bill_listed() -> Value {
    raw(&format!(
        r#"{{
        "id": 8601, "invOrdNum": 8597, "uuid": "{UUID_BILL}", "fic": "{FIC}",
        "timestamp": "2026-09-21T09:01:19.404634Z",
        "status": "CLOSED", "fiscalSatus": "FINISHED", "draft": 0,
        "summaryInvoice": true, "paymentMethod": "CASH", "totalValue": 160.0,
        "totalVatAmount": 26.6666666666, "currencyRate": 1.0,
        "currency": {{"id": 1, "currencyCode": "ALL"}},
        "saleRecords": null, "salePays": null, "delivery": null,
        "client": {{"id": 1}}, "extraUser": {{"id": 1}}, "pointOfSale": {{"id": 1}},
        "saleUnit": {{"id": 14, "identifier": "14", "type": "TABLE", "status": "ACTIVE"}},
        "saleUnitOrder": null,
        "logCis": [{{"id": 8601, "iic": "{IIC}", "fic": "{FIC}", "status": "SUCCESS"}}]
        }}"#
    ))
}

fn wrapped(sale: Value) -> String {
    json!({ "sale": sale, "saleModified": [], "reason": null, "saleType": null,
            "withoutfiscalization": null, "includeClosingTheTable": false, "id": null })
    .to_string()
}

fn course_sale() -> Sale {
    parse_detail(&wrapped(course())).unwrap()
}

fn course_with(edit: impl FnOnce(&mut Value)) -> Sale {
    let mut v = course();
    edit(&mut v);
    parse_detail(&wrapped(v)).unwrap()
}

#[test]
fn the_list_parses_and_its_rows_carry_no_lines() {
    let body = json!({ "sales": [bill_listed(), course()], "total": 660.0 }).to_string();
    let list = parse_list(&body).unwrap();
    assert_eq!(list.sales.len(), 2);
    assert_eq!(list.total, 660.0);
    assert!(
        list.sales[0].sale_records.is_none(),
        "the list says null, and null it stays"
    );
    assert_eq!(list.sales[1].sale_records.as_ref().map(Vec::len), Some(1));
}

#[test]
fn the_detail_wrapper_yields_the_inner_sale() {
    let s = course_sale();
    assert_eq!((s.id, s.inv_ord_num), (8602, 8598));
    assert_eq!(
        s.fiscal_status, "FINISHED",
        "the platform spells it fiscalSatus; we do not"
    );
    assert_eq!(
        s.sale_unit_order.as_ref().and_then(|o| o.status.as_deref()),
        Some("COMPLETED")
    );
    assert_eq!(s.log_cis.as_ref().map(Vec::len), Some(1));
}

#[test]
fn a_missing_declared_key_is_a_parse_error_not_a_default() {
    let mut v = course();
    v.as_object_mut().unwrap().remove("uuid");
    let err = parse_detail(&wrapped(v)).unwrap_err();
    assert!(err.contains("uuid"), "{err}");
}

#[test]
fn the_four_flag_combinations() {
    assert_eq!(classify(&course_sale()), Ok(Kind::Course));
    let bill = parse_list(&json!({"sales":[bill_listed()],"total":0.0}).to_string()).unwrap();
    assert_eq!(classify(&bill.sales[0]), Ok(Kind::Bill));
    let counter = course_with(|v| v["saleUnitOrder"] = Value::Null);
    assert_eq!(classify(&counter), Ok(Kind::CounterSale));
    let odd = course_with(|v| v["summaryInvoice"] = json!(true));
    assert_eq!(classify(&odd), Err(MapError::BillWithOrder));
}

#[test]
fn whole_lek_or_nothing() {
    assert_eq!(whole(160.0, "x"), Ok(160));
    assert_eq!(whole(0.0, "x"), Ok(0));
    assert_eq!(whole(-0.0, "x"), Ok(0));
    assert_eq!(lek(4600.0, "x"), Ok(4600));
    let vat = whole(26.6666666666, "totalVatAmount").unwrap_err();
    assert!(
        matches!(
            vat,
            MapError::NotWhole {
                field: "totalVatAmount",
                ..
            }
        ),
        "{vat:?}"
    );
    assert!(matches!(
        whole(0.35, "amount"),
        Err(MapError::NotWhole {
            field: "amount",
            ..
        })
    ));
    assert!(matches!(
        whole(f64::NAN, "x"),
        Err(MapError::NotWhole { .. })
    ));
    assert!(matches!(
        whole(f64::INFINITY, "x"),
        Err(MapError::NotWhole { .. })
    ));
    assert!(
        matches!(
            whole(9_007_199_254_740_992.0, "x"),
            Err(MapError::NotWhole { .. })
        ),
        "2^53 is where doubles stop being exact"
    );
    assert_eq!(
        whole(9_007_199_254_740_991.0, "x"),
        Ok(9_007_199_254_740_991)
    );
    assert_eq!(
        lek(-5.0, "price"),
        Err(MapError::Negative { field: "price" })
    );
}

#[test]
fn timestamps_to_milliseconds_without_a_clock() {
    // Vectors computed with Python's datetime (UTC) on the box.
    assert_eq!(epoch_ms("2026-09-21T09:01:25.857Z"), Ok(1_789_981_285_857));
    assert_eq!(
        epoch_ms("2026-09-21T09:01:19.404634Z"),
        Ok(1_789_981_279_404),
        "truncated, not rounded"
    );
    assert_eq!(epoch_ms("2026-09-21T09:01:19Z"), Ok(1_789_981_279_000));
    assert_eq!(epoch_ms("2000-03-01T00:00:00Z"), Ok(951_868_800_000));
    assert_eq!(epoch_ms("1970-01-01T00:00:00Z"), Ok(0));
    assert_eq!(
        epoch_ms("2024-02-29T23:59:59.9Z"),
        Ok(1_709_251_199_900),
        "leap day, one-digit fraction"
    );
    for bad in [
        "2026-09-21T09:01:25.857",
        "2026-13-01T00:00:00Z",
        "2026-09-21 09:01:25Z",
        "2026-09-21T09:01Z",
        "2026-09-21T09:01:25.abcZ",
        "",
    ] {
        assert_eq!(
            epoch_ms(bad),
            Err(MapError::Timestamp(bad.to_string())),
            "{bad}"
        );
    }
}

#[test]
fn payment_words() {
    assert_eq!(payment("CASH"), Ok("cash"));
    assert_eq!(payment("CARD"), Ok("card"));
    assert_eq!(payment("CARD_ON_POS"), Ok("card"));
    assert_eq!(payment("POK_CARD"), Ok("card"));
    for other in [
        "PAYSERA", "BANK", "VOUCHER", "WAIVER", "MULTIPLE", "cash", "",
    ] {
        assert_eq!(payment(other), Err(MapError::Payment(other.to_string())));
    }
}

#[test]
fn vat_rate_is_read_never_guessed() {
    assert_eq!(vat_pct(Some("VAT_20")), Some(20));
    assert_eq!(vat_pct(Some("VAT_6")), Some(6));
    assert_eq!(vat_pct(Some("VAT_0")), Some(0));
    assert_eq!(vat_pct(Some("TVSH_20")), None);
    assert_eq!(vat_pct(None), None);
}

#[test]
fn a_course_becomes_a_served_dine_in_order() {
    let o = to_order(&course_sale(), "loc-dubin").unwrap();
    assert_eq!(o["id"], json!(format!("ebills:{UUID_COURSE}")));
    assert_eq!(
        o["status"],
        json!("PICKED_UP"),
        "served and paid: the terminal with no courier leg"
    );
    assert_eq!(o["channel"], json!("ebills"));
    assert_eq!(o["location_id"], json!("loc-dubin"));
    assert_eq!(
        o["fulfilment"],
        json!({ "kind": "dine_in", "table": "12", "fee": 0 })
    );
    assert_eq!(o["payment"], json!("cash"));
    assert_eq!(
        (o["subtotal"].as_i64(), o["total"].as_i64()),
        (Some(500), Some(500))
    );
    assert_eq!(
        (o["delivery_fee"].as_i64(), o["tip"].as_i64()),
        (Some(0), Some(0))
    );
    assert_eq!(o["created_at_ms"], json!(1_789_981_285_857_i64));
    assert_eq!(
        o["price_trusted"],
        json!(false),
        "these prices are not this hub's catalogue's"
    );
    assert_eq!(o["customer_id"], Value::Null);
    assert_eq!(o["contact"], json!({ "name": "", "phone": "" }));
    let items = o["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0],
        json!({
            "product_id": "ebills:56", "name": "korca", "quantity": 2, "unit_price": 250,
            "modifier_ids": [], "vat_rate_pct": 20, "discount_pct": 0
        })
    );
    assert_eq!(
        o["external"],
        json!({
            "source": "ebills", "sale_id": 8602, "inv_ord_num": 8598, "uuid": UUID_COURSE,
            "fic": FIC, "iic": IIC, "pos_id": 1, "sale_unit_order_id": 5614, "operator_id": 1,
            "log_cis_len": 1, "sale_unit_kind": "TABLE"
        })
    );
    let text = o.to_string();
    assert!(
        !text.contains("xx000xx000"),
        "the operator's CODE never travels, only the id"
    );
    assert!(
        !text.contains("Default Client"),
        "no client name in an order record"
    );
}

#[test]
fn the_same_sale_twice_is_the_same_id() {
    let a = to_order(&course_sale(), "loc").unwrap();
    let b = to_order(&course_sale(), "loc").unwrap();
    assert_eq!(a["id"], b["id"]);
    assert_eq!(a, b, "a pure map: same bytes in, same envelope out");
}

#[test]
fn a_counter_sale_is_collected_not_seated() {
    let s = course_with(|v| {
        v["saleUnitOrder"] = Value::Null;
        v["saleUnit"] = Value::Null;
    });
    let o = to_order(&s, "loc").unwrap();
    assert_eq!(
        o["fulfilment"],
        json!({ "kind": "pickup", "code": "", "fee": 0 })
    );
    assert_eq!(o["external"]["sale_unit_order_id"], Value::Null);
}

#[test]
fn a_bill_is_not_an_order_it_is_the_paid_fact() {
    let list = parse_list(&json!({"sales":[bill_listed()],"total":160.0}).to_string()).unwrap();
    let bill = &list.sales[0];
    assert_eq!(to_order(bill, "loc"), Err(MapError::BillIsNotAnOrder));
    let paid = to_paid(bill).unwrap();
    assert_eq!(paid["total"], json!(160));
    assert_eq!(paid["payment"], json!("cash"));
    assert_eq!(paid["table"], json!("14"));
    assert_eq!(paid["at_ms"], json!(1_789_981_279_404_i64));
    assert_eq!(paid["external"]["iic"], json!(IIC));
    assert_eq!(paid["external"]["sale_unit_order_id"], Value::Null);
    assert_eq!(
        to_paid(&course_sale()),
        Err(MapError::BillIsNotAnOrder),
        "and a course is not a bill"
    );
}

#[test]
fn refusals_name_the_reason() {
    let fractional = course_with(|v| {
        v["saleRecords"][0]["amount"] = json!(0.35);
        v["saleRecords"][0]["totalValue"] = json!(87.5);
        v["totalValue"] = json!(87.5);
    });
    assert!(matches!(
        to_order(&fractional, "l"),
        Err(MapError::NotWhole {
            field: "amount",
            ..
        })
    ));

    let mismatch = course_with(|v| v["saleRecords"][0]["totalValue"] = json!(499.0));
    assert_eq!(
        to_order(&mismatch, "l"),
        Err(MapError::LineMismatch {
            line: 0,
            expected: 500,
            got: 499
        })
    );

    // THE MEASURED DISCOUNT (2026-09-23): `price` is already discounted and
    // `discount` is a percent -- a free dish is `0.0 / 100.0 / 0.0`.
    let free = course_with(|v| {
        v["saleRecords"][0]["price"] = json!(0.0);
        v["saleRecords"][0]["discount"] = json!(100.0);
        v["saleRecords"][0]["totalValue"] = json!(0.0);
        v["totalValue"] = json!(0.0);
    });
    let o = to_order(&free, "l").unwrap();
    assert_eq!((o["items"][0]["discount_pct"].as_i64(), o["total"].as_i64(), o["discount"].as_i64()), (Some(100), Some(0), Some(0)));
    // A partial discount on a LIST price would not add up: refused, not guessed.
    let partial = course_with(|v| {
        v["saleRecords"][0]["discount"] = json!(50.0);
        v["saleRecords"][0]["totalValue"] = json!(250.0);
        v["totalValue"] = json!(250.0);
    });
    assert_eq!(to_order(&partial, "l"), Err(MapError::LineMismatch { line: 0, expected: 500, got: 250 }));
    let over = course_with(|v| v["saleRecords"][0]["discount"] = json!(150.0));
    assert!(matches!(to_order(&over, "l"), Err(MapError::NotWhole { field: "discount", .. })));

    let total_off = course_with(|v| v["totalValue"] = json!(600.0));
    assert_eq!(
        to_order(&total_off, "l"),
        Err(MapError::TotalMismatch {
            expected: 500,
            got: 600
        })
    );

    let draft = course_with(|v| v["draft"] = json!(1));
    assert_eq!(
        to_order(&draft, "l"),
        Err(MapError::NotFinished {
            status: "CLOSED".into(),
            fiscal: "FINISHED".into(),
            draft: 1
        })
    );
    let open = course_with(|v| v["status"] = json!("OPEN"));
    assert!(matches!(
        to_order(&open, "l"),
        Err(MapError::NotFinished { .. })
    ));
    // MEASURED: a course at a table still open is `OPENED`, fiscalised, and
    // listed. Taken -- a counter sale that says `OPENED` is not.
    let opened = course_with(|v| v["status"] = json!("OPENED"));
    assert!(to_order(&opened, "l").is_ok());
    let open_counter = course_with(|v| {
        v["status"] = json!("OPENED");
        v["saleUnitOrder"] = Value::Null;
    });
    assert!(matches!(to_order(&open_counter, "l"), Err(MapError::NotFinished { .. })));
    let changed = course_with(|v| v["changedStatus"] = json!("MODIFIED"));
    assert!(matches!(to_order(&changed, "l"), Err(MapError::NotFinished { .. })));

    let failed = course_with(|v| v["logCis"][0]["status"] = json!("ERROR"));
    assert_eq!(to_order(&failed, "l"), Err(MapError::NotFiscalised));
    let unsent = course_with(|v| v["logCis"] = json!([]));
    assert_eq!(to_order(&unsent, "l"), Err(MapError::NotFiscalised));
    let no_fic = course_with(|v| v["fic"] = Value::Null);
    assert_eq!(to_order(&no_fic, "l"), Err(MapError::NotFiscalised));

    let paysera = course_with(|v| v["paymentMethod"] = json!("PAYSERA"));
    assert_eq!(
        to_order(&paysera, "l"),
        Err(MapError::Payment("PAYSERA".into()))
    );

    let euro = course_with(|v| v["currency"]["currencyCode"] = json!("EUR"));
    assert_eq!(
        to_order(&euro, "l"),
        Err(MapError::Currency("EUR @ 1".into()))
    );
    let rate = course_with(|v| v["currencyRate"] = json!(1.5));
    assert!(matches!(to_order(&rate, "l"), Err(MapError::Currency(_))));

    let listed = course_with(|v| v["saleRecords"] = Value::Null);
    assert_eq!(
        to_order(&listed, "l"),
        Err(MapError::NoLines),
        "a LIST row has no lines; map the detail"
    );
    let empty = course_with(|v| v["saleRecords"] = json!([]));
    assert_eq!(to_order(&empty, "l"), Err(MapError::NoLines));

    let zero_qty = course_with(|v| {
        v["saleRecords"][0]["amount"] = json!(0.0);
        v["saleRecords"][0]["totalValue"] = json!(0.0);
        v["totalValue"] = json!(0.0);
    });
    assert_eq!(
        to_order(&zero_qty, "l"),
        Err(MapError::Negative { field: "amount" })
    );
}

/// A VOID, AS MEASURED (sale 8718 -> 8717): `changedStatus: CANCELLED`,
/// negative lines, and `modified` naming the sale it reverses. Mapped to its
/// own negative order that names what it voids; only a void may be negative.
#[test]
fn a_void_is_a_negative_order_that_names_what_it_voids() {
    let void = course_with(|v| {
        v["changedStatus"] = json!("CANCELLED");
        v["modified"] = json!({ "id": 8601, "uuid": UUID_BILL, "totalValue": 500.0 });
        v["saleRecords"][0]["amount"] = json!(-2.0);
        v["saleRecords"][0]["totalValue"] = json!(-500.0);
        v["totalValue"] = json!(-500.0);
    });
    let o = to_order(&void, "l").unwrap();
    assert_eq!((o["total"].as_i64(), o["items"][0]["quantity"].as_i64()), (Some(-500), Some(-2)));
    assert_eq!(o["void_of"], json!(format!("ebills:{UUID_BILL}")));
    assert_eq!(o["external"]["void_of_sale_id"], json!(8601));
    let orphan = course_with(|v| {
        v["changedStatus"] = json!("CANCELLED");
        v["saleRecords"][0]["amount"] = json!(-2.0);
        v["saleRecords"][0]["totalValue"] = json!(-500.0);
        v["totalValue"] = json!(-500.0);
    });
    assert!(matches!(to_order(&orphan, "l"), Err(MapError::NotFinished { .. })), "a void must name its sale");
    let negative = course_with(|v| {
        v["saleRecords"][0]["amount"] = json!(-2.0);
        v["saleRecords"][0]["totalValue"] = json!(-500.0);
        v["totalValue"] = json!(-500.0);
    });
    assert_eq!(to_order(&negative, "l"), Err(MapError::Negative { field: "amount" }), "only a void is negative");
}

#[test]
fn an_item_without_a_code_keeps_its_name_and_no_product() {
    let s = course_with(|v| v["saleRecords"][0]["itemInSale"] = Value::Null);
    let o = to_order(&s, "l").unwrap();
    assert_eq!(o["items"][0]["product_id"], Value::Null);
    assert_eq!(o["items"][0]["name"], json!("korca"));
}

#[test]
fn the_floor_is_read_and_the_server_is_not_kept() {
    let body = r#"[
        {"id": 17, "identifier": "17", "type": "TABLE", "status": "OCCUPIED", "pointOfSaleId": 1,
         "posX": 2.0, "posY": 3.0, "guests": "0", "orderTotal": 5600.0, "server": "Some Person",
         "activeUser": {"id": 1, "user": {"login": "someone@example.com"}}, "size": null, "def": null,
         "note": null, "saleUnitCategoryId": null},
        {"id": 1, "identifier": "1", "type": "TABLE", "status": "ACTIVE", "pointOfSaleId": 1,
         "posX": 1.0, "posY": 1.0, "guests": null, "orderTotal": null, "server": null, "activeUser": null}
    ]"#;
    let tables = parse_tables(body).unwrap();
    assert_eq!(tables.len(), 2);
    assert!(tables[0].occupied());
    assert!(!tables[1].occupied());
    assert_eq!(tables[0].order_total, Some(5600.0));
    assert_eq!(tables[1].order_total, None);
    assert_eq!(tables[0].identifier, "17");
    let dump = format!("{:?}", tables);
    assert!(
        !dump.contains("Some Person") && !dump.contains("example.com"),
        "{dump}"
    );
    assert_eq!(lek(tables[0].order_total.unwrap(), "orderTotal"), Ok(5600));
}

#[test]
fn a_reserved_table_is_not_occupied() {
    let body = json!([{"id": 3, "identifier": "3", "type": "TABLE", "status": "RESERVED", "pointOfSaleId": 1, "orderTotal": null}]).to_string();
    assert!(!parse_tables(&body).unwrap()[0].occupied());
}

/// DOWIZ'S OWN FISCAL SALE is recognised by its `notes` marker, so the
/// poller never imports it back as a second order (L70). Twin: the till's
/// own sale -- `notes: null` -- is not ours.
#[test]
fn a_sale_dowiz_created_is_ours_and_the_tills_is_not() {
    let till: Sale = serde_json::from_value(course()).expect("the measured course parses");
    assert!(!ours(&till), "notes null: the till's sale");
    let mut v = course();
    v["notes"] = json!("dowiz:ord_42");
    let mine: Sale = serde_json::from_value(v).unwrap();
    assert!(ours(&mine));
    let mut other = course();
    other["notes"] = json!("table by the window, dowiz:ord_42");
    assert!(!ours(&serde_json::from_value::<Sale>(other).unwrap()), "only a note that STARTS with the marker");
}

/// The item rows keep their id and the whole row (the create echoes it).
#[test]
fn item_rows_carry_the_id_and_the_row_as_served() {
    let body = r#"[{"id":23,"itemCode":"16","item":"Coca Cola","price":200.0,"vat":"VAT_20","unit":{"id":1839}},
                   {"id":24,"itemCode":"","item":"no code","price":1.0},{"itemCode":"17","item":"no id","price":1.0}]"#;
    let rows = parse_item_rows(body).expect("parses");
    assert_eq!(rows.len(), 1, "no code or no id cannot be named in a create");
    assert_eq!((rows[0].0.as_str(), rows[0].1), ("16", 23));
    assert_eq!(rows[0].2["unit"]["id"], json!(1839), "the row passes through whole");
    assert_eq!(parse_items(body).unwrap().len(), 2, "the importer's read is unchanged: code + whole price");
}
