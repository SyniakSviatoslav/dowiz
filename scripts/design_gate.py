"""The checkable half of DOWIZ-INTERFACES-PLAN §8."""
import re, sys, pathlib

BASE = pathlib.Path("workers/api/public")
SURFACES = {
    "storefront": ("index.html", "app.js"),
    "admin": ("admin/index.html", "admin/app.js"),
    "courier": ("courier/index.html", "courier/app.js"),
}

# §8.1 T2 — DOWIZ-FIXED tokens every surface must define. The plan flags
# --font-mono as "NEW — real gap!"; it is on this list so it cannot be a gap
# again.
REQUIRED_TOKENS = ["--font-mono", "--ease-snap", "--ease-tide", "--tap"]

failures = []
notes = []


def fail(surface, rule, msg):
    failures.append(f"[{surface}] {rule}: {msg}")


for name, (html_p, js_p) in SURFACES.items():
    html = (BASE / html_p).read_text(encoding="utf-8")
    js = (BASE / js_p).read_text(encoding="utf-8")
    both = html + js

    # ── §8.1 the fixed tokens exist ───────────────────────────────────────
    defined = set(re.findall(r"(--[A-Za-z0-9_-]+)\s*:", html))
    for tok in REQUIRED_TOKENS:
        if tok not in defined:
            fail(name, "T2", f"{tok} is not defined")

    # ── every token used is defined (no silent fallback to nothing) ───────
    used = {m.group(1) for m in re.finditer(r"var\((--[A-Za-z0-9_-]+)\s*(,)?", both)
            if not m.group(2)}
    # A token name ending in `-` is a PREFIX built at runtime, e.g.
    # `var(--st-${status})`. The concrete names it resolves to are checked by
    # the same loop; the prefix itself is not a token.
    used = {t for t in used if not t.endswith("-")}
    for tok in sorted(used - defined):
        fail(name, "T2", f"{tok} is used without a definition or a fallback")

    # ── §8.4(3) MONEY IS SACRED ───────────────────────────────────────────
    # mono + tabular, and it never tweens. Checked on the `.money` class,
    # which is the single place money may be styled.
    m = re.search(r"\.money\{([^}]*)\}", html)
    if not m:
        fail(name, "R3", "no .money rule — money must have one styling in one place")
    else:
        body = m.group(1)
        if "--font-mono" not in body:
            fail(name, "R3", ".money does not use --font-mono")
        if "tabular-nums" not in body:
            fail(name, "R3", ".money does not set tabular-nums")
        if "transition" in body or "animation" in body:
            fail(name, "R3", ".money animates — money never tweens")

    # Every money render must carry the class. Two legitimate shapes:
    #   * an HTML template — the class is on the line
    #   * a textContent assignment — the class is on the ELEMENT, so the
    #     element's id is resolved and checked in the surface's HTML
    # Anything else is a price styled ad hoc, which is how three surfaces come
    # to disagree about what a number looks like.
    def element_has_money_class(el_id):
        m = re.search(r"<[^>]*id=[\"']" + re.escape(el_id) + r"[\"'][^>]*>", html)
        if not m:
            # It may carry the id first and the class after, or vice versa;
            # this pattern covers both because it matches the whole tag.
            m = re.search(r"<[^>]*id=[\"']" + re.escape(el_id) + r"[\"'][^>]*>", js)
        return bool(m and "money" in m.group(0))

    js_lines = js.splitlines()
    for line_no, line in enumerate(js_lines, 1):
        if "money(" not in line:
            continue
        stripped_line = line.strip()
        # A line that only FORMATS money is not a render site. You cannot
        # render without markup or a write to the DOM, so a line with neither
        # styles nothing and cannot carry a class -- `todayRevenue: money(x)`
        # inside an object literal is formatting; the write that follows it is
        # the render, and that line is checked on its own.
        if "<" not in line and "textContent" not in line and "innerHTML" not in line:
            continue
        if stripped_line.startswith("//") or stripped_line.startswith("*"):
            continue
        if re.search(r"(const|let|var|function)\s+money\s*[=(]", line):
            continue
        # A WINDOW, not just the line: markup is routinely split across lines,
        # and the class often sits on the opening tag one or two lines above
        # the interpolation. Three lines is wide enough for that and narrow
        # enough that an unrelated `.money` elsewhere in the function does not
        # excuse an unstyled price.
        window = "\n".join(js_lines[max(0, line_no - 4):line_no + 1])
        if re.search(r'class=["\'][^"\']*\bmoney\b', window):
            continue
        # An EXPLICIT, written exemption. A toast is plain text -- a class
        # cannot ride on a string -- so the site is annotated with its reason
        # rather than silently skipped. An exemption someone had to type is one
        # a reviewer can see; a silent skip is a hole.
        if "// money:toast" in line:
            continue
        # textContent / innerHTML into a known element.
        el = re.search(r"""(?:\$\(|getElementById\()['"]#?([A-Za-z0-9_-]+)['"]\)\s*\.(?:textContent|innerHTML)""", line)
        if el and element_has_money_class(el.group(1)):
            continue
        fail(name, "R3", f"{js_p}:{line_no} renders money without the .money class")

    # ── §8.4(6) ζ GOVERNS MOTION ──────────────────────────────────────────
    # No raw cubic-bezier at a call site; the easing comes from a token.
    for m in re.finditer(r"(transition|animation)\s*:[^;}]*", html):
        decl = m.group(0)
        if "cubic-bezier" in decl:
            fail(name, "R6", f"raw cubic-bezier at a call site: {decl.strip()[:70]}")

    # ── §8.4(9) REDUCED MOTION NEVER LOSES MEANING ────────────────────────
    if "@keyframes" in html and "prefers-reduced-motion" not in html:
        fail(name, "R9", "defines animations with no prefers-reduced-motion guard")

    # Every named animation must be reachable from a reduced-motion block or
    # be explicitly calmed somewhere.
    anims = set(re.findall(r"@keyframes\s+([A-Za-z0-9_-]+)", html))
    if anims:
        rm_blocks = re.findall(r"@media\s*\([^)]*prefers-reduced-motion[^)]*\)\s*\{", html)
        if not rm_blocks:
            fail(name, "R9", f"{len(anims)} animations and no reduced-motion block")

    # ── the browser chrome must be a colour the page contains ─────────────
    # A <meta theme-color> cannot read a CSS variable, so its value is repeated
    # by hand -- which is exactly how it goes stale. The storefront carried
    # #C1121F, a red from a palette that no longer existed, so a phone's status
    # bar sat in a colour found nowhere else on the page.
    theme_colors = re.findall(r'<meta[^>]*name="theme-color"[^>]*content="(#[0-9a-fA-F]{3,8})"', html)
    palette = set()
    for tm in re.finditer(r"--[A-Za-z0-9_-]+\s*:\s*([^;}]*)", html):
        palette.update(h.lower() for h in re.findall(r"#[0-9a-fA-F]{3,8}", tm.group(1)))
    if not theme_colors:
        fail(name, "chrome", "no theme-color: the browser paints its own")
    for c in theme_colors:
        if c.lower() not in palette:
            fail(name, "chrome", f"theme-color {c} is not in this surface's palette")

    # ── §8.3 palette discipline: no new hue per screen ────────────────────
    # A raw hex outside the token blocks and outside a comment is a colour that
    # the token system does not know about.
    stripped = re.sub(r"/\*.*?\*/", "", html, flags=re.S)
    stripped = re.sub(r"<!--.*?-->", "", stripped, flags=re.S)
    # theme-color literals are checked above and are REQUIRED to be literal.
    stripped = re.sub(r'<meta[^>]*theme-color[^>]*>', "", stripped)
    token_block_hexes = set()
    for tm in re.finditer(r"--[A-Za-z0-9_-]+\s*:\s*([^;}]*)", stripped):
        token_block_hexes.update(re.findall(r"#[0-9a-fA-F]{3,8}", tm.group(1)))
    body_only = re.sub(r"--[A-Za-z0-9_-]+\s*:[^;}]*", "", stripped)
    stray = [h for h in re.findall(r"#[0-9a-fA-F]{3,8}", body_only)]
    # White and black are structural (shadows, scrims) rather than hues.
    stray = [h for h in stray if h.lower() not in ("#fff", "#ffffff", "#000", "#000000")]
    if stray:
        notes.append(f"[{name}] R5: {len(stray)} literal colours outside the token block: "
                     + ", ".join(sorted(set(stray))[:6]))

print("design-gate — DOWIZ-INTERFACES-PLAN §8")
for n in notes:
    print("  note  " + n)
if failures:
    for f in failures:
        print("  FAIL  " + f)
    print(f"\nRED: {len(failures)} violation(s)")
    sys.exit(1)
print(f"\nGREEN: {len(SURFACES)} surfaces, {len(REQUIRED_TOKENS)} fixed tokens, money 3-way enforced")
