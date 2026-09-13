#!/usr/bin/env python3
"""
glyphshow — glyph table renderer for bebop (T84 stage G4).

Four modes:
  --table (default): Full alphabet grouped by tier and category, display-width aligned
  --card: Compact cheat sheet of TIER A only, 80-column terminal format
  --tokens: Machine-readable, one glyph per line: <glyph>\t<ascii>
  --audit: Rendering audit for TIER A glyphs, with hazard detection
"""

import sys
import os
import csv
import unicodedata
import argparse

def display_width(s):
    """Calculate terminal display width of a string.

    East-Asian Wide (W) and Fullwidth (F) characters: width 2
    Combining characters: width 0
    Others: width 1
    """
    return sum(
        2 if unicodedata.east_asian_width(c) in 'WF'
        else 0 if unicodedata.combining(c)
        else 1
        for c in s
    )

def pad_right(s, width):
    """Pad string to exact display width with spaces."""
    current_width = display_width(s)
    if current_width >= width:
        return s
    return s + ' ' * (width - current_width)

def load_glyphs(table_path):
    """Load glyph table from TSV file."""
    rows = []
    try:
        with open(table_path) as f:
            reader = csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE)
            for row in reader:
                if row.get('tier'):  # Skip empty rows
                    rows.append(row)
    except FileNotFoundError:
        print(f"Error: glyph table not found at {table_path}", file=sys.stderr)
        sys.exit(1)
    return rows

def should_use_color():
    """Determine if color should be used."""
    if '--color' in sys.argv:
        return True
    if '--no-color' in sys.argv:
        return False
    return sys.stdout.isatty()

def get_color_codes(use_color):
    """Return ANSI color codes or empty strings."""
    if not use_color:
        return {
            'bold': '', 'dim': '', 'reset': '',
            'cyan': '', 'yellow': '', 'green': '', 'red': '', 'magenta': ''
        }
    return {
        'bold': '\033[1m',
        'dim': '\033[2m',
        'reset': '\033[0m',
        'cyan': '\033[36m',
        'yellow': '\033[33m',
        'green': '\033[32m',
        'red': '\033[31m',
        'magenta': '\033[35m'
    }

def get_tier_color(tier, colors):
    """Get color for a tier."""
    tier_colors = {
        'A': colors['green'],
        'I': colors['cyan'],
        'C': colors['magenta'],
        'B': colors['yellow'],
        'X': colors['red']
    }
    return tier_colors.get(tier, '')

