use super::*;

#[test]
fn the_newest_op_per_key_wins_and_a_refused_put_changes_nothing() {
    let mut b = Batch::new(Limits { key: 4, value: 4 });
    b.put(b"a", b"1").unwrap();
    b.remove(b"a").unwrap();
    b.put(b"b", b"2").unwrap();
    assert_eq!(b.get(b"a"), Some(None));
    assert_eq!(b.get(b"b"), Some(Some(&b"2"[..])));
    assert_eq!(b.get(b"c"), None);
    let before = b.clone();
    assert_eq!(b.put(b"toolong", b"x"), Err(LimitError::KeyTooLong { len: 7, max: 4 }));
    assert_eq!(b.put(b"k", b"12345"), Err(LimitError::ValueTooLong { len: 5, max: 4 }));
    assert_eq!(b, before);
    let ops: Vec<KeyOp> = b.ops().collect();
    assert_eq!(ops, [KeyOp::Remove(b"a"), KeyOp::Put(b"b", b"2")]);
}

#[test]
fn a_reversed_or_empty_range_is_none_not_a_panic() {
    let mut b = Batch::default();
    b.put(b"m", b"1").unwrap();
    assert!(b.range(Bound::Included(&b"z"[..]), Bound::Included(&b"a"[..])).is_none());
    assert!(b.range(Bound::Excluded(&b"m"[..]), Bound::Excluded(&b"m"[..])).is_none());
    assert!(b.range(Bound::Included(&b"m"[..]), Bound::Excluded(&b"m"[..])).is_none());
    assert_eq!(b.range(Bound::Included(&b"m"[..]), Bound::Included(&b"m"[..])).map(|r| r.count()), Some(1));
    assert_eq!(b.range(Bound::Unbounded, Bound::Unbounded).map(|r| r.count()), Some(1));
}
