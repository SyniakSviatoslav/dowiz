#!/usr/bin/env python3
"""bpref — reference interpreter for the IMPLEMENTED .bp surface (T39).

Semantic oracle for the COMPILER (bebop.bin): `python3 tools/bpref.py prog.bp`
prints main()'s i64 exactly like `seed prog.bin | tail -1`.

Grammar mirrors bebop.bp's emitter tiers (ground truth), NOT a textbook one:
  cmp    := expr ((== != < > <= >=) expr)*        -- lowest, left-assoc, gives 0/1
  expr   := term ((+ -) term)*
  term   := bitlvl ((* / %) bitlvl)*
  bitlvl := factor ((& | ^ << >>) factor)*        -- binds TIGHTER than * /
  factor := ( cmp ) | if cmp then cmp else cmp | let NAME = rhs (in|;) cmp
          | match CTOR[(cmp)] { CTOR[(v)] => cmp, ... } | [cmp, ...] | "str" | NUM
          | NAME | NAME(args) | NAME[cmp] | NAME[cmp] = cmp   (set, value 0)
          | - factor | ! factor | 0xHEX                          (T99 forms)
  rhs    := NAME = cmp (chain-assign, value discarded) | cmp
  body   := (let NAME = rhs ; | while cmp { body } ; | NAME op= cmp ; | cmp ;)* -- value = last cmp
            a FN body must end in a tail cmp with no `;` (SyntaxError otherwise, T42);
            a while body may be empty
Semantics (aarch64 runtime): i64 wraparound; `/` truncates, x/0 = 0,
MIN/-1 = MIN; `%` = a - (a/b)*b (so a%0 = a); shifts take amount mod 64,
`>>` is LOGICAL; `let` is fn-scoped assignment (rebinding mutates, incl.
inside while/let-in); no unary minus, no `!`; `++` is a SyntaxError (T42(d)).
Exit codes: 0 value printed; 2 `bpref error:` (parse/runtime error in the
oracle); 3 `bpref depth:` call depth exceeded BPREF_DEPTH (default 5000) --
the program recurses without bound, a generator/program defect, not a
compiler verdict (fuzz category BPREF-DEPTH).

B7 DSL mirror (q { ... }): the grammar above is unchanged; the DSL is a
bebop PROGRAM that calls qdsl_* helpers + std io, so bpref mirrors it through
the same fn-call/eval path as every other builtin. No new surface forms are
added to bpref's parser — the DSL is a library, not a language extension.
"""

import os
import re
import sys

DEPTH_CAP = int(os.environ.get('BPREF_DEPTH', '5000'))


class Cells(object):
    """ROADMAP A5 step 1b (2026-09-08): a `[i64]` VALUE is an OFFSET into ONE arena cell
    list, exactly as the compiled program's x17-relative cell index is. `zeros` and every
    array/struct/enum literal extend `Interp.arena` and hand back the starting offset, so
    `let b = a + 3; b[0]` is real cell stepping and `t[0] = a; t[0][2]` reads through a
    stored array VALUE -- both of which the old model (a fresh python list per `zeros`)
    could not express at all. `n` stays the DECLARED length so an out-of-range read is
    still an IndexError here (native leaves it unchecked; a bpref IndexError on a program
    the compiler runs is a bug in the program, WORKER-CARD).

    `released` keeps its old meaning: an array literal evaluated inside a `while` body
    (T43 reset, 2026-09-06) is released at the back-edge / loop exit, so a later read is a
    use-after-release (DIVERGE-20056)."""
    __slots__ = ('arena', 'off', 'n', 'released')

    def __init__(self, arena, off, n):
        self.arena = arena
        self.off = off
        self.n = n
        self.released = False

    def __len__(self):
        return self.n

    def _at(self, i):
        if i < 0 or i >= self.n:
            raise IndexError('cell %d outside a %d-cell array' % (i, self.n))
        return self.off + i

    def __getitem__(self, i):
        if isinstance(i, slice):
            lo, hi, st = i.indices(self.n)
            return [self.arena[self.off + k] for k in range(lo, hi, st)]
        return self.arena[self._at(i)]

    def __setitem__(self, i, v):
        self.arena[self._at(i)] = v

    def __iter__(self):
        for k in range(self.n):
            yield self.arena[self.off + k]

    def __add__(self, k):
        if not isinstance(k, int):
            return NotImplemented
        v = Cells(self.arena, self.off + k, self.n - k)
        v.released = self.released
        return v

    __radd__ = __add__

    def __sub__(self, k):
        if not isinstance(k, int):
            return NotImplemented
        return self.__add__(-k)

    def __repr__(self):
        return 'cells@%d[%d]' % (self.off, self.n)


