"""dl_common.py -- the independent twin of selfhost/std/dl_fix.bp (row DG8, SPEC-DATALOG-AND-CODEC Part A).

Not a gate itself: bench/oracles/dl_<gate>.py import it. It rebuilds each fixture from the SAME LCG, applies
the SAME event stream to the EDB, and derives every IDB relation by DIRECT set semantics written per rule
set in Python (a comprehension or a BFS) -- no semi-naive loop, no dirty set, no copy of the bebop
evaluator. A stratifier (Tarjan SCC + Kahn over the condensation) is here for dl_strata and the stale
report of the _neg gates; it shares no code with selfhost/std/dl.bp.

Fold (dl.bp dl_fold / dl_fix.bp dl_idb_fold): per IDB predicate FNV-1a-64 over n then every cell of the
rows sorted lexicographically and zero-padded to 4 cells, combined acc*31 + fold in predicate order,
all as wrapping signed i64.
"""
import json, pathlib, re

ROOT = pathlib.Path(__file__).resolve().parents[3]   # the dowiz repo
BL = ROOT / "bebop-lang"
M64 = (1 << 64) - 1


def s64(x):
    x &= M64
    return x - (1 << 64) if x >> 63 else x


def lcg(s):
    return (s * 1103515245 + 12345) & 2147483647


class Rng:
    def __init__(self, seed):
        self.s = seed

    def __call__(self):
        self.s = lcg(self.s)
        return self.s >> 8


def row(*xs):
    xs = list(xs) + [0] * (4 - len(xs))
    return tuple(xs[:4])


def fnv_rows(rows):
    h = s64(0xcbf29ce484222325)
    rs = sorted(rows)
    h = s64((h ^ len(rs)) * 1099511628211)
    for r in rs:
        for x in r:
            h = s64((h ^ x) * 1099511628211)
    return h


def idb_fold(idb):
    """idb: list of row-sets in IDB predicate index order."""
    acc = 0
    for rows in idb:
        acc = s64(acc * 31 + fnv_rows(rows))
    return acc


# ---- the stratifier (A.4), independent of dl.bp -------------------------------------------------------
def stratify(npreds, kinds, rules):
    """kinds[p] 0 EDB | 1 IDB | 2 builtin; rules = [(head, [(pred, neg), ...]), ...].
    Returns ('ok', strata) or ('negcycle', head, q) or ('unsafe', ...) is checked elsewhere.
    Strata: builtins 0, else 1 + Kahn position of the SCC, ties to the SCC holding the smallest index."""
    dep = {p: set() for p in range(npreds)}
    for h, body in rules:
        for q, neg in body:
            if kinds[q] != 2:
                dep[h].add(q)
    # Tarjan
    index, low, onst, st, comp, cnt = {}, {}, set(), [], {}, [0]

    def visit(v):
        index[v] = low[v] = cnt[0]; cnt[0] += 1; st.append(v); onst.add(v)
        for w in dep[v]:
            if w not in index:
                visit(w); low[v] = min(low[v], low[w])
            elif w in onst:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            members = []
            while True:
                w = st.pop(); onst.discard(w); members.append(w)
                if w == v:
                    break
            rep = min(members)
            for w in members:
                comp[w] = rep
    for p in range(npreds):
        if kinds[p] != 2 and p not in index:
            visit(p)
    for h, body in rules:
        for q, neg in body:
            if neg and kinds[q] != 2 and comp[q] == comp[h]:
                return ('negcycle', h, q)
    reps = sorted(set(comp.values()))
    done, order = set(), []
    while True:
        ready = [c for c in reps if c not in done and
                 all(comp[q] in done or comp[q] == c for p in comp if comp[p] == c for q in dep[p])]
        if not ready:
            break
        c = min(ready); done.add(c); order.append(c)
    strata = [0] * npreds
    for p in range(npreds):
        if kinds[p] != 2:
            strata[p] = order.index(comp[p]) + 1
    return ('ok', strata)


