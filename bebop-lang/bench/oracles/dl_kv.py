# Oracle for gate `dl_kv` (LV-INC-F): from_kv_chain(k) of crates/bebop-wasm/fixtures/kv3.store re-derived
# here from the image's BYTES (the format: docs/FORMAT-BEBOP-IMAGE-v2.md, kv/delta.rs) -- not from the bebop
# program and not from the Rust: the chain newest first, `old` = the newest older record naming the key
# (put: its value, remove: nothing) else the base, put = +(k, v) - (k, old), remove = -(k, old); rows as
# K64('O' || len8 || bytes) pairs, digest as zset.rs `digest`. Then the integration law: kv / has(K) over
# the base + the whole chain's Z-set must equal kv / has over the image's replay (oracle.py kv_entries).
import sys, pathlib
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parents[2] / "crates" / "bebop-wasm"))
import dl_common as C
import oracle as O

cells, n, live, root, _ = O.image(str(HERE.parents[2] / "crates" / "bebop-wasm" / "fixtures" / "kv3.store"))
assert live is not None and root, "kv3.store: no live superblock / data root"
get = lambda o, i: cells[o + 2 + i]
ref = lambda o, i: (o + get(o, i)) if get(o, i) != 0 else 0
packed = lambda o, at, ln: bytes((cells[o + 2 + ((at + b) >> 3)] >> (8 * ((at + b) & 7))) & 0xFF for b in range(ln))
assert get(root, 5) == 3, "kv3.store is not a v3 root"
kidx, kblob, vidx, vblob = (ref(root, i) for i in (1, 2, 3, 4))
basemap = {packed(kblob, get(kidx, 2 * i), get(kidx, 2 * i + 1)): packed(vblob, get(vidx, 2 * i), get(vidx, 2 * i + 1))
           for i in range(get(root, 0))}
chain, cur = [], ref(root, 6)
while cur:
    kl, vl = get(cur, 2), get(cur, 3)
    chain.append((get(cur, 1), packed(cur, 32, kl), packed(cur, 8 * (4 + (kl + 7) // 8), vl)))
    cur = ref(cur, 0)
assert len(chain) == get(root, 7)


def zadd(z, r, w):
    z[r] = z.get(r, 0) + w
    if z[r] == 0:
        del z[r]


def from_kv_chain(k):
    z = {}
    for i in range(k):
        op, key, val = chain[i]
        older = [c for c in chain[i + 1:] if c[1] == key]
        old = (older[0][2] if older[0][0] == 1 else None) if older else basemap.get(key)
        if old is not None:
            zadd(z, (O.obj_key(key), O.obj_key(old)), -1)
        if op == 1:
            zadd(z, (O.obj_key(key), O.obj_key(val)), 1)
    return z


acc = 0
for k in range(len(chain) + 1):
    z = from_kv_chain(k)
    acc = C.s64(acc * 31 + O.zset_digest(list(z.items())))
status, kv = O.kv_entries(cells, n, root)
assert status == 0
kvrows = {C.row(O.obj_key(a), O.obj_key(b)) for a, b in kv["entries"]}
hasrows = {C.row(r[0]) for r in kvrows}
print(C.s64((C.s64(acc * 31 + C.fnv_rows(kvrows)) * 31 + C.fnv_rows(hasrows)) * 2 + 1))