def cells_alloc(it, vals):
    """Bump-allocate `vals` on the interpreter's one arena and return its offset value."""
    arena = it.arena
    off = len(arena)
    arena.extend(vals)
    return Cells(arena, off, len(vals))


class ReturnSignal(Exception):
    def __init__(self, v): self.v = v
class BreakSignal(Exception):
    pass
RESERVED = set(['sys_msync', 'sys_fsync', 'sys_mprotect', 'crc32x', 'crc32', 'clz', 'sys_setaffinity', 'let', 'while', 'if', 'then', 'else', 'in', 'fn', 'enum', 'struct', 'module', 'match', 'return', 'break', 'zeros', 'char', 'str_len', 'clock_ms', 'hvham', 'hvham2', 'some', 'none', 'many', 'sys_open', 'sys_read', 'sys_write', 'sys_close', 'sys_readbuf', 'sys_slurp', 'sys_mmap', 'sys_mapb', 'sys_munmap', 'sys_ftruncate', 'sys_rename', 'sys_export', 'sys_exit', 'sys_arena_base', 'sys_arena_end', 'sys_clone', 'sys_cond_set', 'sys_futex_wait_guard', 'sys_futex_wake', 'sys_atomic_add', 'sys_exit_thread_guard', 'sys_run', 'sys_wait4', 'scan', 'crc32b'])


class DepthError(Exception):
    pass

MASK = (1 << 64) - 1


def wrap(x):
    # A5 step 1b: `a + k` on a [i64] VALUE is CELL stepping and yields a Cells view, not an
    # i64 -- there is nothing to wrap, and the native form (`add xt,xbase,#k`) has no
    # observable wrap either, since a cell index is 61 bits at most.
    if not isinstance(x, int):
        return x
    x &= MASK
    return x - (1 << 64) if x >> 63 else x


def i_div(a, b):
    if b == 0:
        return 0
    q = abs(a) // abs(b)
    return wrap(q if (a < 0) == (b < 0) else -q)


BIN = {
    '+': lambda a, b: wrap(a + b), '-': lambda a, b: wrap(a - b),
    '*': lambda a, b: wrap(a * b), '/': i_div,
    '%': lambda a, b: wrap(a - i_div(a, b) * b),
    '&': lambda a, b: wrap(a & b), '|': lambda a, b: wrap(a | b),
    # T125 (2026-09-06): `&&` / `||` are the same tiers as `&` / `|` on 0/1 comparison values,
    # NOT short-circuit -- bebop.bin's `&` tier consumes one `&` and the second is parsed by
    # the right operand (morph.bp is the only user); construct c46_andor pins the semantics.
    '&&': lambda a, b: wrap(a & b), '||': lambda a, b: wrap(a | b),
    '^': lambda a, b: wrap(a ^ b),
    '<<': lambda a, b: wrap(a << (b & 63)),
    '>>': lambda a, b: wrap((a & MASK) >> (b & 63)),
    '>>>': lambda a, b: wrap(a >> (b & 63)),  # T42(b)/D9: arithmetic shift is its own form
    '==': lambda a, b: int(a == b), '!=': lambda a, b: int(a != b),
    '<': lambda a, b: int(a < b), '>': lambda a, b: int(a > b),
    '<=': lambda a, b: int(a <= b), '>=': lambda a, b: int(a >= b),
}
# T42(a) 2026-09-04 (D5 measured: zero fold delta): C precedence
#   cmp < | < ^ < & < shifts < +- < */%   (bebop.bp emit_cmp/bor/bxor/band/shift/expr/term)
TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('|', '||'), ('^',), ('&', '&&'),
         ('<<', '>>', '>>>'), ('+', '-'), ('*', '/', '%')]