# ---- the five rule sets: shapes (for the stratifier) -------------------------------------------------
SHAPES = {
    1: ([0, 0, 0, 2, 2, 1], [(5, [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)])]),
    2: ([0, 0, 1], [(2, [(0, 0), (1, 0)])]),
    3: ([0, 0, 0, 1, 1], [(3, [(1, 0), (2, 0)]), (4, [(0, 0), (3, 1)])]),
    4: ([0, 0, 1], [(2, [(0, 0)]), (2, [(1, 0), (2, 0)])]),
    5: ([0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1],
        [(6, [(1, 0)]), (6, [(2, 0), (6, 0)]), (7, [(0, 0), (6, 0)]), (8, [(7, 0), (3, 1)]),
         (9, [(3, 0), (7, 1)]), (10, [(4, 0), (7, 0), (5, 0)])]),
}

# ---- fixtures and events (mirror dl_fix.bp line for line) ------------------------------------------------
ALLOWED = [(0, 1), (0, 6), (0, 7), (1, 2), (1, 4), (1, 10), (2, 3), (2, 10), (3, 4), (3, 9), (3, 10),
           (4, 5), (4, 10), (10, 11)]
TERMINAL = [5, 6, 7, 9, 11]


def seed_of(s):
    return 12345 + 1000 * s


class Db:
    def __init__(self, npreds):
        self.F = [set() for _ in range(npreds)]

    def put(self, p, r):
        self.F[p].add(r)

    def pull(self, p, i):
        rs = sorted(self.F[p])
        if rs:
            self.F[p].discard(rs[i % len(rs)])

    def replace(self, p, r):   # keyed on column 0 (dl_replace k = 1)
        old = [x for x in self.F[p] if x[0] == r[0]]
        for x in old:
            self.F[p].discard(x)
        self.F[p].add(r)


def load(set_, db, rnd):
    if set_ == 1:
        for d in range(70):
            k = 6 if d < 2 else 5
            pk = []
            while len(pk) < k:
                s = rnd() % 120
                if s not in pk:
                    pk.append(s)
                    c = 1 + rnd() % 200
                    db.put(0, row(d, s, c))
        for s in range(120):
            db.put(1, row(s, rnd() % 1000))
        for d in range(165):
            db.put(2, row(d, 2 if d % 5 == 0 else 1))
    elif set_ == 2:
        for a, b in ALLOWED:
            db.put(1, row(a, b))
        for o in range(50):
            db.put(0, row(o, rnd() % 12))
    elif set_ == 3:
        for t in TERMINAL:
            db.put(2, row(t))
        for o in range(40):
            db.put(1, row(o, rnd() % 12))
            c = rnd() % 8
            x = rnd() % 3
            if x != 0:
                db.put(0, row(c, o))
    elif set_ == 4:
        for _ in range(5):
            db.put(0, row(rnd() % 60))
        for _ in range(90):
            x = rnd() % 60
            y = rnd() % 60
            db.put(1, row(x, y))
    elif set_ == 5:
        for p, r in sushi_rows()[0]:
            db.put(p, r)


def event(set_, db, rnd):
    if set_ == 1:
        s = rnd() % 120; q = rnd() % 1000
        db.replace(1, row(s, q))
    elif set_ == 2:
        o = rnd() % 50; f = rnd() % 12
        db.replace(0, row(o, f))
    elif set_ == 3:
        k = rnd() % 4; a = rnd() % 40; b = rnd() % 12
        if k < 2:
            db.replace(1, row(a, b))
        elif k == 2:
            db.put(0, row(b % 8, a))
        else:
            db.pull(0, a + b)
    elif set_ == 4:
        k = rnd() % 4; x = rnd() % 60; y = rnd() % 60
        if k == 0:
            db.put(1, row(x, y))
        elif k == 1:
            db.pull(1, x * 60 + y)
        elif k == 2:
            db.put(0, row(x))
        else:
            db.pull(0, x)
    elif set_ == 5:
        k = rnd() % 4; x = rnd() % 4096; y = rnd() % 4096
        if k == 0:
            db.pull(0, x)
        rs0 = sorted(db.F[0])
        sup = rs0[y % len(rs0)][1] if rs0 else 0
        if k == 1:
            db.put(0, row(x % 59, sup, 1))
        rs1 = sorted(db.F[1])
        s1 = rs1[x % len(rs1)][0] if rs1 else 0
        if k == 2:
            db.put(1, row(s1, y % 14))
        if k == 3:
            db.pull(1, y)


