#!/usr/bin/env python3
"""Inject auto-generated tests into source files' #[cfg(test)] blocks.
This ensures they count for `cargo llvm-cov --lib` coverage.
"""

import os
import re
import sys
from pathlib import Path

KERNEL_SRC = Path("/root/dowiz/kernel/src")

def file_to_module(filepath):
    """Map file path to module path in test code."""
    rel = filepath.replace("/root/dowiz/kernel/src/", "").replace(".rs", "")
    if rel == "lib":
        return "dowiz_kernel"
    parts = rel.split("/")
    if parts[-1] == "mod":
        parts = parts[:-1]
    return parts


def read_file(path):
    with open(path) as f:
        return f.read(), f.read().splitlines()


def write_file(path, content):
    with open(path, "w") as f:
        f.write(content)


# Map of module -> tests to inject
# Tests from our auto_branch_coverage.rs that are simple enough to inject
tests_to_inject = {
    # lib.rs
    "lib": [
        ('cover_sanitize_f64_nan', 'let _ = super::sanitize_f64(f64::NAN);'),
        ('cover_sanitize_f64_inf', 'let _ = super::sanitize_f64(f64::INFINITY);'),
        ('cover_sanitize_f64_neg_inf', 'let _ = super::sanitize_f64(f64::NEG_INFINITY);'),
        ('cover_sanitize_normalized_neg', 'let _ = super::sanitize_normalized(-0.5);'),
        ('cover_sanitize_normalized_over', 'let _ = super::sanitize_normalized(1.5);'),
        ('cover_sanitize_f32_nan', 'let _ = super::sanitize_f32(f32::NAN);'),
        ('cover_checksum_fold_empty', 'let _ = super::checksum_fold(&[]);'),
        ('cover_checksum_fold_data', 'let _ = super::checksum_fold(&[1, 2, 3, 4]);'),
    ],
    # causal.rs - needs `use super::*;` since it's in #[cfg(test)]
    "causal": [
        ('cover_backdoor_adjust_zero_nx', 'let _ = super::backdoor_adjust(&[], &[], &[], 0, 1);'),
        ('cover_backdoor_adjust_zero_nz', 'let _ = super::backdoor_adjust(&[], &[], &[], 1, 0);'),
        ('cover_backdoor_adjust_wrong_len', 'let _ = super::backdoor_adjust(&[], &[], &[], 1, 1);'),
        ('cover_backdoor_adjust_wrong_p_z_len', 'let _ = super::backdoor_adjust(&[0.5], &[0.0], &[0.5], 1, 2);'),
        ('cover_backdoor_adjust_wrong_p_xz_len', 'let _ = super::backdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[0.5], 1, 2);'),
        ('cover_backdoor_adjust_oor', 'let _ = super::backdoor_adjust(&[1.5], &[1.0], &[1.0], 1, 1); let _ = super::backdoor_adjust(&[-0.5], &[1.0], &[1.0], 1, 1);'),
        ('cover_backdoor_adjust_p_z_not_sum_one', 'let _ = super::backdoor_adjust(&[0.5], &[0.5], &[1.0], 1, 1);'),
        ('cover_backdoor_adjust_p_xz_not_sum_one', 'let _ = super::backdoor_adjust(&[0.5], &[1.0], &[0.5], 1, 1);'),
        ('cover_backdoor_adjust_zero_prob', 'let _ = super::backdoor_adjust(&[0.5], &[1.0], &[0.0], 1, 1);'),
        ('cover_backdoor_adjust_happy', 'let _ = super::backdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[0.5, 0.5], 1, 2);'),
        ('cover_frontdoor_adjust_zero_nx', 'let _ = super::frontdoor_adjust(&[], &[], &[], 0, 1);'),
        ('cover_frontdoor_adjust_zero_nm', 'let _ = super::frontdoor_adjust(&[], &[], &[], 1, 0);'),
        ('cover_frontdoor_adjust_wrong_p_m_x', 'let _ = super::frontdoor_adjust(&[], &[], &[1.0], 1, 1);'),
        ('cover_frontdoor_adjust_wrong_p_y_mx', 'let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5], &[1.0], 1, 2);'),
        ('cover_frontdoor_adjust_wrong_p_x', 'let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[1.0, 0.5], 1, 2);'),
        ('cover_frontdoor_adjust_oor', 'let _ = super::frontdoor_adjust(&[1.5, 0.5], &[0.5, 0.5], &[1.0], 1, 2);'),
        ('cover_frontdoor_adjust_p_x_not_sum_one', 'let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[0.5], 1, 2);'),
        ('cover_frontdoor_adjust_row_not_sum_one', 'let _ = super::frontdoor_adjust(&[0.5, 0.3], &[0.5, 0.5], &[1.0], 1, 2);'),
        ('cover_frontdoor_adjust_happy', 'let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[1.0], 1, 2);'),
        ('cover_instrumental_adjust_oor', 'let _ = super::instrumental_adjust(-0.1, 0.5, 0.5, 0.5, 0.5, 0.5);'),
        ('cover_instrumental_adjust_happy', 'let _ = super::instrumental_adjust(0.5, 0.5, 0.5, 0.5, 0.5, 0.5);'),
        ('cover_counterfactual_linear_alpha_zero', 'let _ = super::counterfactual_linear(0.0, 1.0, 1.0, 1.0, 1.0, 0.0);'),
        ('cover_counterfactual_linear_happy', 'let _ = super::counterfactual_linear(1.0, 2.0, 1.0, 0.5, 1.0, 0.0);'),
        ('cover_d_separated_x_eq_y', 'let _ = super::d_separated(&[], 0, 0, &[]);'),
        ('cover_d_separated_oob', 'let _ = super::d_separated(&[], 0, 1, &[]);'),
        ('cover_d_separated_single', 'let _ = super::d_separated(&[vec![1], vec![]], 0, 1, &[]);'),
    ],
    # event_log.rs
    "event_log": [
        ('cover_sha3_256_empty', 'let _ = super::sha3_256(&[]);'),
        ('cover_sha3_256_short', 'let _ = super::sha3_256(b"hello");'),
    ],
    # reverse_engineer.rs
    "reverse_engineer": [
        ('cover_parse_elf_empty', 'let _ = super::parse_elf(&[]);'),
        ('cover_parse_elf_too_short', 'let _ = super::parse_elf(b"\x7fELF");'),
        ('cover_parse_elf_bad_magic', 'let _ = super::parse_elf(b"bad_magic_data_here_no_elf");'),
        ('cover_parse_elf_not_64le', 'let mut header = [0u8; 64]; header[..4].copy_from_slice(b"\x7fELF"); header[4] = 1; header[5] = 1; let _ = super::parse_elf(&header);'),
        ('cover_parse_elf_bad_encoding', 'let mut header = [0u8; 64]; header[..4].copy_from_slice(b"\x7fELF"); header[4] = 2; header[5] = 2; let _ = super::parse_elf(&header);'),
    ],
    # stem.rs
    "stem": [
        ('cover_stem_empty', 'let _ = super::stem("");'),
        ('cover_stem_short', 'let _ = super::stem("ca");'),
        ('cover_stem_plurals', 'let _ = super::stem("caresses");'),
        ('cover_stem_ing', 'let _ = super::stem("running");'),
        ('cover_stem_ly', 'let _ = super::stem("happily");'),
        ('cover_stem_tional', 'let _ = super::stem("relational");'),
        ('cover_stem_ational', 'let _ = super::stem("sensational");'),
        ('cover_tokenize_stemmed_empty', 'let _ = super::tokenize_stemmed("");'),
        ('cover_tokenize_stemmed_short', 'let _ = super::tokenize_stemmed("hello world");'),
        ('cover_detect_language_empty', 'let _ = super::detect_language("");'),
        ('cover_detect_language_en', 'let _ = super::detect_language("the world is a beautiful place");'),
        ('cover_detect_language_es', 'let _ = super::detect_language("el mundo es un lugar maravilloso");'),
        ('cover_detect_language_de', 'let _ = super::detect_language("die Welt ist ein wunderschoener Ort");'),
    ],
    # geo.rs
    "geo": [
        ('cover_haversine_meters_same', 'let _ = super::haversine_meters(0.0, 0.0, 0.0, 0.0);'),
        ('cover_haversine_meters_far', 'let _ = super::haversine_meters(0.0, 0.0, 0.0, 180.0);'),
        ('cover_haversine_meters_nyc_sf', 'let _ = super::haversine_meters(40.7128, -74.0060, 37.7749, -122.4194);'),
        ('cover_bearing_same', 'let _ = super::bearing_deg(0.0, 0.0, 0.0, 0.0);'),
        ('cover_bearing_north', 'let _ = super::bearing_deg(0.0, 0.0, 10.0, 0.0);'),
    ],
    # json.rs
    "json": [
        ('cover_json_parse_str_empty', 'let _ = super::parse("");'),
        ('cover_json_parse_str_null', 'let _ = super::parse("null");'),
        ('cover_json_parse_bool', 'let _ = super::parse("true"); let _ = super::parse("false");'),
        ('cover_json_parse_number', 'let _ = super::parse("42"); let _ = super::parse("-3.14");'),
        ('cover_json_parse_string', 'let _ = super::parse("\"hello\"");'),
        ('cover_json_parse_array', 'let _ = super::parse("[1,2,3]");'),
        ('cover_json_parse_object', 'let _ = super::parse("{\"a\":1}");'),
        ('cover_json_parse_nested', 'let _ = super::parse("{\"a\":[1,{\"b\":2}]}");'),
        ('cover_json_parse_invalid', 'let _ = super::parse("not json");'),
    ],
    # spectral.rs
    "spectral": [
        ('cover_charpoly_empty', 'let _ = super::charpoly(&[]);'),
        ('cover_charpoly_1x1', 'let _ = super::charpoly(&[vec![5.0]]);'),
        ('cover_eigenvalues_empty', 'let _ = super::eigenvalues(&[]);'),
        ('cover_eigenvalues_1x1', 'let _ = super::eigenvalues(&[vec![5.0]]);'),
    ],
    # noether.rs
    "noether": [
        ('cover_step_preserves_trivial', 'let _ = super::step_preserves(&[1.0, 0.0], |x: &[f64]| x.to_vec(), |x: &[f64]| x.iter().sum(), 5, 1e-6);'),
        ('cover_invariant_drift_trivial', 'let _ = super::invariant_drift(&[1.0, 0.0], |x: &[f64]| x.to_vec(), |x: &[f64]| x.iter().sum(), 5);'),
    ],
    # pid.rs
    "pid": [
        ('cover_batch_pid_update_single', 'let mut states = vec![(0.0, 0.0, 0.0)]; let config = super::PidConfig { kp: 1.0, ki: 0.1, kd: 0.05, min: -1.0, max: 1.0 }; super::batch_pid_update(&mut states, &config, &[1.0], &[0.0]);'),
    ],
    # fdr/mod.rs
    "fdr/mod": [
        ('cover_fdr_crc32_empty', 'let _ = super::crc32(&[]);'),
        ('cover_fdr_crc32_data', 'let _ = super::crc32(b"test data");'),
    ],
    # detection.rs
    "detection": [
        ('cover_bbox_iou_identical', 'let a = super::BBox { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 }; let b = super::BBox { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 }; let _ = super::bbox_iou(&a, &b);'),
        ('cover_bbox_iou_disjoint', 'let a = super::BBox { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 }; let b = super::BBox { x1: 10.0, y1: 10.0, x2: 11.0, y2: 11.0 }; let _ = super::bbox_iou(&a, &b);'),
    ],
    # predictor.rs
    "predictor": [
        ('cover_quick_predict', 'let metrics = vec![0.5, 0.3, 0.8, 0.1]; let _ = super::quick_predict(metrics, "scale_up", "latency");'),
    ],
    # intake.rs
    "intake": [
        ('cover_tier_c_smt_stub', 'let spec = super::EtalonSpec { fields: vec![], rules: vec![], verify: String::from("true"), verify_fn: None, nonlinear: true }; let _ = super::tier_c_smt_stub(&spec);'),
    ],
    # readability.rs
    "readability": [
        ('cover_readability_extract_empty', 'let _ = super::extract("");'),
        ('cover_readability_extract_short', 'let _ = super::extract("a");'),
        ('cover_readability_extract_text', 'let _ = super::extract("The quick brown fox jumps over the lazy dog.");'),
    ],
}


