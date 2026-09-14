#!/usr/bin/env python3
"""axiom_i64.py -- the wrapping-i64 ground model every axiom refuter evaluates in,
plus the BOUNDARY-FIRST point generator that decides what the refuters visit.

WHY THIS FILE EXISTS SEPARATELY. Two of the seven axioms in
formal/Bebop/Theorems.lean were FALSE, and each fell at exactly one point:
`isqrt_correct` at s = -1 and `cursor_monotone` at len = -10. A random sweep
(`fp_mul_sweep`, Theorems.lean, 1024 LCG pairs) visits neither with any useful
probability: -1 is one value out of 2^64. A sweep that enumerates the BOUNDARY
first finds both in its first few hundred points. So the ordering of the point
generator is not a convenience, it is the whole mechanism, and it lives in one
place so that no refuter can quietly drop it.

LANGUAGE FACTS THIS MODEL ENCODES, each with the tree's own measurement:
  * Every value is a WRAPPING i64 (Z/2^64, read signed).
  * `/` TRUNCATES toward zero, NOT floors. Quoted from selfhost/tcheck.bp:2101
    ("MEASURED PREREQUISITE (tmp/divprobe.bp, this box, today)"):
        7/3=2 r=1, -7/3=-2 r=-1, -1/3=0 r=-1, 7/-3=-2 r=1
    `self_test()` below asserts all four against this model.
  * `>>` is a LOGICAL shift; `>>>` is arithmetic (docs/LANGUAGE.md:64, quoted in
    ROADMAP.md:206). Quoted measurements: (-16) >> 4 = 1152921504606846975 and
    MIN >> 32 = 2147483648. `self_test()` asserts both.
  * `&&` / `||` are LOGICAL in value but do NOT short-circuit, and in .bp source
    `&&` binds tighter than comparison and is effectively a constant zero -- which
    is why tcheck.bp writes conjunctions as `(if p then 1 else 0) * (if q ...)`.
    No refuter here transcribes a `&&`.
  * There are no floats. Q32 fixed point is i64 with a scaling convention, so an
    axiom "about fp" is an axiom about i64.

Nothing in this file reads a repo file or runs a subprocess; it is pure arithmetic
so that a refuter's failure can never be a plumbing failure.
"""

BITS = 64
W = 1 << BITS
MIN = -(1 << (BITS - 1))
MAX = (1 << (BITS - 1)) - 1
UMASK = W - 1


def w(x):
    """Wrap an arbitrary Python int into signed i64 (Z/2^64, read signed)."""
    x &= UMASK
    return x - W if x >= (1 << (BITS - 1)) else x


def u(x):
    """The unsigned reading of an i64 bit pattern."""
    return x & UMASK


def add(a, b): return w(a + b)
def sub(a, b): return w(a - b)
def mul(a, b): return w(a * b)
def neg(a): return w(-a)


def tdiv(a, b):
    """Bebop `/`: truncate toward zero, wrapping. MIN/-1 == MIN.

    Raises ZeroDivisionError on b == 0 -- a refuter must decide what a zero divisor
    means for its axiom rather than inherit a silent 0.
    """
    if b == 0:
        raise ZeroDivisionError("i64 division by zero")
    q = abs(a) // abs(b)
    if (a < 0) != (b < 0):
        q = -q
    return w(q)


def trem(a, b):
    """Bebop remainder: a - (a/b)*b with the truncating quotient, wrapping."""
    return sub(a, mul(tdiv(a, b), b))


def lsr(x, k):
    """`>>` -- LOGICAL right shift by k in [0,63]."""
    return w(u(x) >> k)


def asr(x, k):
    """`>>>` -- ARITHMETIC right shift by k in [0,63]."""
    return w(x >> k)


def shl(x, k):
    """`<<` -- left shift by k in [0,63], wrapping."""
    return w(u(x) << k)


# ---------------------------------------------------------------------------
# The boundary-first point generator.
# ---------------------------------------------------------------------------
# ORDER IS THE CONTRACT: every point in BOUNDARY is visited before any LCG value.
# The list is built once, deduplicated with insertion order preserved, so a point
# added later cannot displace an earlier one.

# Constants that are boundaries for THIS tree specifically, not for i64 in general:
#   2^32 and its neighbours   -- Q32's scale (1.0 in fixed point is 2^32)
#   2^16                      -- fp_mul_impl's inner limb split
#   0xFFFFFFFF                -- st_len's mask
#   3037000499 / 3037000500   -- isqrt(i64::MAX) and its successor, the point the
#                                OLD isqrt_correct's upper bound wrapped at
DOMAIN_CONSTANTS = (
    1 << 16, (1 << 16) - 1, (1 << 16) + 1,
    1 << 32, (1 << 32) - 1, (1 << 32) + 1,
    0xFFFFFFFF, 0x100000000,
    3037000499, 3037000500,
    8192, 1024, 1023, 256,
)


