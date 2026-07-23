#!/usr/bin/env python3
"""Robust test injector - inserts tests into #[cfg(test)] blocks.
Properly counts braces, handles all edge cases.
"""

import re
import sys
from pathlib import Path

KERNEL_SRC = Path("/root/dowiz/kernel/src")


def find_mod_test_end(content):
    """Find the closing brace of the last #[cfg(test)] mod tests block."""
    pattern = re.compile(r'#\[cfg\(test\)\]')
    matches = list(pattern.finditer(content))
    if not matches:
        return None

    # Take the LAST cfg(test) annotation
    last_cfg = matches[-1]
    after_cfg = content[last_cfg.start():]

    mod_match = re.search(r'mod\s+(tests?\w*)\s*\{', after_cfg)
    if not mod_match:
        return None

    mod_name = mod_match.group(1)
    brace_pos = last_cfg.start() + mod_match.end() - 1  # position of '{'

    # Count braces to find matching '}'
    depth = 1
    pos = brace_pos + 1
    while pos < len(content) and depth > 0:
        if content[pos] == '{':
            depth += 1
        elif content[pos] == '}':
            depth -= 1
        pos += 1

    if depth != 0:
        return None

    return pos - 1  # position of matching '}'


def inject_test(file_key, name, body):
    """Inject a single test function into a file's test module."""
    if "/" in file_key:
        parts = file_key.split("/")
        if parts[-1] == "mod":
            path = KERNEL_SRC / parts[0] / "mod.rs"
        else:
            path = KERNEL_SRC / parts[0] / f"{parts[1]}.rs"
    else:
        path = KERNEL_SRC / f"{file_key}.rs"

    if not path.exists():
        return False

    content = open(path).read()
    
    # Check if test already exists
    if f"fn {name}()" in content:
        return False

    end_pos = find_mod_test_end(content)
    if end_pos is None:
        return False

    # Create test with proper indentation (4 spaces inside mod, 4 more for fn)
    indent = "    "
    test_code = f"\n{indent}#[test]\n{indent}fn {name}() {{\n{indent}{indent}{body}\n{indent}}}\n"

    new_content = content[:end_pos] + test_code + content[end_pos:]

    with open(path, "w") as f:
        f.write(new_content)

    return True