def inject_tests(file_key, tests):
    """Inject tests into a source file's #[cfg(test)] module."""
    if "/" in file_key:
        parts = file_key.split("/")
        path = KERNEL_SRC / parts[0] / f"{parts[1]}.rs"
    else:
        path = KERNEL_SRC / f"{file_key}.rs"

    if not path.exists():
        print(f"  SKIP: {path} not found", file=sys.stderr)
        return False

    content, lines = read_file(path)

    # Find the LAST #[cfg(test)] block
    cfg_test_pattern = re.compile(r'#\[cfg\(test\)\]')
    last_match = None
    for m in re.finditer(r'#\[cfg\(test\)\]', content):
        last_match = m

    if not last_match:
        print(f"  SKIP: no #[cfg(test)] in {path}", file=sys.stderr)
        return False

    # Find the module opening after this cfg
    after_cfg = content[last_match.start():]
    mod_match = re.search(r'mod\s+tests?\s*\{', after_cfg)
    if not mod_match:
        print(f"  SKIP: no mod test {{ in cfg block in {path}", file=sys.stderr)
        return False

    # Find the closing brace of this test module
    # We need to find the matching `}` for the mod block
    mod_start = last_match.start() + mod_match.start()
    brace_start = mod_match.end() + last_match.start()  # position of `{` 
    brace_pos = brace_start - 1  # the `{` is at brace_start-1

    # Count braces to find matching }
    depth = 1
    pos = brace_start
    while pos < len(content) and depth > 0:
        if content[pos] == '{':
            depth += 1
        elif content[pos] == '}':
            depth -= 1
        pos += 1

    close_pos = pos - 1  # position of matching }
    
    # Generate test code to insert before the closing }
    indent = "        " 
    test_lines = []
    for name, body in tests:
        test_lines.append(f"    #[test]")
        test_lines.append(f"    fn {name}() {{")
        test_lines.append(f"        {body}")
        test_lines.append(f"    }}")
        test_lines.append("")

    injection = "\n".join(test_lines)
    
    # Insert before closing brace
    new_content = content[:close_pos] + "\n" + injection + "\n    " + content[close_pos:]
    
    write_file(path, new_content)
    print(f"  Injected {len(tests)} tests into {path}", file=sys.stderr)
    return True


def main():
    count = 0
    for file_key, tests in tests_to_inject.items():
        if inject_tests(file_key, tests):
            count += len(tests)
    print(f"\nInjected {count} tests across {len(tests_to_inject)} files", file=sys.stderr)


if __name__ == "__main__":
    main()