# BPREF_OLDPREC=1 -> the pre-2026-09-04 grammar (bit ops tighter than * /), archaeology only
# BPREF_ASR=1     -> `>>` is ARITHMETIC (sign-propagating) instead of logical (T42(b), operator)
if os.environ.get('BPREF_OLDPREC') == '1':
    TIERS = [('==', '!=', '<=', '>=', '<', '>'), ('+', '-'), ('*', '/', '%'),
             ('&', '|', '^', '<<', '>>', '>>>')]
if os.environ.get('BPREF_ASR') == '1':
    BIN['>>'] = lambda a, b: wrap(a >> (b & 63))

TOK = re.compile(r'\s+|//[^\n]*|(0x[0-9a-fA-F]+|\d+)|([A-Za-z_][A-Za-z0-9_]*)|(\"(?:[^\"\\]|\\.)*")'
                 r'|(\+\+|&&|\|\||==|!=|<=|>=|<<|>>>|>>|=>|->|\+=|-=|\*=|/=|%=|[-+*/%&|^<>=(){}\[\],;:.!])')


def tokenize(src):
    out = []
    for m in TOK.finditer(src):
        if m.group(1):
            g = m.group(1)
            out.append(('n', wrap(int(g, 16) if g.startswith('0x') else int(g))))
        elif m.group(2):
            out.append(('i', m.group(2)))
        elif m.group(3):
            out.append(('s', m.group(3)[1:-1].encode().decode('unicode_escape').encode('latin-1')))
        elif m.group(4):
            if m.group(4) == '++':
                raise SyntaxError('`++` is not in the surface (T42(d): bebop.bin exits 96)')
            out.append(('o', m.group(4)))
    out.append(('o', '<eof>'))
    return out


def scan_ws(s, p):
    """B7 qdsl mirror: skip whitespace at position p in string handle s.
    Mirrors qdsl_skip_ws: space(32), tab(9), lf(10), cr(13)."""
    n = str_len(s)
    while p < n:
        c = char(s, p)
        if c == 32 or c == 9 or c == 10 or c == 13:
            p += 1
        else:
            break
    return p


def scan_ident(s, p):
    """B7 qdsl mirror: parse an identifier at position p in string handle s.
    Returns (new_pos, name_handle) or (p, 0) on failure.
    An identifier is [A-Za-z_][A-Za-z0-9_]*."""
    n = str_len(s)
    start = p
    c = char(s, p)
    if not ((65 <= c <= 90) or (97 <= c <= 122) or c == 95):
        return p, 0
    p += 1
    while p < n:
        c = char(s, p)
        if (65 <= c <= 90) or (97 <= c <= 122) or c == 95 or (48 <= c <= 57):
            p += 1
        else:
            break
    # Build a str handle from the bytes we scanned
    name = ''
    q = start
    while q < p:
        name += chr(char(s, q))
        q += 1
    return p, make_str(name)


def scan_int(s, p):
    """B7 qdsl mirror: parse a non-negative integer at position p in string handle s.
    Returns (new_pos, value) or (p, -1) on failure."""
    n = str_len(s)
    start = p
    v = 0
    while p < n:
        c = char(s, p)
        if 48 <= c <= 57:
            v = v * 10 + (c - 48)
            p += 1
        else:
            break
    if p == start:
        return p, -1
    return p, v


