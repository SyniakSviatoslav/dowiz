"""The checkable half of DOWIZ-INTERFACES-PLAN §8."""
import re, sys, pathlib

BASE = pathlib.Path("workers/api/public")
SURFACES = {
    "storefront": ("store/index.html", "app.js"),
    "admin": ("admin/index.html", "admin/app.js"),
    "courier": ("courier/index.html", "courier/app.js"),
    # The main hub. A fourth dowiz-owned surface, on the gate from its first
    # commit rather than added after it had already drifted -- which is the
    # order the other three learned this in.
    "platform": ("platform/index.html", "platform/app.js"),
}

# §8.1 T2 — DOWIZ-FIXED tokens every surface must resolve. The plan flags
# --font-mono as "NEW — real gap!"; it is on this list so it cannot be a gap
# again.
REQUIRED_TOKENS = ["--font-mono", "--ease-snap", "--ease-tide", "--tap"]

# The shared layer. T2 lives here ONCE and a surface may not redefine it: the
# three surfaces each held their own copy until seventeen tokens had drifted
# apart, including --font-mono, which meant the same price was set in a
# different typeface on the console than on the storefront.
SHARED_CSS = BASE / "lib" / "tokens.css"
shared_text = SHARED_CSS.read_text(encoding="utf-8")
SHARED_TOKENS = set(re.findall(r"(--[A-Za-z0-9_-]+)\s*:", shared_text))

# The shared COMPONENT layer. Same argument as the tokens one directory up: a
# rule written out once per surface is a rule fixed in three places and
# forgotten in the fourth. Only rules that were byte-identical in every copy
# live here; see the file's own header for what is deliberately left out.
SHARED_COMPONENTS = BASE / "lib" / "components.css"
components_text = SHARED_COMPONENTS.read_text(encoding="utf-8")
# Strip comments before reading selectors, or a class named in prose counts.
_components_css = re.sub(r"/\*.*?\*/", "", components_text, flags=re.S)
SHARED_RULES = {
    " ".join(sel.split())
    for sel in re.findall(r"([^{}@]+)\{", _components_css)
}
SHARED_CLASSES = set(re.findall(r"\.([a-zA-Z][\w-]*)", " ".join(SHARED_RULES)))

failures = []
notes = []


def fail(surface, rule, msg):
    failures.append(f"[{surface}] {rule}: {msg}")