def mode_table(rows, colors):
    """Output full alphabet table, grouped by tier and category."""
    tier_info = {
        'A': ('TIER A — LIVE GLYPHS', 'the closed set bebop.bin must accept after G2'),
        'I': ('TIER I — IDENTITY', 'glyph == ASCII; no glyph form is minted'),
        'C': ('TIER C — COMPOSITIONAL', 'not table entries; fall out of operator rewriting'),
        'B': ('TIER B — RESERVED', 'roadmap-committed, NOT in the language today'),
        'X': ('TIER X — OUT OF SCOPE / DEFECTIVE', 'no roadmap row, or a defect in v0.2')
    }

    print()
    print(colors['bold'] + '  BEBOP GLYPH SURFACE — ROADMAP T84, alphabet v1.0 (draft, frozen for this session)' + colors['reset'])
    print(colors['dim'] + '  glyphs are vector delta-outlines; the char below is a TERMINAL-RENDER PLACEHOLDER only' + colors['reset'])
    print()

    for tier in ['A', 'I', 'C', 'B', 'X']:
        sub = [x for x in rows if x['tier'] == tier]
        if not sub:
            continue

        title, note = tier_info[tier]
        tier_color = get_tier_color(tier, colors)

        print(tier_color + colors['bold'] + '  ' + title + colors['reset'] + colors['dim'] + '  (' + str(len(sub)) + ')  ' + note + colors['reset'])
        print(colors['dim'] + '  ' + '-' * 76 + colors['reset'])

        current_category = None
        for entry in sub:
            category = entry['category']
            if category != current_category:
                current_category = category
                print(colors['dim'] + '    [' + category + ']' + colors['reset'])

            glyph = entry['glyph']
            ascii_val = entry['ascii']
            note_text = entry.get('note', '') or ''

            # Pad glyph and ascii to consistent widths
            glyph_padded = pad_right(glyph, 4)
            ascii_padded = pad_right(ascii_val, 10)

            # Color the note based on content
            if 'COLLIDES' in note_text or 'BANNED' in note_text or 'dropped' in note_text:
                note_color = colors['red']
            elif 'NEW' in note_text or 'REASSIGNED' in note_text:
                note_color = colors['green']
            else:
                note_color = colors['dim']

            print('     ' + tier_color + colors['bold'] + glyph_padded + colors['reset'] +
                  ' ' + colors['bold'] + ascii_padded + colors['reset'] + ' ' +
                  note_color + note_text + colors['reset'])

        print()

    # Print summary
    tier_a_count = len([x for x in rows if x['tier'] == 'A'])
    tier_i_count = len([x for x in rows if x['tier'] == 'I'])
    tier_b_count = len([x for x in rows if x['tier'] == 'B'])
    tier_x_count = len([x for x in rows if x['tier'] == 'X'])

    print(colors['bold'] + '  TOTALS  ' + colors['reset'] +
          colors['green'] + 'live glyphs ' + str(tier_a_count) + colors['reset'] +
          colors['dim'] + ' + ' + colors['reset'] +
          colors['cyan'] + 'identity ' + str(tier_i_count) + colors['reset'] +
          colors['dim'] + '  =  ' + str(tier_a_count + tier_i_count) + ' tokens covered; ' + colors['reset'] +
          colors['yellow'] + str(tier_b_count) + ' reserved' + colors['reset'] +
          colors['dim'] + ', ' + colors['reset'] +
          colors['red'] + str(tier_x_count) + ' out of scope' + colors['reset'])
    print()

def mode_card(rows, colors):
    """Output compact TIER A cheat sheet in 80-column format."""
    tier_a = [x for x in rows if x['tier'] == 'A']

    # Format as fixed-width cells: glyph(2) + space(1) + ascii(8) + spacing(2) = 13 cols per cell
    # 5 cells per row = 65 columns (fits in 80)
    cells_per_row = 5

    print()
    print(colors['bold'] + 'BEBOP TIER A CHEAT SHEET' + colors['reset'])
    print(colors['dim'] + '(use while reading glyph source)' + colors['reset'])
    print()

    for i in range(0, len(tier_a), cells_per_row):
        row_entries = tier_a[i:i + cells_per_row]
        cells = []
        for entry in row_entries:
            glyph = entry['glyph']
            ascii_val = entry['ascii']
            # Build cell: glyph(padded to 2) + space + ascii(padded to 8)
            glyph_part = pad_right(glyph, 2)
            ascii_part = pad_right(ascii_val, 8)
            cell = colors['green'] + colors['bold'] + glyph_part + colors['reset'] + ' ' + colors['bold'] + ascii_part + colors['reset']
            cells.append(cell)
        # Join cells with two spaces between them
        print('  ' + '  '.join(cells))

    print()

def mode_tokens(rows, colors):
    """Output machine-readable tokens (no color, one per line)."""
    for entry in rows:
        glyph = entry['glyph']
        ascii_val = entry['ascii']
        print(f"{glyph}\t{ascii_val}")

