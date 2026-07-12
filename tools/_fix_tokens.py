#!/usr/bin/env python3
"""Bulk convert arbitrary token-bracket Tailwind classes to design-scale token
classes within packages/ui/src/components only.

Fix for greedy-prefix bug: property must be preceded by a token boundary
(start, whitespace, backtick, quote, '[', '(', ':', or '@') so every token in a
class string is matched independently (not just the last one in a chain).

Conversions preserve semantics:
  bg-[var(--brand-X)]          -> bg-brand-X
  text-[var(--brand-X)]        -> text-brand-X
  border-[var(--brand-X)]      -> border-brand-X
  ring-[var(--brand-X)]        -> ring-brand-X
  ring-offset-[var(--brand-X)] -> ring-offset-brand-X
  placeholder-[var(--brand-X)] -> placeholder-brand-X
  <p>[var(--color-success)]    -> <p>-semantic-success
  <p>[var(--color-on-*)]       -> <p>-white
  <p>[var(--status-XXX-bg)]    -> <p>-status-XXX/20
  <p>[var(--status-XXX-light)] -> <p>-status-XXX/10
  <p>[var(--status-XXX-border)]-> <p>-status-XXX/30
  <p>[var(--status-XXX)]       -> <p>-status-XXX
  shadow-[var(--elev- N)]      -> shadow-elevation-N   (also elevation-)
  rounded-[var(--brand-radius)]-> rounded-lg (variants: -sm->md, -btn->full, -xl/-2xl->2xl)
Variant prefixes (hover:, focus:, [@media...]:hover:, ...) and opacity (/NN) preserved.
"""
import re, os

ROOT = "/root/dowiz/packages/ui/src/components"

PROP = r"(bg|text|border|ring|ring-offset|placeholder)"
VAR_RE = re.compile(
    r"(?:^|(?<=[\s`'\":@]))"                 # token boundary (no consuming prefix)
    r"((?:[@\w()\-.:]+:)*)"                  # variant prefix e.g. hover: / [@media...]:
    r"(?:" + PROP + r")"                     # property
    r"-\[var\(--"                            # -[var(--
    r"(brand|color|status)-([\w-]+)"         # namespace + rest
    r"\)\]"                                  # )]
    r"(/\d+)?"                               # optional opacity
)
SHADOW_RE = re.compile(
    r"(?:^|(?<=[\s`'\":@]))((?:[@\w()\-.:]+:)*)shadow-\[var\(--elev(?:ation)?-(\d+)\)\]"
)
ROUNDED_RE = re.compile(
    r"(?:^|(?<=[\s`'\":@]))((?:[@\w()\-.:]+:)*)rounded(-[trbl])?-\[var\(--brand-radius"
    r"(-(?:sm|btn|xl|2xl))?(?:,[^)]*)?\)\]"
)


def map_ns(ns, rest, prop, opacity):
    if ns == "brand":
        return f"{prop}-brand-{rest}{opacity or ''}"
    if ns == "color":
        if rest in ("success", "warning", "danger", "info"):
            return f"{prop}-semantic-{rest}{opacity or ''}"
        if rest in ("on-success", "on-primary"):
            return f"{prop}-white{opacity or ''}"
        return f"{prop}-color-{rest}{opacity or ''}"
    if ns == "status":
        if rest.endswith("-bg"):
            return f"{prop}-status-{rest[:-3]}/20{opacity or ''}"
        if rest.endswith("-light"):
            return f"{prop}-status-{rest[:-6]}/10{opacity or ''}"
        if rest.endswith("-border"):
            return f"{prop}-status-{rest[:-7]}/30{opacity or ''}"
        return f"{prop}-status-{rest}{opacity or ''}"
    return None


def repl_var(m):
    variant = m.group(1) or ""
    prop = m.group(2)
    ns = m.group(3)
    rest = m.group(4)
    opacity = m.group(5) or ""
    return variant + map_ns(ns, rest, prop, opacity)


def repl_shadow(m):
    variant = m.group(1) or ""
    return variant + f"shadow-elevation-{m.group(2)}"


def repl_rounded(m):
    variant = m.group(1) or ""
    side = m.group(2) or ""
    suffix = m.group(3)
    if suffix is None:
        mapped = "lg"
    elif suffix == "-sm":
        mapped = "md"
    elif suffix == "-btn":
        mapped = "full"
    elif suffix in ("-xl", "-2xl"):
        mapped = "2xl"
    else:
        mapped = "lg"
    return variant + f"rounded{side}-{mapped}"


files = 0
for dirpath, _, names in os.walk(ROOT):
    for n in names:
        if not n.endswith((".tsx", ".ts")):
            continue
        p = os.path.join(dirpath, n)
        with open(p, "r", encoding="utf-8") as f:
            src = f.read()
        new = src
        # iterate to a fixpoint (a conversion may create new token boundaries)
        for _ in range(3):
            new2 = VAR_RE.sub(repl_var, new)
            new2 = SHADOW_RE.sub(repl_shadow, new2)
            new2 = ROUNDED_RE.sub(repl_rounded, new2)
            if new2 == new:
                break
            new = new2
        if new != src:
            with open(p, "w", encoding="utf-8") as f:
                f.write(new)
            files += 1

print(f"Updated {files} files (token-bracket conversion, v2).")
