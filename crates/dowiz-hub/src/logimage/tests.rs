use super::*;

/// The mistake this floor exists for, pinned so it cannot come back.
#[test]
fn an_image_below_the_arena_is_raised_rather_than_born_useless() {
    let mut tiny = LogImage::create_sized(1).unwrap();
    tiny.append("e", "v", "{}").expect("an image with no room refused its first record");
    assert!(MIN_LOG_BYTES > bebop_store::ARENA * 8);
}

/// A TRUNCATED IMAGE IS NOT A SHORTER LOG, and for two days it was read as
/// one. The superblock is fifteen cells at the FRONT of the image, so it
/// survives a cut that takes half the records with it: the image loaded,
/// `len()` answered 40, and `entries()` handed back two. A short read on a
/// chunked fetch from a Durable Object is the realistic failure here, not a
/// theoretical one -- and a venue cannot notice thirty-eight missing
/// orders in a reply that looks exactly like a correct one.
///
/// The cuts are FIXED fractions, not random ones: the property is a law
/// about every prefix, and a fixed set of them says so without a generator.
#[test]
fn a_truncated_image_is_refused_rather_than_read_short() {
    let mut l = LogImage::create().unwrap();
    for i in 0..40 {
        l.append("e", &format!("s{}", i % 7), &format!(r#"{{"n":{i}}}"#)).unwrap();
    }
    let bytes = l.to_bytes();
    let mut refused = 0;
    for tenth in 1..10usize {
        let keep = bytes.len() * tenth / 10;
        match LogImage::load(&bytes[..keep]) {
            Err(_) => refused += 1,
            Ok(short) => assert_eq!(
                short.len(),
                short.entries().len() + short.quarantined().len(),
                "a log cut to {keep} of {} bytes lost a record to neither list",
                bytes.len()
            ),
        }
    }
    assert!(refused > 0, "no prefix of a 40-record log was refused");
    // And the whole image still loads whole -- a check that refuses
    // everything is not a check.
    assert_eq!(LogImage::load(&bytes).unwrap().entries().len(), 40);
}

/// L5 ON THIS SIDE OF THE HOUSE. The order log is not the only log: the
/// audit image holds the venue's own failures, and a record it cannot read
/// is the INSTRUMENT losing records. So the same law, and it is arithmetic:
/// `len() == entries().len() + quarantined().len()`.
///
/// The reason is named rather than "it did not decode", because the
/// quarantine list is evidence for a human and "it did not decode" is not
/// evidence.
#[test]
fn a_record_this_build_cannot_read_is_quarantined_and_the_log_still_answers() {
    let mut l = LogImage::create().unwrap();
    for i in 0..6 {
        l.append("error", "notify.telegram", &format!(r#"{{"n":{i}}}"#)).unwrap();
    }
    let mut st = bebop_store::Store::from_bytes(&l.to_bytes());
    let root = st.root().expect("root");
    let newest = st.follow(root, 1).expect("newest");
    // v2 packs eight payload bytes per cell after a twelve-cell header,
    // plus four more when an actor key is present (bit 0 of cell 11). This
    // log appends without an actor, so the payload starts at twelve.
    let at = if st.get(newest, 11) & 1 != 0 { 16 } else { 12 };
    let cell = st.get(newest, at);
    // Byte 0 is the kind's LENGTH, and 255 runs off the end of a payload
    // this short -- the framing damage a truncated write would leave.
    st.cells[newest + 2 + at] = (cell & !0xFF) | 0xFF;
    // SEALED (W-CRC): see `tests::a_record_this_build_cannot_read_is_quarantined...`.
    st.seal(newest);

    let broken = LogImage::load(&st.to_bytes()).expect("one bad record must not refuse the image");
    assert_eq!(broken.len(), 6, "the image still holds six records");
    assert_eq!(broken.entries().len(), 5, "the unreadable one is not served");
    let q = broken.quarantined();
    assert_eq!(q.len(), 1, "and it is named: {q:?}");
    assert_eq!(q[0].reason, "kind-framing");
    assert_eq!(q[0].at, 0, "it was the newest record");
    assert_eq!(q[0].id.len(), 64, "the id is there for a human to find");
    assert_eq!(
        broken.len(),
        broken.entries().len() + broken.quarantined().len(),
        "a record must be in exactly one of the two lists"
    );
    // And the filtered query -- what the console actually calls -- still
    // answers, which is the point of not refusing the image.
    assert_eq!(broken.about("error", Some("notify.telegram"), 10).len(), 5);
}

/// The other half: the image is all there, and one `next` ref is not.
#[test]
fn a_chain_ref_that_leaves_the_image_is_refused() {
    let mut l = LogImage::create().unwrap();
    for i in 0..6 {
        l.append("e", "s", &format!(r#"{{"n":{i}}}"#)).unwrap();
    }
    let mut st = bebop_store::Store::from_bytes(&l.to_bytes());
    let root = st.root().expect("root");
    let newest = st.follow(root, 1).expect("newest");
    // Payload cell 2 of a record is the ref to the next (older) one.
    st.cells[newest + 2 + 2] = 1 << 40;
    st.seal(newest); // W-CRC: only the chain lies, not the crc
    let broken = st.to_bytes();
    assert!(
        matches!(LogImage::load(&broken), Err(HubError::Corrupt { claimed: 6, .. })),
        "a chain that stops early must be refused, not read short"
    );
}

#[test]
fn a_record_comes_back_newest_first() {
    let mut l = LogImage::create().unwrap();
    l.append("err", "dubin", r#"{"m":"one"}"#).unwrap();
    l.append("err", "sushi", r#"{"m":"two"}"#).unwrap();
    let e = l.entries();
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].json, r#"{"m":"two"}"#);
    assert_eq!(e[0].subject, "sushi");
    assert_eq!(e[0].seq, 1);
    assert_eq!(e[1].seq, 0);
}

#[test]
fn it_survives_a_round_trip_through_bytes() {
    let mut l = LogImage::create().unwrap();
    for i in 0..20 {
        l.append("msg", "t1", &format!(r#"{{"n":{i}}}"#)).unwrap();
    }
    let back = LogImage::load(&l.to_bytes()).unwrap();
    assert_eq!(back.len(), 20);
    assert_eq!(back.entries()[0].json, r#"{"n":19}"#);
    assert!(back.chain_check().intact());
}

#[test]
fn about_filters_by_kind_and_subject() {
    let mut l = LogImage::create().unwrap();
    l.append("msg", "t1", "{}").unwrap();
    l.append("msg", "t2", "{}").unwrap();
    l.append("err", "t1", "{}").unwrap();
    assert_eq!(l.about("msg", None, 10).len(), 2);
    assert_eq!(l.about("msg", Some("t1"), 10).len(), 1);
    assert_eq!(l.about("err", Some("t2"), 10).len(), 0);
    assert_eq!(l.about("msg", None, 1).len(), 1, "the limit is the limit");
}

/// The order log's expensive lesson, inherited: it GROWS.
#[test]
fn it_grows_rather_than_refusing() {
    let mut l = LogImage::create_sized(MIN_LOG_BYTES).unwrap();
    for i in 0..400 {
        l.append("err", "v", &format!(r#"{{"n":{i},"pad":"{}"}}"#, "x".repeat(80)))
            .unwrap_or_else(|e| panic!("refused at {i}: {e:?}"));
    }
    assert_eq!(l.len(), 400);
    assert!(l.chain_check().intact(), "growing broke the chain");
    assert_eq!(l.entries()[0].json.contains("\"n\":399"), true);
}

#[test]
fn the_chain_survives_a_round_trip_after_growing() {
    let mut l = LogImage::create_sized(MIN_LOG_BYTES).unwrap();
    for i in 0..200 {
        l.append("e", "v", &format!(r#"{{"n":{i}}}"#)).unwrap();
    }
    let back = LogImage::load(&l.to_bytes()).unwrap();
    let c = back.chain_check();
    assert_eq!(c.records, 200);
    assert_eq!(c.broken, 0);
}

#[test]
fn keep_drops_the_oldest_and_leaves_a_valid_chain() {
    let mut l = LogImage::create().unwrap();
    for i in 0..50 {
        l.append("e", "v", &format!(r#"{{"n":{i}}}"#)).unwrap();
    }
    assert_eq!(l.keep(10).unwrap(), 40);
    assert_eq!(l.len(), 10);
    let e = l.entries();
    assert_eq!(e[0].json, r#"{"n":49}"#, "the newest survives");
    assert_eq!(e[9].json, r#"{"n":40}"#, "and the tenth-newest is the oldest left");
    assert!(l.chain_check().intact(), "the rebuilt chain does not verify");
    // And it keeps working afterwards.
    l.append("e", "v", r#"{"n":50}"#).unwrap();
    assert_eq!(l.len(), 11);
    assert!(l.chain_check().intact());
}

#[test]
fn keeping_more_than_there_are_drops_nothing() {
    let mut l = LogImage::create().unwrap();
    l.append("e", "v", "{}").unwrap();
    assert_eq!(l.keep(10).unwrap(), 0);
    assert_eq!(l.len(), 1);
}

/// An EDITED record is what a chained id is for.
#[test]
fn chain_check_finds_an_edit() {
    let mut l = LogImage::create().unwrap();
    for i in 0..5 {
        l.append("e", "v", &format!(r#"{{"n":{i}}}"#)).unwrap();
    }
    assert!(l.chain_check().intact());
    // Rebuild the image with one payload changed but the ORIGINAL ids kept,
    // which is what editing the stored bytes would look like.
    let mut records = EvLog::walk(&l.store);
    records.reverse();
    let mut fresh = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut fresh).unwrap();
    let mut last = None;
    for (i, r) in records.iter().enumerate() {
        let mut r = r.clone();
        if i == 2 {
            r.payload = encode("e", "v", r#"{"n":999}"#).unwrap();
        }
        EvLog::append_bytes(&mut fresh, &r).unwrap();
        last = Some(r.id);
    }
    EvLog::set_tip_bytes(&mut fresh, &last.unwrap()).unwrap();
    let tampered = LogImage { store: fresh };
    let c = tampered.chain_check();
    assert_eq!(c.records, 5);
    assert_eq!(c.broken, 1, "an edited payload did not break its id");
}

#[test]
fn a_payload_with_a_long_kind_or_subject_is_refused_rather_than_truncated() {
    let mut l = LogImage::create().unwrap();
    let long = "x".repeat(256);
    assert!(l.append(&long, "v", "{}").is_err());
    assert!(l.append("e", &long, "{}").is_err());
    assert_eq!(l.len(), 0, "a refused append left a record behind");
}

#[test]
fn json_with_the_separator_bytes_in_it_still_decodes() {
    let mut l = LogImage::create().unwrap();
    // A payload whose text contains what could be read as a length prefix.
    let tricky = r#"{"m":"\u0001a\u0002bb","n":1}"#;
    l.append("e", "subj", tricky).unwrap();
    assert_eq!(l.entries()[0].json, tricky);
}