def closure(seeds, edges):
    """Reflexive seeds, then every (a, b) edge from a reached a: the least fixpoint by BFS."""
    out = set(seeds)
    succ = {}
    for a, b in edges:
        succ.setdefault(a, []).append(b)
    todo = list(out)
    while todo:
        x = todo.pop()
        for y in succ.get(x, []):
            if y not in out:
                out.add(y); todo.append(y)
    return out


def derive(set_, F):
    """The IDB rows of every IDB predicate, in predicate index order, by direct semantics."""
    if set_ == 1:
        stock = {r[0]: r[1] for r in F[1]}
        qmin = {r[0]: r[1] for r in F[2]}
        unav = set()
        for d, s, c, _ in F[0]:
            if s in stock and d in qmin:
                cm = c * qmin[d]
                assert -(1 << 63) <= cm < (1 << 63)
                if stock[s] < cm:
                    unav.add(row(d))
        return [unav]
    if set_ == 2:
        allowed = {(r[0], r[1]) for r in F[1]}
        return [{row(o, t) for o, f, _, _ in F[0] for (a, t) in allowed if a == f}]
    if set_ == 3:
        term = {r[0] for r in F[2]}
        ended = {row(o) for o, f, _, _ in F[1] if f in term}
        endo = {r[0] for r in ended}
        may = {row(c, o) for c, o, _, _ in F[0] if o not in endo}
        return [ended, may]
    if set_ == 4:
        pers = closure({r[0] for r in F[0]}, [(r[0], r[1]) for r in F[1]])
        return [{row(n) for n in pers}]
    if set_ == 5:
        # carries(S,A): supply_allergen, closed under produced(S2,S): S2 carries what S carries
        sa = {(r[0], r[1]) for r in F[1]}
        prod = [(r[0], r[1]) for r in F[2]]
        carries = set(sa)
        changed = True
        while changed:
            changed = False
            for s2, s in prod:
                for (x, a) in list(carries):
                    if x == s and (s2, a) not in carries:
                        carries.add((s2, a)); changed = True
        allergen = {(d, a) for d, s, _, _ in F[0] for (x, a) in carries if x == s}
        declared = {(r[0], r[1]) for r in F[3]}
        veg = {r[0] for r in F[4]}
        animal = {r[0] for r in F[5]}
        und = allergen - declared
        uns = declared - allergen
        clash = {(d, a) for (d, a) in allergen if d in veg and a in animal}
        return [{row(*x) for x in carries}, {row(*x) for x in allergen}, {row(*x) for x in und},
                {row(*x) for x in uns}, {row(*x) for x in clash}]


NPREDS = {1: 6, 2: 3, 3: 5, 4: 3, 5: 11}


def idb_preds(set_):
    kinds, _ = SHAPES[set_]
    return [p for p in range(len(kinds)) if kinds[p] == 1]


def gate(set_, neg, events):
    """The value dl_fix.bp dl_gate(set, neg, events, every) must return."""
    db = Db(NPREDS[set_])
    rnd = Rng(seed_of(set_))
    load(set_, db, rnd)
    if neg == 0:
        for _ in range(events):
            event(set_, db, rnd)
        return idb_fold(derive(set_, db.F))
    kinds, rules = SHAPES[set_]
    strata = stratify(len(kinds), kinds, rules)[1]
    ids = idb_preds(set_)
    prev = derive(set_, db.F)
    for t in range(1, events + 1):
        event(set_, db, rnd)
        cur = derive(set_, db.F)
        lost = [(strata[p], p, sorted(prev[i] - cur[i])) for i, p in enumerate(ids) if prev[i] - cur[i]]
        if lost:
            s, p, extra = min(lost)
            return t * 1000000000 + p * 1000000 + len(extra) * 1000 + extra[0][0]
        prev = cur
    return -2


