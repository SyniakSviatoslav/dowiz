#!/usr/bin/env python3
"""
Glyph table validator for bebop-lang T84 stage G1.
Implements 5 checks on the GLYPHS-v1.tsv table.
"""

import csv
import sys

def is_ascii(s):
    """Check if string contains only ASCII characters."""
    try:
        s.encode('ascii')
        return True
    except UnicodeEncodeError:
        return False

def load_table(filepath):
    """Load TSV using csv.DictReader with QUOTE_NONE."""
    rows = []
    with open(filepath, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE)
        for row in reader:
            if row:  # Skip empty rows
                rows.append(row)
    return rows

def check_bijective_glyph(rows):
    """Within tier A, no glyph appears twice."""
    tier_a_glyphs = {}
    for row in rows:
        if row.get('tier') == 'A':
            glyph = row.get('glyph', '')
            if glyph in tier_a_glyphs:
                return False, f"Duplicate glyph '{glyph}' in tier A"
            tier_a_glyphs[glyph] = row
    return True, "OK"

def check_bijective_ascii(rows):
    """Within tiers A and I combined, no ASCII name appears twice."""
    ascii_names = {}
    for row in rows:
        tier = row.get('tier')
        if tier in ('A', 'I'):
            ascii_name = row.get('ascii', '')
            if ascii_name in ascii_names:
                return False, f"Duplicate ascii name '{ascii_name}' in tiers A/I"
            ascii_names[ascii_name] = row
    return True, "OK"

def check_tier_a_nonascii(rows):
    """Every tier-A glyph contains at least one non-ASCII codepoint."""
    for row in rows:
        if row.get('tier') == 'A':
            glyph = row.get('glyph', '')
            if is_ascii(glyph):
                return False, f"Tier-A glyph '{glyph}' is pure ASCII (should be non-ASCII)"
    return True, "OK"

def check_tier_i_ascii(rows):
    """Every tier-I glyph is pure ASCII and equals its own ascii column."""
    for row in rows:
        if row.get('tier') == 'I':
            glyph = row.get('glyph', '')
            ascii_name = row.get('ascii', '')
            if not is_ascii(glyph):
                return False, f"Tier-I glyph '{glyph}' is not pure ASCII"
            if glyph != ascii_name:
                return False, f"Tier-I row: glyph '{glyph}' != ascii '{ascii_name}'"
    return True, "OK"

def check_closure(rows):
    """Every token the compiler accepts appears in tier A or I.

    Expected tokens (hardcoded, 54 strings):
    fn module struct enum use contract test kernel let in if then else while
    break return match i64 str == != <= >= && || ^ << >> >>> * / % ! -> =>
    < > + - & | = : ; , . ( ) { } [ ] " _
    """
    expected_tokens = {
        'fn', 'module', 'struct', 'enum', 'use', 'contract', 'test', 'kernel',
        'let', 'in', 'if', 'then', 'else', 'while', 'break', 'return', 'match',
        'i64', 'str', '==', '!=', '<=', '>=', '&&', '||', '^', '<<', '>>',
        '>>>', '*', '/', '%', '!', '->', '=>', '<', '>', '+', '-', '&', '|',
        '=', ':', ';', ',', '.', '(', ')', '{', '}', '[', ']', '"', '_'
    }

    # Collect ASCII names from tiers A and I
    table_ascii_names = set()
    for row in rows:
        tier = row.get('tier')
        if tier in ('A', 'I'):
            ascii_name = row.get('ascii', '')
            if ascii_name:
                table_ascii_names.add(ascii_name)

    # Check for missing tokens
    missing = expected_tokens - table_ascii_names

    # Check for unexpected entries
    unexpected = table_ascii_names - expected_tokens

    if missing or unexpected:
        details = []
        if missing:
            details.append(f"Missing from table: {sorted(missing)}")
        if unexpected:
            details.append(f"Unexpected in table: {sorted(unexpected)}")
        return False, "; ".join(details)

    return True, "OK"


def check_global_glyph_unique(rows):
    """No glyph may appear on two rows ANYWHERE in the table, across all tiers.

    Global uniqueness, not just within tier A: a glyph that means one thing today
    and a reserved thing tomorrow cannot be read unambiguously, and the day the
    reserved row goes live the surface silently becomes non-bijective.
    """
    seen = {}
    dups = []
    for row in rows:
        g = row.get('glyph', '')
        if not g:
            continue
        if g in seen:
            dups.append(f"{g!r} on {seen[g]} and {row.get('tier')}/{row.get('ascii')}")
        else:
            seen[g] = f"{row.get('tier')}/{row.get('ascii')}"
    if dups:
        return False, "Duplicate glyph across tiers: " + "; ".join(dups)
    return True, "OK"


def check_global_ascii_unique(rows):
    """No ASCII name may appear on two rows ANYWHERE in the table, across all tiers.

    Two glyphs sharing one ASCII name makes the glyph->ASCII projection lossy:
    the reverse map cannot know which glyph to restore.
    """
    seen = {}
    dups = []
    for row in rows:
        a = row.get('ascii', '')
        if not a:
            continue
        if a in seen:
            dups.append(f"{a!r} on {seen[a]} and {row.get('tier')}/{row.get('glyph')}")
        else:
            seen[a] = f"{row.get('tier')}/{row.get('glyph')}"
    if dups:
        return False, "Duplicate ascii name across tiers: " + "; ".join(dups)
    return True, "OK"


def main(filepath):
    """Run all checks and report results."""
    rows = load_table(filepath)

    checks = [
        ('bijective_glyph', check_bijective_glyph),
        ('bijective_ascii', check_bijective_ascii),
        ('tier_a_nonascii', check_tier_a_nonascii),
        ('tier_i_ascii', check_tier_i_ascii),
        ('closure', check_closure),
        ('global_glyph_unique', check_global_glyph_unique),
        ('global_ascii_unique', check_global_ascii_unique),
    ]

    results = []
    for check_name, check_func in checks:
        passed, detail = check_func(rows)
        status = 'PASS' if passed else 'FAIL'
        print(f"CHECK {check_name} {status} {detail}")
        results.append((check_name, passed))

    # Count results
    passed_count = sum(1 for _, passed in results if passed)
    failed_count = sum(1 for _, passed in results if not passed)

    print(f"glyph_check: {passed_count} PASS {failed_count} FAIL")

    # Exit with appropriate code
    return 0 if failed_count == 0 else 1

if __name__ == '__main__':
    if len(sys.argv) != 2:
        print("Usage: glyph_check.py <tsv_file>", file=sys.stderr)
        sys.exit(1)

    sys.exit(main(sys.argv[1]))