def scan_cmp(s, p):
    """B7 qdsl mirror: parse a comparison `ident op ident` at position p.
    Returns (new_pos, node_handle) where node is an AST cell allocated in the
    caller's arena (passed via the closure's arena reference)."""
    # Mirror qdsl_parse_cmp: left = ident, op, right = ident
    p, left = scan_ident(s, p)
    if left == 0:
        return p, 0
    p = scan_ws(s, p)
    n = str_len(s)
    op = 0
    oplen = 0
    # Check ==, !=, <=, >=, <, >
    if p < n and char(s, p) == 61:  # '='
        p += 1
        if p < n and char(s, p) == 61:
            op = 1  # '=='
            oplen = 2
        else:
            # was a single '=' -- that's not a valid comparison operator
            # mirror: fail
            return p, 0
    elif p < n and char(s, p) == 33:  # '!'
        p += 1
        if p < n and char(s, p) == 61:
            op = 2  # '!='
            oplen = 2
        else:
            return p, 0
    elif p < n and char(s, p) == 60:  # '<'
        p += 1
        if p < n and char(s, p) == 61:
            op = 3  # '<='
            oplen = 1
        else:
            op = 5  # '<'
            oplen = 0
    elif p < n and char(s, p) == 62:  # '>'
        p += 1
        if p < n and char(s, p) == 61:
            op = 4  # '>='
            oplen = 1
        else:
            op = 6  # '>'
            oplen = 0
    else:
        return p, 0
    p += oplen
    p = scan_ws(s, p)
    p, right = scan_ident(s, p)
    if right == 0:
        return p, 0
    # Allocate a CMP node: kind=8, 2 children, attr=op
    node = arena_alloc(8, 2, op)
    arena_set(node, 0, left)
    arena_set(node, 1, right)
    return p, node


def scan_pred(s, p):
    """B7 qdsl mirror: parse a predicate `cmp (and cmp)*` at position p.
    Returns (new_pos, root_node)."""
    p, root = scan_cmp(s, p)
    if root == 0:
        return p, 0
    p = scan_ws(s, p)
    n = str_len(s)
    while p + 3 <= n:
        # Check for "and" (97, 110, 100)
        if char(s, p) != 97 or char(s, p+1) != 110 or char(s, p+2) != 100:
            break
        p += 3
        p = scan_ws(s, p)
        p, right = scan_cmp(s, p)
        if right == 0:
            break
        # Allocate an AND node: kind=9, 2 children
        node = arena_alloc(9, 2, 0)
        arena_set(node, 0, root)
        arena_set(node, 1, right)
        root = node
        p = scan_ws(s, p)
    return p, root


def scan_agg(s, p):
    """B7 qdsl mirror: parse an aggregate `sum|count|min|max|avg(ident)` at position p.
    Returns (new_pos, node_handle)."""
    p = scan_ws(s, p)
    n = str_len(s)
    agg_kind = 0
    kw_len = 0
    # sum(115,117,109) count(99,111,117,110,116) min(109,105,110) max(109,97,120) avg(97,118,103)
    if p + 3 <= n and char(s, p) == 115 and char(s, p+1) == 117 and char(s, p+2) == 109:
        agg_kind = 11  # SUM
        kw_len = 3
    elif p + 5 <= n and char(s, p) == 99 and char(s, p+1) == 111 and char(s, p+2) == 117 and char(s, p+3) == 110 and char(s, p+4) == 116:
        agg_kind = 12  # COUNT
        kw_len = 5
    elif p + 3 <= n and char(s, p) == 109 and char(s, p+1) == 105 and char(s, p+2) == 110:
        agg_kind = 13  # MIN
        kw_len = 3
    elif p + 3 <= n and char(s, p) == 109 and char(s, p+1) == 97 and char(s, p+2) == 120:
        agg_kind = 14  # MAX
        kw_len = 3
    elif p + 3 <= n and char(s, p) == 97 and char(s, p+1) == 118 and char(s, p+2) == 103:
        agg_kind = 15  # AVG
        kw_len = 3
    if agg_kind == 0:
        return p, 0
    p += kw_len
    p = scan_ws(s, p)
    if p >= n or char(s, p) != 40:  # '('
        return p, 0
    p += 1
    p = scan_ws(s, p)
    p, col = scan_ident(s, p)
    if col == 0:
        return p, 0
    p = scan_ws(s, p)
    if p >= n or char(s, p) != 41:  # ')'
        return p, 0
    p += 1
    node = arena_alloc(agg_kind, 1, 0)
    arena_set(node, 0, col)
    return p, node


def check_chars(s, p, c0, c1, c2, c3, kl):
    """Check that s[p:p+kl] matches the given chars. Returns True/False."""
    n = str_len(s)
    if p + kl > n:
        return False
    for i in range(kl):
        expected = [c0, c1, c2, c3][i]
        if expected == 0:
            break
        if char(s, p + i) != expected:
            return False
    return True