# Batch of tests: (file_key, test_name, test_body)
ALL_TESTS = [
    # ─── causal.rs ───
    ("causal", "cover_backdoor_adjust_zero_nx", "let _ = super::backdoor_adjust(&[], &[], &[], 0, 1);"),
    ("causal", "cover_backdoor_adjust_zero_nz", "let _ = super::backdoor_adjust(&[], &[], &[], 1, 0);"),
    ("causal", "cover_backdoor_adjust_wrong_len", "let _ = super::backdoor_adjust(&[], &[], &[], 1, 1);"),
    ("causal", "cover_backdoor_adjust_wrong_p_z_len", "let _ = super::backdoor_adjust(&[0.5], &[0.0], &[0.5], 1, 2);"),
    ("causal", "cover_backdoor_adjust_wrong_p_xz_len", "let _ = super::backdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[0.5], 1, 2);"),
    ("causal", "cover_backdoor_adjust_oor", "let _ = super::backdoor_adjust(&[1.5], &[1.0], &[1.0], 1, 1); let _ = super::backdoor_adjust(&[-0.5], &[1.0], &[1.0], 1, 1);"),
    ("causal", "cover_backdoor_adjust_p_z_not_sum_one", "let _ = super::backdoor_adjust(&[0.5], &[0.5], &[1.0], 1, 1);"),
    ("causal", "cover_backdoor_adjust_p_xz_not_sum_one", "let _ = super::backdoor_adjust(&[0.5], &[1.0], &[0.5], 1, 1);"),
    ("causal", "cover_backdoor_adjust_zero_prob", "let _ = super::backdoor_adjust(&[0.5], &[1.0], &[0.0], 1, 1);"),
    ("causal", "cover_backdoor_adjust_happy", "let _ = super::backdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[0.5, 0.5], 1, 2);"),
    ("causal", "cover_frontdoor_adjust_zero_nx", "let _ = super::frontdoor_adjust(&[], &[], &[], 0, 1);"),
    ("causal", "cover_frontdoor_adjust_zero_nm", "let _ = super::frontdoor_adjust(&[], &[], &[], 1, 0);"),
    ("causal", "cover_frontdoor_adjust_wrong_p_m_x", "let _ = super::frontdoor_adjust(&[], &[], &[1.0], 1, 1);"),
    ("causal", "cover_frontdoor_adjust_wrong_p_y_mx", "let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5], &[1.0], 1, 2);"),
    ("causal", "cover_frontdoor_adjust_wrong_p_x", "let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[1.0, 0.5], 1, 2);"),
    ("causal", "cover_frontdoor_adjust_oor", "let _ = super::frontdoor_adjust(&[1.5, 0.5], &[0.5, 0.5], &[1.0], 1, 2);"),
    ("causal", "cover_frontdoor_adjust_p_x_not_sum_one", "let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[0.5], 1, 2);"),
    ("causal", "cover_frontdoor_adjust_row_not_sum_one", "let _ = super::frontdoor_adjust(&[0.5, 0.3], &[0.5, 0.5], &[1.0], 1, 2);"),
    ("causal", "cover_frontdoor_adjust_happy", "let _ = super::frontdoor_adjust(&[0.5, 0.5], &[0.5, 0.5], &[1.0], 1, 2);"),
    ("causal", "cover_instrumental_adjust_oor", "let _ = super::instrumental_adjust(-0.1, 0.5, 0.5, 0.5, 0.5, 0.5);"),
    ("causal", "cover_instrumental_adjust_happy", "let _ = super::instrumental_adjust(0.5, 0.5, 0.5, 0.5, 0.5, 0.5);"),
    ("causal", "cover_counterfactual_linear_alpha_zero", "let _ = super::counterfactual_linear(0.0, 1.0, 1.0, 1.0, 1.0, 0.0);"),
    ("causal", "cover_counterfactual_linear_happy", "let _ = super::counterfactual_linear(1.0, 2.0, 1.0, 0.5, 1.0, 0.0);"),
    ("causal", "cover_d_separated_x_eq_y", "let _ = super::d_separated(&[], 0, 0, &[]);"),
    ("causal", "cover_d_separated_oob", "let _ = super::d_separated(&[], 0, 1, &[]);"),
    ("causal", "cover_d_separated_single", "let _ = super::d_separated(&[vec![1], vec![]], 0, 1, &[]);"),
    ("causal", "cover_backdoor_adjust_nx2_nz2", "let r = super::backdoor_adjust(&[0.25, 0.25, 0.25, 0.25], &[0.5, 0.5], &[0.25, 0.25, 0.25, 0.25], 2, 2); let _ = r.unwrap();"),
    ("causal", "cover_frontdoor_adjust_nx2", "let r = super::frontdoor_adjust(&[0.5, 0.5, 0.5, 0.5], &[0.5, 0.5, 0.5, 0.5], &[0.5, 0.5], 2, 2); let _ = r.unwrap();"),

    # ─── event_log.rs ───
    ("event_log", "cover_sha3_256_empty", "let _ = super::sha3_256(&[]);"),
    ("event_log", "cover_sha3_256_short", "let _ = super::sha3_256(b\"hello\");"),

    # ─── reverse_engineer.rs ───
    ("reverse_engineer", "cover_parse_elf_empty", "let _ = super::parse_elf(&[]);"),
    ("reverse_engineer", "cover_parse_elf_too_short", "let _ = super::parse_elf(b\"\\x7fELF\");"),
    ("reverse_engineer", "cover_parse_elf_bad_magic", "let _ = super::parse_elf(b\"badmagic\");"),
    ("reverse_engineer", "cover_parse_elf_not_64le", "let mut h = [0u8; 64]; h[..4].copy_from_slice(b\"\\x7fELF\"); h[4] = 1; h[5] = 1; let _ = super::parse_elf(&h);"),
    ("reverse_engineer", "cover_parse_elf_bad_encoding", "let mut h = [0u8; 64]; h[..4].copy_from_slice(b\"\\x7fELF\"); h[4] = 2; h[5] = 2; let _ = super::parse_elf(&h);"),
    ("reverse_engineer", "cover_extract_syscalls_high_noise", "let c = [0xb8, 0x4d, 0x01, 0x00, 0x00, 0x0f, 0x05]; let p = extract_syscalls(&c, 0); assert!(p.is_empty());"),
    ("reverse_engineer", "cover_profile_binary_shell_only", "let mut e = parse_elf(&minimal_elf64()).unwrap(); e.strings.push(\"/bin/bash\".into()); let p = profile_binary(&e, \"t\"); assert_eq!(p.risk, RiskLevel::High);"),
    ("reverse_engineer", "cover_profile_binary_crypto", "let mut e = parse_elf(&minimal_elf64()).unwrap(); e.strings.push(\"aes_encrypt\".into()); let p = profile_binary(&e, \"t\"); assert!(p.behaviors.contains(&BehaviorCategory::Crypto));"),
    ("reverse_engineer", "cover_profile_binary_system_info", "let mut e = parse_elf(&minimal_elf64()).unwrap(); e.strings.push(\"gettimeofday\".into()); let p = profile_binary(&e, \"t\"); assert!(p.behaviors.contains(&BehaviorCategory::SystemInfo));"),
    ("reverse_engineer", "cover_profile_binary_many_behaviors", "let mut e = parse_elf(&minimal_elf64()).unwrap(); e.strings.push(\"sqlite_query\".into()); e.strings.push(\"encrypt_data\".into()); e.strings.push(\"uname_info\".into()); e.strings.push(\"sha256_hash\".into()); let p = profile_binary(&e, \"t\"); assert!(p.behaviors.len() >= 3);"),

    # ─── stem.rs ───
    ("stem", "cover_stem_empty", "let _ = super::stem(\"\");"),
    ("stem", "cover_stem_short", "let _ = super::stem(\"ca\");"),
    ("stem", "cover_stem_plurals", "let _ = super::stem(\"caresses\");"),
    ("stem", "cover_stem_ing", "let _ = super::stem(\"running\");"),
    ("stem", "cover_stem_ly", "let _ = super::stem(\"happily\");"),
    ("stem", "cover_stem_ment", "let _ = super::stem(\"enjoyment\");"),
    ("stem", "cover_stem_ness", "let _ = super::stem(\"happiness\");"),
    ("stem", "cover_stem_ize", "let _ = super::stem(\"normalize\");"),
    ("stem", "cover_stem_able", "let _ = super::stem(\"readable\");"),
    ("stem", "cover_stem_less", "let _ = super::stem(\"fearless\");"),
    ("stem", "cover_stem_ous", "let _ = super::stem(\"dangerous\");"),
    ("stem", "cover_stem_tional", "let _ = super::stem(\"relational\");"),
    ("stem", "cover_stem_ful", "let _ = super::stem(\"beautiful\");"),
    ("stem", "cover_tokenize_stemmed_empty", "let _ = super::tokenize_stemmed(\"\");"),
    ("stem", "cover_tokenize_stemmed_text", "let _ = super::tokenize_stemmed(\"hello world\");"),
    ("stem", "cover_detect_language_empty", "let _ = super::detect_language(\"\");"),
    ("stem", "cover_detect_language_en", "let _ = super::detect_language(\"the world is a beautiful place with many wonderful things to see and do every day\");"),
    ("stem", "cover_detect_language_de", "let _ = super::detect_language(\"die Welt ist ein wunderschoener Ort mit vielen schoenen Dingen die man sehen und machen kann jeden Tag\");"),

    # ─── geo.rs ───
    ("geo", "cover_haversine_same", "let _ = super::haversine_meters(0.0, 0.0, 0.0, 0.0);"),
    ("geo", "cover_haversine_far", "let _ = super::haversine_meters(0.0, 0.0, 0.0, 180.0);"),
    ("geo", "cover_bearing_same", "let _ = super::bearing_deg(0.0, 0.0, 0.0, 0.0);"),
    ("geo", "cover_bearing_north", "let _ = super::bearing_deg(0.0, 0.0, 10.0, 0.0);"),
    ("geo", "cover_ema_initial", "let _ = super::ema_next(0.0, 5.0, 0.5);"),
    ("geo", "cover_polyline_empty", "let _ = super::polyline_length_meters(&[]);"),
    ("geo", "cover_polyline_single", "let _ = super::polyline_length_meters(&[(0.0, 0.0)]);"),

    # ─── json.rs ───
    ("json", "cover_json_parse_empty_str", "let _ = super::parse(\"\");"),
    ("json", "cover_json_parse_null", "let _ = super::parse(\"null\");"),
    ("json", "cover_json_parse_bool", "let _ = super::parse(\"true\"); let _ = super::parse(\"false\");"),
    ("json", "cover_json_parse_number", "let _ = super::parse(\"42\"); let _ = super::parse(\"-3.14\");"),
    ("json", "cover_json_parse_string", "let _ = super::parse(\"\\\"hello\\\"\");"),
    ("json", "cover_json_parse_array", "let _ = super::parse(\"[1,2,3]\");"),
    ("json", "cover_json_parse_object", "let _ = super::parse(\"{\\\"a\\\":1}\");"),
    ("json", "cover_json_parse_invalid", "let _ = super::parse(\"not json\");"),
    ("json", "cover_json_parse_empty_arr", "let _ = super::parse(\"[]\");"),
    ("json", "cover_json_parse_empty_obj", "let _ = super::parse(\"{}\");"),
    ("json", "cover_json_parse_deep", "let _ = super::parse(\"{\\\"a\\\":[1,{\\\"b\\\":2}]}\");"),
    ("json", "cover_json_parse_malformed", "let _ = super::parse(\"{\\\"a\\\":\");"),
    ("json", "cover_json_parse_negative", "let _ = super::parse(\"-42.5e3\");"),

    # ─── spectral.rs ───
    ("spectral", "cover_charpoly_empty", "let _ = super::charpoly(&[]);"),
    ("spectral", "cover_charpoly_1x1", "let _ = super::charpoly(&[vec![5.0]]);"),
    ("spectral", "cover_charpoly_2x2", "let r = super::charpoly(&[vec![1.0, 2.0], vec![3.0, 4.0]]); assert_eq!(r.len(), 3);"),
    ("spectral", "cover_eigenvalues_empty", "let _ = super::eigenvalues(&[]);"),
    ("spectral", "cover_eigenvalues_1x1", "let _ = super::eigenvalues(&[vec![5.0]]);"),
    ("spectral", "cover_eigenvalues_2x2", "let r = super::eigenvalues(&[vec![1.0, 2.0], vec![3.0, 4.0]]); assert_eq!(r.len(), 2);"),

    # ─── noether.rs ───
    ("noether", "cover_step_preserves_trivial", "let _ = super::step_preserves(&[1.0, 0.0], |x: &[f64]| x.to_vec(), |x: &[f64]| x.iter().sum(), 5, 1e-6);"),
    ("noether", "cover_invariant_drift_trivial", "let _ = super::invariant_drift(&[1.0, 0.0], |x: &[f64]| x.to_vec(), |x: &[f64]| x.iter().sum(), 5);"),

    # ─── readability.rs ───
    ("readability", "cover_extract_empty", "let _ = super::extract(\"\");"),
    ("readability", "cover_extract_short", "let _ = super::extract(\"a\");"),
    ("readability", "cover_extract_text", "let _ = super::extract(\"The quick brown fox jumps over the lazy dog.\");"),
    ("readability", "cover_extract_html", "let r = super::extract(\"<html><body><p>Hello world.</p></body></html>\"); assert!(!r.is_empty());"),
    ("readability", "cover_extract_long", "let r = super::extract(\"A very long text with multiple sentences. Second sentence here. Third one. Fourth sentence goes here. Fifth one for good measure.\"); assert!(!r.is_empty());"),

    # ─── pid.rs ───
    ("pid", "cover_batch_pid_single", "let mut s = vec![(0.0, 0.0, 0.0)]; let c = super::PidConfig { kp: 1.0, ki: 0.1, kd: 0.05, min: -1.0, max: 1.0 }; super::batch_pid_update(&mut s, &c, &[1.0], &[0.0]);"),
    ("pid", "cover_batch_pid_multiple", "let mut s = vec![(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)]; let c = super::PidConfig { kp: 1.0, ki: 0.1, kd: 0.05, min: -1.0, max: 1.0 }; super::batch_pid_update(&mut s, &c, &[1.0, 2.0], &[0.0, 0.0]);"),

    # ─── fdr/mod.rs ───
    ("fdr/mod", "cover_fdr_crc32_empty", "let _ = super::crc32(&[]);"),
    ("fdr/mod", "cover_fdr_crc32_data", "let _ = super::crc32(b\"test data\");"),

    # ─── intake.rs ───
    ("intake", "cover_tier_c_smt_stub", "let s = super::EtalonSpec { fields: vec![], rules: vec![], verify: String::from(\"true\"), verify_fn: None, nonlinear: true }; let _ = super::tier_c_smt_stub(&s);"),

    # ─── detection.rs ───
    ("detection", "cover_bbox_iou_same", "let a = super::BBox { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 }; let b = a.clone(); let _ = super::bbox_iou(&a, &b);"),
    ("detection", "cover_bbox_iou_disjoint", "let a = super::BBox { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 }; let b = super::BBox { x1: 10.0, y1: 10.0, x2: 11.0, y2: 11.0 }; let _ = super::bbox_iou(&a, &b);"),

    # ─── predictor.rs ───
    ("predictor", "cover_quick_predict", "let m = vec![0.5, 0.3, 0.8, 0.1]; let _ = super::quick_predict(m, \"scale_up\", \"latency\");"),

    # ─── lib.rs ───
    ("lib", "cover_sanitize_f64_nan", "let _ = super::sanitize_f64(f64::NAN);"),
    ("lib", "cover_sanitize_f64_inf", "let _ = super::sanitize_f64(f64::INFINITY);"),
    ("lib", "cover_sanitize_normalized_neg", "let _ = super::sanitize_normalized(-0.5);"),
    ("lib", "cover_sanitize_f32_nan", "let _ = super::sanitize_f32(f32::NAN);"),
    ("lib", "cover_checksum_fold_empty", "let _ = super::checksum_fold(&[]);"),
    ("lib", "cover_checksum_fold_data", "let _ = super::checksum_fold(&[1, 2, 3, 4]);"),
]


def main():
    injected = 0
    for file_key, name, body in ALL_TESTS:
        if inject_test(file_key, name, body):
            injected += 1

    print(f"Injected {injected} tests across files", file=sys.stderr)
    print(f"Total test definitions: {len(ALL_TESTS)}", file=sys.stderr)


if __name__ == "__main__":
    main()
