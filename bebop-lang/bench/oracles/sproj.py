# DG5 sproj oracle (docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md §6): every number
# bench/vs_rust/std_tests/sproj.bp folds, derived from the RECORD BYTES and the spec's answer
# codes alone -- this file never reads the program's output or its store.
#   log fold (S-2, the four-reader LOG fold of crates/bebop-wasm/src/lib.rs): FNV-1a 64 over
#     seq(8 LE) || payload_len(8 LE) || payload, oldest first. Record i: seq i, 330 bytes
#     (i*31 + j*7) % 251. After the in-place rewrite, record 3's first 8 bytes are 'x'.
#   kv fold (S-3, kv.bp / dowiz InMemoryStore::snapshot_root): FNV-1a 64 over
#     len(8 LE) || key || len(8 LE) || value, keys sorted.
#   how codes (proj.bp st_proj_eval): 0 hit, 1 step (how[1] = records stepped), 2 first read
#     (how[1] = 0), 3 refold (how[1]: 3 tip moved, 4 input generation moved).
# Result: acc*31 + x over the sequence sproj.bp documents, as a wrapping i64.
M = (1 << 64) - 1
OFF, PRIME = 0xcbf29ce484222325, 0x100000001b3


def fnv(h, bs):
    for b in bs:
        h = ((h ^ b) * PRIME) & M
    return h


def payload(i, redacted=False):
    p = bytes((i * 31 + j * 7) % 251 for j in range(330))
    return (b'x' * 8 + p[8:]) if redacted else p


def log_fold(n, redact3=False):
    h = OFF
    for i in range(n):
        p = payload(i, redact3 and i == 3)
        h = fnv(h, i.to_bytes(8, 'little') + len(p).to_bytes(8, 'little') + p)
    return h


def kv_fold(entries):
    h = OFF
    for k, v in sorted(entries):
        h = fnv(h, len(k).to_bytes(8, 'little') + k + len(v).to_bytes(8, 'little') + v)
    return h


seq = []
v300, v301, v351 = log_fold(300), log_fold(301), log_fold(351)
vred = log_fold(351, redact3=True)
seq += [2, v300]                    # first read: no memo
seq += [0, v300]                    # hit
seq += [1, 1, v301, v301]           # +1 record: one fold_step, equal to the full refold
seq += [1, 50, v351, v351]          # +50 records
seq += [3, 3, vred, vred]           # record 3 rewritten in place, tip re-chained: refold (tip)
seq += [0]                          # hit on the rewritten memo
seq += [2, vred]                    # another code: its key finds nothing (K-4)
kv2 = kv_fold([(b'a', b'1'), (b'b', b'22')])
kv3 = kv_fold([(b'a', b'1'), (b'b', b'22'), (b'c', b'333')])
seq += [2, kv2, 0, kv2, 3, 4, kv3]  # kind 2: first, hit, a put re-derives whole (gen)

a = 0
for x in seq:
    a = (a * 31 + (x & M)) & M
print(a - (1 << 64) if a >= 1 << 63 else a)