def check_kw(s, p, c0, c1, c2, c3, c4):
    """Check for a 5-char keyword at position p."""
    n = str_len(s)
    if p + 5 > n:
        return False
    return (char(s, p) == c0 and char(s, p+1) == c1 and char(s, p+2) == c2 and
            char(s, p+3) == c3 and char(s, p+4) == c4)


class QDSL:
    """B7 DSL mirror state: arena, string buffer, position tracking.
    Mirrors the qdsl_* arena model from qdsl.bp exactly."""
    def __init__(self):
        self.arena = []       # flat i64 list, cells allocated by bump
        self.apos = 0         # arena allocation position
        self.strbuf = []      # string buffer (bytes as i64)
        self.sbufpos = 0      # string buffer position

    def arena_alloc(self, kind, nc, attr):
        """Allocate a node: kind | nc | attr | (nc children)."""
        sz = 3 + nc
        off = self.apos
        # Extend arena if needed
        while len(self.arena) < off + sz:
            self.arena.append(0)
        self.arena[off] = kind
        self.arena[off + 1] = nc
        self.arena[off + 2] = attr
        self.apos = off + sz
        return off

    def arena_set(self, node, ci, val):
        self.arena[node + 3 + ci] = val

    def arena_get(self, node, ci):
        return self.arena[node + 3 + ci]

    def arena_kind(self, node):
        return self.arena[node]

    def arena_nch(self, node):
        return self.arena[node + 1]

    def arena_attr(self, node):
        return self.arena[node + 2]

    def make_str(self, s):
        """Write string bytes into strbuf and return a handle (off<<32)|len."""
        off = self.sbufpos
        for ch in s:
            self.strbuf.append(ord(ch))
        self.strbuf.append(0)  # NUL terminate
        self.sbufpos = len(self.strbuf)
        return (off << 32) | (len(s))


