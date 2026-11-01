

def frontier_of(segs, actor):
    """`seg::frontier_of`: the row of the NEWEST segment that names the device, or None."""
    return next((f for s in segs for f in s["frontier"] if f[0] == actor), None)


def first_ok(segs, actor, r):
    """`verify.rs` `first_ok`: a device's OLDEST kept record continues the frontier the newest segment
    naming it recorded (seq + 1, prev = that tip); a device no segment names starts at any seq."""
    row = frontier_of(segs, actor)
    if row is None:
        return True
    return row[1] == (r["seq"] - 1) & M64 and row[2] == r["prev"]


def forgotten_list(b):
    """`seg::parse_forgotten`: the (id, digest) rows of a FORGOTTEN body, or None when it is not one,
    or when `n` is not what the body holds."""
    if not b.startswith(FORGOTTEN_MARK) or len(b) < len(FORGOTTEN_MARK) + 40:
        return None
    n, rest = struct.unpack_from("<Q", b, len(FORGOTTEN_MARK))[0], b[len(FORGOTTEN_MARK) + 40:]
    if len(rest) % 64 or len(rest) // 64 != n:
        return None
    return [(rest[i:i + 32], rest[i + 32:i + 64]) for i in range(0, len(rest), 64)]


def effective(sound):
    """`declare.rs` `declarations` with `signatures` False (this file judges no seal): a declaration counts
    on its chain alone -- its record classed chained, so its `prev` test held, with no tombstone mark.
    `sound` = those records, oldest first. Returns (truncates, forgotten, revokes, erased): `erased` =
    (id, digest, signer) in file order, FORGOTTEN's list."""
    truncates, forgotten, revokes, erased = [], 0, [], []
    for r in sound:
        if is_redacted(r):
            continue
        b = body_of(r)
        if b.startswith(TRUNCATE_MARK) and len(b) >= len(TRUNCATE_MARK) + 128:
            x = b[len(TRUNCATE_MARK):]
            truncates.append((x[:32], x[32:64]) + struct.unpack_from("<QQQQ", x, 64) + (x[96:128],))
        elif (rows := forgotten_list(b)) is not None:
            forgotten += len(rows)
            erased += [(i, d, r["actor"]) for i, d in rows]
        elif b.startswith(REVOKE_MARK) and len(b) >= len(REVOKE_MARK) + 40:
            revokes.append((b[len(REVOKE_MARK):len(REVOKE_MARK) + 32], struct.unpack_from("<q", b, len(REVOKE_MARK) + 32)[0]))
    return truncates, forgotten, revokes, erased


def keys_disagree(keys, revokes):
    """`settle.rs` under a reader with no verifier: per key the earliest `after_seq` of the REVOKEs
    naming it, on their chain alone (`keys::narrower`: 0 = none, -1 = after 0), against the table's
    `revoked_at`. No seal is judged: this says the table and the records' CLAIM differ, nothing more."""
    def cell(actor):
        c = 0
        for who, after in revokes:
            if who == actor:
                this = after if after > 0 else -1
                c = this if c == 0 else min(c, this)
        return c
    return sum(1 for actor, _, table in keys if table != cell(actor))


def seg_message(s, below):
    """`seg::seg_message`: every field a reader acts on; `blob` as a bit, its offset is not a fact."""
    m = SEG_TAG + below + s["root"] + bytes([s["blob"]])
    m += struct.pack("<QQQQQ", s["first"], s["last"], s["redacted"], s["declared"], len(s["frontier"]))
    for actor, seq, tip, scheme in s["frontier"]:
        m += actor + struct.pack("<Q", seq) + tip + struct.pack("<q", scheme)
    return m


def chain_hash(segs):
    """`seg::chain_hash`: `segs` newest first, folded oldest to newest, each hash over the one below."""
    h = bytes(32)
    for s in reversed(segs):
        h = hashlib.sha256(seg_message(s, h)).digest()
    return h


def check_segments(segs, truncates):
    """`declare.rs` `check_segments`: one effective TRUNCATE names the NEWEST segment, its six fields
    and the hash of the whole chain, else 75. Older segments need no TRUNCATE of their own."""
    if not segs:
        return
    s = segs[0]
    if (s["root"], s["prev_root"], s["first"], s["last"], s["redacted"], s["declared"], chain_hash(segs)) not in truncates:
        raise Refuse(75)


def tombs_foreign(recs, tombs, erased, keys):
    """`declare.rs` `foreign_tombs` with the library's reading of `roots` under `Trust::Genesis`: the
    keys with no ADMIT record. WHAT THAT LETS THIS READER SAY: not 0 = the file's own FORGOTTENs do
    not account for a tombstone as it stands (its id is unlisted, the digest differs, the signer is
    neither its author nor a table root), or list a live record. WHAT IT CANNOT SAY: 0 proves no
    authority. No seal is judged, so a FORGOTTEN counts on its chain alone, and the roots are rows of
    a table nobody signs: a forger with no key can write both. Only a verifier with an algorithm and
    its own roots (`Trust::Only`) turns 0 into "nobody but those two erased a record"."""
    roots = {actor for actor, admit, _ in keys if admit is None}
    listed = {i: (d, by) for i, d, by in erased}
    stones, foreign = set(), 0
    for r in tombs:
        stones.add(r["id"])
        d, by = listed.get(r["id"], (None, None))
        foreign += not (d == chained_id(r) and (by == r["actor"] or by in roots))
    return foreign + sum(1 for r in recs if r["id"] in listed and r["id"] not in stones)


