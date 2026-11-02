use super::*;

/// Cells go to the device little-endian, eight bytes each.
#[test]
fn cells_are_little_endian_bytes() {
    assert_eq!(cells_to_bytes(&[1, -1]), [[1, 0, 0, 0, 0, 0, 0, 0], [0xff; 8]].concat());
}