# ---- the sushi fixture (set 5) ------------------------------------------------------------------------
EU14 = ["gluten", "crustaceans", "eggs", "fish", "peanuts", "soy", "milk", "nuts", "celery", "mustard",
        "sesame", "sulphites", "lupin", "molluscs"]
# ASSUMPTIONS (authored for this fixture, not read from any declaration -- the hub's catalogue declares
# no allergens for this venue): what each named supply carries directly, and which supplies are made
# from which. A derived allergen the menu does not claim is a FINDING for a person, never a fix.
SUPPLY_ALLERGENS = {
    "salmon": ["fish"], "grilled salmon": ["fish"], "tuna": ["fish"], "tobiko": ["fish"], "surimi": ["fish"],
    "shrimp": ["crustaceans"], "cream cheese": ["milk"], "torched gouda": ["milk"], "egg": ["eggs"],
    "sesame": ["sesame"], "chuka": ["sesame", "soy"], "lemon-kimchi sauce": ["fish"], "crispy onion": ["gluten"],
    "panko shrimp": ["gluten"],
}
MADE_FROM = {
    "Kewpie mayo": ["egg"], "spicy mayo": ["Kewpie mayo"], "crab mix": ["surimi", "Kewpie mayo"],
    "salmon mix": ["salmon", "Kewpie mayo"], "cheese mix": ["cream cheese"], "panko shrimp": ["shrimp", "egg"],
}
CLAIMS = {"salmon": "fish", "tuna": "fish", "shrimp": "crustaceans"}
ANIMAL = ["crustaceans", "fish", "molluscs"]


def sushi_menu():
    return json.loads((ROOT / "design/dubin-sushi-menu.json").read_text())


def resolve(name, dishes):
    """A set's component -> a dish index, or None: exact, case-insensitive, without ' 1/2' or a trailing
    ' Classic', or the unique dish whose name starts with it."""
    low = [d.lower() for d in dishes]
    for cand in (name, name.replace(" 1/2", ""), re.sub(r" Classic$", "", name)):
        if cand.lower() in low:
            return low.index(cand.lower())
    pre = [i for i, d in enumerate(low) if d.startswith(name.lower())]
    return pre[0] if len(pre) == 1 else None


def sushi_rows():
    """([(pred, row)], ids) -- the EDB of set 5 and the name of every id."""
    items = sushi_menu()["items"]
    dishes = [it["name"] for it in items]
    comp = {}
    ingr = set()
    for i, it in enumerate(items):
        lst = []
        for x in it["translations"]["en"]["ingredients"]:
            j = resolve(x, dishes) if "sets" in it["filters"] else None
            if j is None:
                ingr.add(x); lst.append(("i", x))
            else:
                lst.append(("d", j))
        comp[i] = lst
    for n in list(SUPPLY_ALLERGENS) + list(MADE_FROM) + [y for v in MADE_FROM.values() for y in v]:
        ingr.add(n)
    names = sorted(ingr)
    iid = {n: 100 + k for k, n in enumerate(names)}
    ids = {i: d for i, d in enumerate(dishes)}
    ids.update({v: k for k, v in iid.items()})
    out = []
    used_as_supply = set()
    for i, lst in comp.items():
        for kind, x in lst:
            s = x if kind == "d" else iid[x]
            if kind == "d":
                used_as_supply.add(x)
            out.append((0, row(i, s, 1)))
    for n, al in SUPPLY_ALLERGENS.items():
        for a in al:
            out.append((1, row(iid[n], EU14.index(a))))
    for d in sorted(used_as_supply):
        for kind, x in comp[d]:
            out.append((2, row(d, x if kind == "d" else iid[x])))
    for n, srcs in MADE_FROM.items():
        for s in srcs:
            out.append((2, row(iid[n], iid[s])))
    for i, it in enumerate(items):
        for f in it["filters"]:
            if f in CLAIMS:
                out.append((3, row(i, EU14.index(CLAIMS[f]))))
        if "vegetarian" in it["filters"]:
            out.append((4, row(i)))
    for a in ANIMAL:
        out.append((5, row(EU14.index(a))))
    return sorted(set(out)), ids