def mode_audit(rows, colors):
    """Audit TIER A glyphs for rendering hazards."""
    tier_a = [x for x in rows if x['tier'] == 'A']

    print()
    print(colors['bold'] + 'GLYPH AUDIT — TIER A' + colors['reset'])
    print()

    unnamed_count = 0
    wide_count = 0
    ambiguous_count = 0

    for entry in tier_a:
        glyph = entry['glyph']
        ascii_val = entry['ascii']

        # Check display width
        width = display_width(glyph)

        # Check East-Asian width property
        ea_width = unicodedata.east_asian_width(glyph)

        # Try to get Unicode name
        try:
            uni_name = unicodedata.name(glyph)
            name_status = ''
        except ValueError:
            uni_name = '(unnamed/private-use)'
            name_status = ' ' + colors['red'] + colors['bold'] + '✗ unnamed' + colors['reset']
            unnamed_count += 1

        # Flag wide glyphs (W/F in East-Asian width property)
        wide_status = ''
        if ea_width in 'WF':
            wide_status = ' ' + colors['red'] + colors['bold'] + '✗ wide (2 cols)' + colors['reset']
            wide_count += 1

        # Flag ambiguous glyphs (A in East-Asian width property)
        ambiguous_status = ''
        if ea_width == 'A':
            ambiguous_status = ' ' + colors['red'] + colors['bold'] + '✗ ambiguous (1 or 2 cols in different locales)' + colors['reset']
            ambiguous_count += 1

        # Format width display: show 'A' for ambiguous, else show the numeric width
        width_display = 'A' if ea_width == 'A' else str(width)

        codepoint = f"U+{ord(glyph):04X}"
        print(f"  {colors['green']}{colors['bold']}{glyph}{colors['reset']} {codepoint} {uni_name} width={width_display}{name_status}{wide_status}{ambiguous_status}")

    print()
    print(f"{colors['bold']}glyph_audit{colors['reset']} tier_a={len(tier_a)} wide={wide_count} ambiguous={ambiguous_count} unnamed={unnamed_count}")
    print()

def find_table_path():
    """Find the glyph table, with fallback."""
    # Try primary path first (relative to cwd, assumes running from g4show)
    primary = 'docs/design/GLYPHS-v1.tsv'
    if os.path.exists(primary):
        return primary

    # Try fallback path
    fallback = '/root/dowiz/.claude/lanes/glyph-kit/GLYPHS-v1.tsv'
    if os.path.exists(fallback):
        return fallback

    # Return primary anyway; load_glyphs will error with clear message
    return primary

def main():
    parser = argparse.ArgumentParser(
        description='Glyph table renderer for bebop (T84 stage G4)',
        add_help=True
    )
    parser.add_argument(
        'table_path',
        nargs='?',
        default=None,
        help='Path to glyph table TSV file (default: docs/design/GLYPHS-v1.tsv or fallback)'
    )
    parser.add_argument(
        '--table',
        action='store_true',
        help='Output full table grouped by tier (default)'
    )
    parser.add_argument(
        '--card',
        action='store_true',
        help='Output compact TIER A cheat sheet'
    )
    parser.add_argument(
        '--tokens',
        action='store_true',
        help='Output machine-readable tokens (glyph\\tascii)'
    )
    parser.add_argument(
        '--audit',
        action='store_true',
        help='Output rendering audit for TIER A glyphs'
    )
    parser.add_argument(
        '--color',
        action='store_true',
        help='Force color output'
    )
    parser.add_argument(
        '--no-color',
        action='store_true',
        help='Disable color output'
    )

    args = parser.parse_args()

    # Determine table path
    if args.table_path:
        table_path = args.table_path
    else:
        table_path = find_table_path()

    # Load glyphs
    rows = load_glyphs(table_path)

    # Determine color usage
    use_color = should_use_color()
    colors = get_color_codes(use_color)

    # Determine which mode to run
    # Default to --table if no mode specified
    if not (args.table or args.card or args.tokens or args.audit):
        args.table = True

    # Run the selected mode(s)
    if args.table:
        mode_table(rows, colors)
    elif args.card:
        mode_card(rows, colors)
    elif args.tokens:
        mode_tokens(rows, colors)
    elif args.audit:
        mode_audit(rows, colors)

if __name__ == '__main__':
    main()