def read_seals(im, heads):
    """`doc/seals.rs` `load`: every seal read once, whichever row reached it, paid for first. A walk that
    meets a seal already read stops there, and one that meets a seal it read itself (a ring) counts one
    bad; an object that fails its crc or is no SEAL ends the walk, one bad, no seal. A null `ref SEAL`
    starts nothing. Returns (tips of the seals read, bad)."""
    last_row, tips, bad = {}, [], 0
    for row, (_, _, _, cur) in enumerate(heads):
        while cur is not None:
            if cur in last_row:
                bad += last_row[cur] == row
                last_row[cur] = row
                break
            im.spend(cur)
            seal = read_seal(im, cur) if im.crc_ok(cur) else None
            if seal is None:
                bad += 1
                break
            last_row[cur] = row
            tips.append(seal[2])
            cur = seal[3]
    return tips, bad


def unsealed(recs, tips, segs):
    """`verify.rs` `unsealed` with no seal judged: from the tip cell of EVERY seal reached from HEADS,
    follow `prev` by id. Each record never visited counts 1; each walk that stops on an id absent
    from the file and neither 0^32 nor the tip of any frontier row of any SEG counts 1. A HEADS
    row's own tip cell is never a start: HEADS is signed by nobody."""
    at = {r["id"]: r for r in recs}
    ends = {f[2] for s in segs for f in s["frontier"]} | {bytes(32)}
    seen, dangling = set(), 0
    for i in tips:
        while i in at and i not in seen:
            seen.add(i)
            i = at[i]["prev"]
        if i not in at and i not in ends:
            dangling += 1
    return max(len(recs) - len(seen), 0) + dangling


def spend_walks(im, recs, tables, segs):
    """§9.4: the walks before the seals, from one budget (`verify.rs`); a file that runs it out is 2.
    `tables` = (HEADS, KEYS); `read_seals` pays for the seals out of the same budget."""
    for o in [r["obj"] for r in recs] + [t for t in tables if t is not None] + [s["obj"] for s in segs] + key_rows(im, tables[1]):
        im.spend(o)


def doc_counts(im, recs, heads, keys, log_tip, segs):
    """`doc::verify`'s eighteen counters as the MIT reader computes them (bebop-wasm-reader `ids.rs`,
    `Verifier::checks_signatures` false): ids under dowiz-hub's sha256 scheme or the pre-M0 FNV lanes
    (a legacy id newer than a chained record of its device is broken: the scheme moves once), a
    redaction by the kind byte's high bit, forks by `(actor, seq)`, every seal COUNTED (`checked` 0,
    `genesis` = keys with no ADMIT record), segments declared by TRUNCATEs on their chain alone or 75,
    `keys_disagree` the table against the REVOKEs on their chain alone, `unsealed` from every seal's tip,
    `tombs_foreign` (printed apart: the ABI's eighteen cells do not hold it) against the table's roots."""
    c = dict(records=len(recs), chained=0, legacy=0, redacted=0, broken=0, forks=0, seals=0,
             checked=0, heads=len(heads), keys=len(keys), genesis=sum(1 for _, a, _ in keys if a is None),
             segments=len(segs), cut=sat(sum(s["last"] - s["first"] + 1 for s in segs)),
             cut_redacted=sat(sum(s["redacted"] for s in segs)), declared=0)
    tombs, sound, at = [], set(), {}
    for r in recs:
        at[(r["actor"], r["seq"])] = at.get((r["actor"], r["seq"]), 0) + 1
    c["forks"] = sum(1 for n in at.values() if n > 1)  # by records: one id held twice is a fork too
    by_actor = {}
    for r in recs:
        by_actor.setdefault(r["actor"], []).append(r)
    head_tip = {}
    for h in heads:
        head_tip.setdefault(h[0], h[2])  # the first row of an actor answers for it
    keyed, ids = {k[0] for k in keys}, {r["id"] for r in recs}
    for actor, rs in by_actor.items():
        # M0's forward-once rule continues from the frontier row's `scheme` (W-MG-M3b)
        row = frontier_of(segs, actor)
        tip, moved = head_tip.get(actor, log_tip), row is not None and row[3] == SCHEME_CHAINED
        last, soft = None, None  # the seq of the previous chained or tombstone record; of a legacy one
        for j, r in enumerate(rs):
            prev_ok = first_ok(segs, actor, r) if j == 0 else rs[j - 1]["id"] == r["prev"]
            next_names_me = rs[j + 1]["prev"] == r["id"] if j + 1 < len(rs) else tip == r["id"]
            dense = (actor not in keyed or at[(actor, r["seq"])] > 1
                     or (last + 1 == r["seq"] if last is not None else
                         row is not None or r["seq"] == 1 or (soft is not None and soft + 1 == r["seq"])
                         or (j == 0 and r["prev"] != bytes(32) and r["prev"] not in ids)))
            if not prev_ok:
                c["broken"] += 1
                last = r["seq"]
            elif chained_id(r) == r["id"]:
                moved, last = True, r["seq"]
                if dense:
                    c["chained"] += 1
                    sound.add(id(r))
                else:
                    c["broken"] += 1
            elif is_legacy(r):
                last, soft = None, r["seq"]
                c["broken" if moved else "legacy"] += 1
            elif is_redacted(r) and next_names_me and (dense or last is None):
                last = r["seq"]
                c["redacted"] += 1
                tombs.append(r)
            else:
                last = r["seq"]
                c["broken"] += 1
    tips, bad = read_seals(im, heads)
    c["seals"] = len(tips) + bad
    truncates, forgotten, revokes, erased = effective([r for r in recs if id(r) in sound])
    check_segments(segs, truncates)
    c["declared"] = sat(forgotten + sum(s["declared"] for s in segs))
    c.update(keys_disagree=keys_disagree(keys, revokes), unsealed=unsealed(recs, tips, segs), tombs_foreign=tombs_foreign(recs, tombs, erased, keys))
    return c