def _build_boundary():
    pts = []
    def push(v):
        v = w(v)
        if v not in seen:
            seen.add(v)
            pts.append(v)
    seen = set()
    # 1. zero and the unit neighbourhood, signed both ways
    for v in (0, 1, -1, 2, -2, 3, -3, 10, -10):
        push(v)
    # 2. the representable extremes and their neighbours
    for v in (MIN, MIN + 1, MIN + 2, MAX, MAX - 1, MAX - 2):
        push(v)
    # 3. every power of two and both neighbours, in both signs
    for k in range(0, BITS):
        p = 1 << k
        for v in (p, p - 1, p + 1, -p, -p + 1, -p - 1):
            push(v)
    # 4. the tree's own domain constants
    for c in DOMAIN_CONSTANTS:
        push(c)
        push(-c)
    return tuple(pts)


BOUNDARY = _build_boundary()

# A smaller ordered prefix for CROSS PRODUCTS, so a two-argument sweep stays
# affordable while still visiting every structural boundary in both coordinates.
# It keeps the whole of groups 1 and 2 above, the powers of two at every 4th
# exponent plus their neighbours, and the domain constants.
def _build_cross():
    keep = []
    seen = set()
    def push(v):
        v = w(v)
        if v not in seen:
            seen.add(v)
            keep.append(v)
    for v in (0, 1, -1, 2, -2, 3, -3, 10, -10,
              MIN, MIN + 1, MIN + 2, MAX, MAX - 1, MAX - 2):
        push(v)
    for k in (1, 15, 16, 17, 31, 32, 33, 47, 62, 63):
        p = 1 << k
        for v in (p, p - 1, p + 1, -p, -p + 1, -p - 1):
            push(v)
    for c in DOMAIN_CONSTANTS:
        push(c)
        push(-c)
    return tuple(keep)


CROSS = _build_cross()


def lcg_stream(seed=0x853c49e6748fea9b):
    """The same 64-bit LCG constants Theorems.lean's `fp_mul_sweep` uses, so a
    random tail here is comparable with the sweep it is meant to subsume.
    Yields i64 values forever. It runs AFTER the boundary, never instead of it."""
    x = seed & UMASK
    while True:
        x = (x * 6364136223846793005 + 1442695040888963407) & UMASK
        yield w(x)


def points(n_random=0, seed=0x853c49e6748fea9b):
    """Boundary points first, in order, then n_random LCG values."""
    for p in BOUNDARY:
        yield p
    if n_random:
        g = lcg_stream(seed)
        for _ in range(n_random):
            yield next(g)


def pairs(n_random=0, seed=0x853c49e6748fea9b):
    """The full cross product of CROSS x CROSS first, then n_random LCG pairs."""
    for a in CROSS:
        for b in CROSS:
            yield a, b
    if n_random:
        g = lcg_stream(seed)
        for _ in range(n_random):
            yield next(g), next(g)


def self_test():
    """Assert this model against numbers MEASURED ELSEWHERE IN THE TREE.

    Every assertion here quotes a value produced by something other than this
    file -- tcheck.bp's divprobe, LANGUAGE.md's shift measurement, or Lean's own
    output in Theorems.lean's header. A model validated only against itself is
    the failure this whole lane exists to catch.

    Returns a list of (claim, source) that were checked. Raises AssertionError
    with the offending line on any mismatch.
    """
    checked = []

    def chk(got, want, claim, src):
        assert got == want, "%s: got %r want %r (%s)" % (claim, got, want, src)
        checked.append((claim, src))

    src = "selfhost/tcheck.bp:2101 MEASURED PREREQUISITE (tmp/divprobe.bp)"
    chk(tdiv(7, 3), 2, "7/3 == 2", src)
    chk(trem(7, 3), 1, "7 rem 3 == 1", src)
    chk(tdiv(-7, 3), -2, "-7/3 == -2", src)
    chk(trem(-7, 3), -1, "-7 rem 3 == -1", src)
    chk(tdiv(-1, 3), 0, "-1/3 == 0", src)
    chk(trem(-1, 3), -1, "-1 rem 3 == -1", src)
    chk(tdiv(7, -3), -2, "7/-3 == -2", src)
    chk(trem(7, -3), 1, "7 rem -3 == 1", src)

    src = "docs/LANGUAGE.md:64 via ROADMAP.md:206 (`>>` is LOGICAL)"
    chk(lsr(-16, 4), 1152921504606846975, "(-16) >> 4", src)
    chk(lsr(MIN, 32), 2147483648, "MIN >> 32", src)
    chk(asr(-16, 4), -1, "(-16) >>> 4 (arithmetic, for contrast)", src)

    src = "formal/Bebop/Theorems.lean:38-40 (Lean's own output, quoted in header)"
    r = 3037000499
    chk(mul(r + 1, r + 1), -9223372036709301616,
        "(isqrt(MAX)+1)^2 wraps to -9223372036709301616", src)

    src = "MIN/-1 overflow, wrapping"
    chk(tdiv(MIN, -1), MIN, "MIN/-1 == MIN", src)
    chk(neg(MIN), MIN, "0 - MIN == MIN", src)

    return checked


if __name__ == "__main__":
    import sys
    try:
        rows = self_test()
    except AssertionError as e:
        print("i64_model: REFUTED -- %s" % e)
        sys.exit(1)
    print("i64_model: %d/%d assertions hold against measured tree values" % (len(rows), len(rows)))
    print("boundary_points: %d  cross_points: %d  cross_pairs: %d"
          % (len(BOUNDARY), len(CROSS), len(CROSS) ** 2))
    for claim, src in rows:
        print("  ok  %-52s [%s]" % (claim, src))