for name, (html_p, js_p) in SURFACES.items():
    html = (BASE / html_p).read_text(encoding="utf-8")
    js = (BASE / js_p).read_text(encoding="utf-8")
    both = html + js
    # THE SURFACE'S OWN STYLESHEETS, AS LINKED. Since 2026-09-16 every surface's
    # styles live in a file (`style-src 'self'` dropped the <style> element),
    # so a rule is no longer in the HTML; a gate that reads only the HTML
    # reported every class as unstyled -- twenty on the storefront -- and the
    # true count of missing rules was invisible in the noise. Shared /lib
    # sheets are handled separately above; the rest are read here.
    css_own = ""
    for href in re.findall(r'<link[^>]*rel="stylesheet"[^>]*href="(/[^"]+\.css)"', html):
        if href.startswith("/lib/"):
            continue
        sheet_path = BASE / href.lstrip("/")
        if sheet_path.exists():
            css_own += "\n" + re.sub(r"/\*.*?\*/", "", sheet_path.read_text(encoding="utf-8"), flags=re.S)

    # ── §8.1 the fixed tokens exist ───────────────────────────────────────
    # A surface resolves a token if it declares it OR the shared layer does.
    own = set(re.findall(r"(--[A-Za-z0-9_-]+)\s*:", html))
    defined = own | SHARED_TOKENS

    # ── T2 IS NOT A SURFACE'S TO REDEFINE ─────────────────────────────────
    # This is the rule that would have caught the drift. A surface that wants a
    # different spacing scale or a different monospace stack is a surface that
    # has stopped sharing a design system with the other two.
    for tok in sorted(own & SHARED_TOKENS):
        fail(name, "T2", f"{tok} is fixed by lib/tokens.css and redefined here")

    # And the shared layer has to actually be loaded, or the tokens resolve to
    # nothing and every measurement on the page silently becomes zero.
    if 'href="/lib/tokens.css"' not in html:
        fail(name, "T2", "lib/tokens.css is not linked")
    if 'href="/lib/components.css"' not in html:
        fail(name, "T5", "lib/components.css is not linked")

    # ── A SHARED RULE IS NOT A SURFACE'S TO REDEFINE ──────────────────────
    # The token rule (T2) one block up, applied to components. A surface that
    # re-declares `.money` or `.sr` has stopped sharing a component layer with
    # the other three, and the drift is invisible until two screens disagree.
    own_rules = set()
    for sty in re.findall(r"<style>(.*?)</style>", html, re.S):
        sty_nc = re.sub(r"/\*.*?\*/", "", sty, flags=re.S)
        depth = 0
        buf = ""
        for ch in sty_nc:
            if ch == "{":
                if depth == 0:
                    own_rules.add(" ".join(buf.split()))
                    buf = ""
                depth += 1
            elif ch == "}":
                depth = max(0, depth - 1)
                if depth == 0:
                    buf = ""
            elif depth == 0:
                buf += ch
    for sel in sorted(own_rules & SHARED_RULES):
        if sel:
            fail(name, "T5", f"'{sel}' is fixed by lib/components.css and redefined here")
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
    # THE SHARED SHEET IS CONSULTED FIRST. A surface may legitimately carry
    # something like `input.money{font-size:...}`, and a substring search over
    # the surface would match THAT and then report the canonical rule missing
    # font-mono. The rule that governs money lives in one place; look there.
    m = re.search(r"(?:^|[\s,}])\.money\{([^}]*)\}", components_text) \
        or re.search(r"(?:^|[\s,}])\.money\{([^}]*)\}", html)
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

    # ── MONEY IS FORMATTED IN ONE PLACE, FOR EVERY SURFACE ────────────────
    #
    # R3 above says money has ONE STYLING in one place. This says it has one
    # IMPLEMENTATION. It did not: the storefront formatted with the customer's
    # locale while the console and the courier app each carried their own
    # `Intl.NumberFormat` with `currency:'ALL'` hardcoded, so a venue trading in
    # anything else showed lek to its owner and to its couriers and the right
    # currency to its customers. Three copies of a rule is three chances to
    # disagree, and money is where a disagreement is a support call.
    if "money(" in js or "class=\"money\"" in html:
        if "/lib/money.js" not in js:
            fail(name, "R3b", "renders money without importing /lib/money.js")
        # A surface may still build a currency string for something that is not
        # money (a rate, a label). What it may not do is re-implement the
        # formatter, which is what `style:'currency'` outside the module means.
        for m in re.finditer(r"style\s*:\s*['\"]currency['\"]", js):
            line_no = js[: m.start()].count("\n") + 1
            fail(name, "R3b", f"{js_p}:{line_no} formats currency itself; use /lib/money.js")

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

    # ── §8.3 every class the markup uses must have a rule ─────────────────
    # A class with no rule is markup that looks styled and is not. This was
    # being checked by hand after every change, which is exactly the kind of
    # thing that stops happening.
    #
    # `ti-*` are the icon font's own and are defined in its stylesheet, not
    # here; a class built by interpolation is skipped because its concrete
    # values cannot be read statically.
    used = set()
    for m in re.finditer(r"""class=["'`]([^"'`$\\]+)""", js):
        used |= set(m.group(1).split())
    # A SUBSTRING TEST IS NOT A SELECTOR TEST. `".hint" in html` is true when the
    # sheet only defines `.hint2`, so a class with no rule of its own passed for
    # as long as some longer class shared its prefix -- which is how `hint`
    # reached the courier surface unstyled while the gate reported GREEN. The
    # match now has to end where a CSS identifier ends.
    for cls in sorted(used):
        # A class that ends in a hyphen is a PREFIX the script completes at
        # run time (`st-` + status); its concrete values cannot be read here.
        if cls.startswith("ti") or not cls or cls.endswith("-"):
            continue
        sel = re.compile(r"\." + re.escape(cls) + r"(?![A-Za-z0-9_-])")
        # The shared component layer counts as a rule the surface has: it is
        # linked by every surface, so a class defined there IS styled here.
        if not sel.search(html) and not sel.search(js) and not sel.search(css_own) and cls not in SHARED_CLASSES:
            fail(name, "R5", f'class "{cls}" is used and has no rule')

    # ── NOTHING THIRD-PARTY MAY BLOCK THE FIRST PAINT ─────────────────────
    #
    # Measured before this rule existed: 982 KB of icon font from jsdelivr and
    # 200 KB of typeface from Google, all render-blocking, on a storefront
    # whose own critical path is 40 KB. Two CDNs a venue does not control, and
    # every customer's IP reaching both -- on a product whose first invariant
    # is local-first.
    #
    # `preconnect` to a host nothing then loads is also caught: it is a DNS and
    # TLS handshake bought for nothing.
    for m in re.finditer(r'<link[^>]*href="(https?://[^"]+)"[^>]*>', html):
        tag, url = m.group(0), m.group(1)
        host = url.split("/")[2]
        if 'rel="preconnect"' in tag:
            # A preconnect is a handshake, not a payload. It is legitimate when
            # the page really does fetch from that host later -- and waste when
            # it does not, which is the case this catches.
            if host not in js:
                fail(name, "origin", f"preconnects to {host}, which nothing loads")
        else:
            fail(name, "origin", f"loads {host} before the page can paint")

    # ── the browser chrome must be a colour the page contains ─────────────
    # A <meta theme-color> cannot read a CSS variable, so its value is repeated
    # by hand -- which is exactly how it goes stale. The storefront carried
    # #C1121F, a red from a palette that no longer existed, so a phone's status
    # bar sat in a colour found nowhere else on the page.
    theme_colors = re.findall(r'<meta[^>]*name="theme-color"[^>]*content="(#[0-9a-fA-F]{3,8})"', html)
    palette = set()
    # The palette lives in the surface's own stylesheet now, not in the HTML.
    for tm in re.finditer(r"--[A-Za-z0-9_-]+\s*:\s*([^;}]*)", html + css_own):
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