def sushi_bp():
    """The text of selfhost/std/dl_sushi.bp (GENERATED -- bench/oracles/dl_sushi_gen.py writes it)."""
    rows, ids = sushi_rows()
    L = ["// selfhost/std/dl_sushi.bp -- GENERATED by bench/oracles/dl_sushi_gen.py from design/dubin-sushi-menu.json",
         "// and bench/oracles/dl_common.py's authored supply table (ASSUMPTIONS, see there). DO NOT EDIT: re-run",
         "// the generator; bench/oracles/dl_allergen.py refuses a stale copy. Set 5 of selfhost/std/dl_fix.bp:",
         "// 0 recipe(D,S,1) 1 supply_allergen(S,A) 2 produced(S2,S) 3 declared(D,A) 4 veg(D) 5 animal(A).",
         "// Dishes are item indexes 0..%d, supplies 100 + their index in the sorted name list." % (len([k for k in ids if isinstance(k, int) and k < 100]) - 1),
         "fn dl_sushi_row(db: [i64], p: i64, a: i64, b: i64, c: i64) -> i64 {",
         "  dl_put(db, p, dl_setrow(db, a, b, c, 0))",
         "}"]
    chunk = 0
    for k in range(0, len(rows), 120):
        L.append("fn dl_sushi_load%d(db: [i64]) -> i64 {" % chunk)
        for p, r in rows[k:k + 120]:
            L.append("  let _ = dl_sushi_row(db, %d, %d, %d, %d);" % (p, r[0], r[1], r[2]))
        L.append("  0")
        L.append("}")
        chunk += 1
    L.append("fn dl_sushi_load(db: [i64]) -> i64 {")
    for c in range(chunk):
        L.append("  let _ = dl_sushi_load%d(db);" % c)
    L.append("  %d" % len(rows))
    L.append("}")
    return "\n".join(L) + "\n"


# ---- safety (A-3), independent of dl.bp -----------------------------------------------------------------
def first_unsafe(kinds, ops, rules):
    """rules = [(head, head_args, [(pred, neg, args)])], args: var >= 0, const as ('c', n), None unused.
    ops[p] = builtin op (4 = mul binds its third arg). Returns (rule, var) of the first variable (head args
    first, then each negated atom's args / each builtin's two inputs in atom order) that no positive
    relational atom binds, directly or through a mul whose inputs are bound; None when every rule is safe."""
    for ri, (h, hargs, body) in enumerate(rules):
        bound = set()
        for p, neg, args in body:
            if kinds[p] != 2 and not neg:
                bound |= {a for a in args if isinstance(a, int)}
        for _ in body:
            for p, neg, args in body:
                if kinds[p] == 2 and ops[p] == 4 and not neg:
                    if all(not isinstance(a, int) or a in bound for a in args[:2]) and isinstance(args[2], int):
                        bound.add(args[2])
        check = list(hargs)
        for p, neg, args in body:
            if kinds[p] == 2:
                check += list(args[:2])
            elif neg:
                check += list(args)
        for a in check:
            if isinstance(a, int) and a not in bound:
                return (ri, a)
    return None


def sushi_report():
    """Refuse a stale selfhost/std/dl_sushi.bp, then print the allergen diff of the sushi fixture (before any
    event) with names: derived-but-not-claimed, claimed-but-not-derived, vegetarian dishes deriving an
    animal allergen. A FINDING for the venue, never a fix -- the menu claims only what its filters say."""
    import sys
    f = BL / "selfhost/std/dl_sushi.bp"
    if not f.exists() or f.read_text() != sushi_bp():
        print("dl_sushi.bp STALE -- re-run bench/oracles/dl_sushi_gen.py")
        sys.exit(1)
    rows, ids = sushi_rows()
    db = Db(NPREDS[5])
    for p, r in rows:
        db.put(p, r)
    carries, allergen, und, uns, clash = derive(5, db.F)
    for kind, rs in (("undeclared", und), ("unsupported", uns), ("vegclash", clash)):
        for d, a, _, _ in sorted(rs):
            print("allergen %s dish %d %d  %s: %s" % (kind, d, a, ids[d], EU14[a]))
    print("allergen diff undeclared %d unsupported %d vegclash %d" % (len(und), len(uns), len(clash)))
