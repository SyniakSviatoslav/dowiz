#![no_main]
use libfuzzer_sys::fuzz_target;
use dowiz_kernel::trinary::Tri;

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 { return; }
    let a = match data[0] % 3 {
        0 => Tri::True, 1 => Tri::False, _ => Tri::Unknown
    };
    let b = match data[1] % 3 {
        0 => Tri::True, 1 => Tri::False, _ => Tri::Unknown
    };
    let c = match data[2] % 3 {
        0 => Tri::True, 1 => Tri::False, _ => Tri::Unknown
    };
    assert_eq!(a.not().not(), a, "Double negation must return original");
    assert_eq!(a.and(b), b.and(a), "AND must be commutative");
    assert_eq!(a.or(b), b.or(a), "OR must be commutative");
    let ab_and_c = a.and(b).and(c);
    let a_and_bc = a.and(b.and(c));
    assert_eq!(ab_and_c, a_and_bc, "AND must be associative");
    assert_eq!(a.and(b).not(), a.not().or(b.not()), "De Morgan: not(A and B) = not A or not B");
    assert_eq!(a.or(b).not(), a.not().and(b.not()), "De Morgan: not(A or B) = not A and not B");
});