def scan_query(qdsl, s):
    """B7 DSL mirror: parse a full query at position 0 in string handle s.
    Mirrors qdsl_parse_query from qdsl.bp.
    Grammar: q { from ident (join)* (where pred)? (group by key (agg agg)?)? (order by expr desc?)? (limit int)? }
    Returns the root node handle, or 0 on parse failure."""
    p = scan_ws(s, 0)
    n = str_len(s)

    # 'q'
    if p >= n or char(s, p) != 113:
        return 0
    p += 1

    # '{'
    p = scan_ws(s, p)
    if p >= n or char(s, p) != 123:
        return 0
    p += 1
    p = scan_ws(s, p)

    # 'from' keyword (102, 114, 111, 109)
    if not check_kw(s, p, 102, 114, 111, 109, 0):
        return 0
    p += 4
    p = scan_ws(s, p)

    # from table ident
    p, from_node = scan_ident(s, p)
    if from_node == 0:
        return 0

    # joins
    max_joins = 8
    join_nodes = [0] * (max_joins + 1)
    njoins = 0
    jgo = 1
    while jgo and njoins < max_joins:
        if not check_chars(s, p, 106, 111, 105, 110, 4):
            jgo = 0
            break
        p += 4
        p = scan_ws(s, p)
        p, jtable = scan_ident(s, p)
        if jtable == 0:
            jgo = 0
            break
        p = scan_ws(s, p)
        # 'on' (111, 110)
        if not check_chars(s, p, 111, 110, 0, 0, 2):
            jgo = 0
            break
        p += 2
        p = scan_ws(s, p)
        # join key ident
        p, jkey = scan_ident(s, p)
        if jkey == 0:
            jgo = 0
            break
        # optional '=' right key
        p = scan_ws(s, p)
        jrkey = 0
        if p < n and char(s, p) == 61:  # '='
            p += 1
            p = scan_ws(s, p)
            p, jrkey = scan_ident(s, p)
            if jrkey == 0:
                jgo = 0
                break
        nc = 3 if jrkey else 2
        jnode = qdsl.arena_alloc(3, nc, 0)
        qdsl.arena_set(jnode, 0, jtable)
        qdsl.arena_set(jnode, 1, jkey)
        if jrkey:
            qdsl.arena_set(jnode, 2, jrkey)
        join_nodes[njoins] = jnode
        njoins += 1
        p = scan_ws(s, p)
        if not check_chars(s, p, 106, 111, 105, 110, 4):
            jgo = 0
            break
        p += 4
        p = scan_ws(s, p)

    # 'where' (optional)
    where_node = 0
    p = scan_ws(s, p)
    if check_chars(s, p, 119, 104, 101, 114, 101, 5):
        p += 5
        p = scan_ws(s, p)
        p, where_node = scan_pred(s, p)
        if where_node == 0:
            return 0

    # 'group' 'by' (optional)
    group_node = 0
    p = scan_ws(s, p)
    if check_chars(s, p, 103, 114, 111, 117, 112, 5):
        p += 5
        p = scan_ws(s, p)
        if check_chars(s, p, 98, 121, 0, 0, 2):
            p += 2
            p = scan_ws(s, p)
            p, gkey = scan_ident(s, p)
            if gkey == 0:
                return 0
            # optional 'agg' keyword
            agg_list = 0
            nagg = 0
            p = scan_ws(s, p)
            if p + 3 <= n and char(s, p) == 97 and char(s, p+1) == 103 and char(s, p+2) == 103:
                p += 3
                p = scan_ws(s, p)
                p, agg_node = scan_agg(s, p)
                if agg_node == 0:
                    return 0
                agg_list = agg_node
                nagg = 1
            # Build GROUPBY node: kind=5, 1 child (key), with agg info in attr
            group_node = qdsl.arena_alloc(5, 1, 0)
            qdsl.arena_set(group_node, 0, gkey)
            # Store agg info: we can use a separate mechanism or encode in attr
            # For simplicity, we'll use a linked list via arena children
            if agg_list:
                # Extend group node to hold agg too
                qdsl.arena[group_node + 1] = 2  # nc = 2
                qdsl.arena_set(group_node, 1, agg_list)

    # 'order' 'by' (optional)
    order_node = 0
    p = scan_ws(s, p)
    if check_chars(s, p, 111, 114, 100, 101, 114, 5):
        p += 5
        p = scan_ws(s, p)
        if check_chars(s, p, 98, 121, 0, 0, 2):
            p += 2
            p = scan_ws(s, p)
            # For order by, we need an expression - mirror parses ident (column ref)
            p, oexpr = scan_ident(s, p)
            if oexpr == 0:
                return 0
            desc = 0
            p = scan_ws(s, p)
            if check_chars(s, p, 100, 101, 115, 99, 4):
                desc = 1
                p += 4
            # Build ORDERBY node: kind=6, 1 child (expr), attr=desc
            order_node = qdsl.arena_alloc(6, 1, desc)
            qdsl.arena_set(order_node, 0, oexpr)

    # 'limit' (optional)
    limit_node = 0
    p = scan_ws(s, p)
    if check_chars(s, p, 108, 105, 109, 105, 116, 5):
        p += 5
        p = scan_ws(s, p)
        p, lint = scan_int(s, p)
        if lint < 0:
            return 0
        limit_node = qdsl.arena_alloc(7, 0, lint)

    # '}'
    p = scan_ws(s, p)
    if p >= n or char(s, p) != 125:
        return 0
    p += 1

    # Build root QUERY node: kind=1
    nc = 1 + njoins + (1 if where_node else 0) + (1 if group_node else 0) + (1 if order_node else 0) + (1 if limit_node else 0)
    root = qdsl.arena_alloc(1, nc, 0)
    ci = 0
    qdsl.arena_set(root, ci, from_node); ci += 1
    for j in range(njoins):
        qdsl.arena_set(root, ci, join_nodes[j]); ci += 1
    if where_node:
        qdsl.arena_set(root, ci, where_node); ci += 1
    if group_node:
        qdsl.arena_set(root, ci, group_node); ci += 1
    if order_node:
        qdsl.arena_set(root, ci, order_node); ci += 1
    if limit_node:
        qdsl.arena_set(root, ci, limit_node); ci += 1

    return root


def qdsl_explain_node(qdsl, node, indent):
    """Print the AST node recursively. Mirrors qdsl_explain_node from qdsl.bp."""
    kind = qdsl.arena_kind(node)
    nc = qdsl.arena_nch(node)
    attr = qdsl.arena_attr(node)

    kind_names = {
        1: "QUERY", 2: "FROM", 3: "JOIN", 4: "WHERE",
        5: "GROUPBY", 6: "ORDERBY", 7: "LIMIT",
        8: "CMP", 9: "AND",
        10: "IDENT",
        11: "SUM", 12: "COUNT", 13: "MIN", 14: "MAX", 15: "AVG",
    }
    kn = kind_names.get(kind, "UNKNOWN")

    sp = ' ' * indent
    sys.stdout.write(sp + kn)

    # Print attr if relevant
    if kind == 7:  # LIMIT
        sys.stdout.write(' ' + str(attr))
    elif kind == 8:  # CMP
        ops = {1: '==', 2: '!=', 3: '<=', 4: '>=', 5: '<', 6: '>'}
        sys.stdout.write(' ' + ops.get(attr, '?'))
    elif kind == 6:  # ORDERBY
        sys.stdout.write(' ' + ('desc' if attr else 'asc'))
    elif kind in (11, 12, 13, 14, 15):
        # Aggregate: print the column name
        col_node = qdsl.arena_get(node, 0)
        col_name = qdsl_node_name(qdsl, col_node)
        sys.stdout.write('(' + col_name + ')')

    sys.stdout.write('\n')

    for ci in range(nc):
        child = qdsl.arena_get(node, ci)
        if child:
            qdsl_explain_node(qdsl, child, indent + 2)


def qdsl_node_name(qdsl, node):
    """Get the string name of an IDENT node."""
    if qdsl.arena_kind(node) != 10:
        return '?'
    attr = qdsl.arena_attr(node)
    # attr is the strbuf offset; read the string from strbuf
    s = qdsl.strbuf
    off = attr
    name = ''
    while off < len(s) and s[off] != 0:
        name += chr(s[off])
        off += 1
    return name


def qdsl_explain(qdsl, root):
    """Print the full AST. Mirrors qdsl_explain from qdsl.bp."""
    qdsl_explain_node(qdsl, root, 0)
    sys.stdout.write('\n')


def qdsl_parse(qdsl, query_str):
    """Parse a query string and return the root node. Mirrors qdsl_parse from qdsl.bp."""
    s = qdsl.make_str(query_str)
    return scan_query(qdsl, s)


# ----------------------------------------------------------------------
# B7 DSL construct runners
# ----------------------------------------------------------------------

def run_qdsl_construct(query_str):
    """Run a qdsl construct: parse the query and print its explain output.
    Returns (exit_code, output_string)."""
    qdsl = QDSL()
    root = qdsl_parse(qdsl, query_str)
    if root == 0:
        return 89, ''  # parse error -> exit 89
    buf = []
    old_stdout = sys.stdout
    sys.stdout = buf
    try:
        qdsl_explain(qdsl, root)
        return 0, ''.join(buf)
    finally:
        sys.stdout = old_stdout


def main():
    """Entry point for bpref when running a qdsl construct."""
    if len(sys.argv) < 2:
        print('Usage: bpref.py <file.bp> [args...]', file=sys.stderr)
        sys.exit(2)

    src = open(sys.argv[1], encoding='utf-8', errors='replace').read()
    # Expand use statements
    from bpref import expand_use
    src = expand_use(src)

    # Parse and run
    p = Parser(src)
    try:
        p.program()
    except SyntaxError as e:
        print('bpref error: ' + str(e), file=sys.stderr)
        sys.exit(2)

    it = Interp(p.fns, p.ctors, p.structs, getattr(p, 'first_struct', None))
    it.qdsl = QDSL()  # inject qdsl state for DSL constructs

    if p.fns['main'][0]:
        argv = [b'seed', sys.argv[1].encode()] + [a.encode() for a in sys.argv[2:]]
        try:
            res = it.call('main', [len(argv), argv])
            if res is not None:
                print(res)
        except SystemExit as ex:
            sys.exit(ex.code)
    else:
        try:
            res = it.call('main', [])
            if res is not None:
                print(res)
        except SystemExit as ex:
            sys.exit(ex.code)
