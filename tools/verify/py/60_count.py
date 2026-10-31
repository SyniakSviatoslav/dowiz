

def frontier_of(segs, actor):
    """`seg::frontier_of`: the row of the NEWEST segment that names the device, or None."""
    return next((f for s in segs for f in s["frontier"] if f[0] == actor), None)


def first_ok(segs, actor, r):
    """`verify.rs` `first_ok`: a device's OLDEST kept record continues the frontier the newest segment
    naming it recorded (seq + 1, prev = that tip); a device no segment names starts at any seq."""
    row = frontier_of(segs, actor)
    if row is None:
        return True
    return row[1] == r["seq"] - 1 and row[2] == r["prev"]


def effective(recs):
    """`declare.rs` `effective` with `signatures` False (this file judges no seal): a declaration counts
    on its chain alone -- its record not redacted and its id = sha256(prev || payload). Returns
    (truncates, forgotten, revokes)."""
    truncates, forgotten, revokes = [], 0, []
    for r in recs:
        if is_redacted(r) or chained_id(r) != r["id"]:
            continue
        b = body_of(r)
        if b.startswith(TRUNCATE_MARK) and len(b) >= len(TRUNCATE_MARK) + 96:
            x = b[len(TRUNCATE_MARK):]
            truncates.append((x[:32], x[32:64]) + struct.unpack_from("<QQQQ", x, 64))
        elif b.startswith(FORGOTTEN_MARK) and len(b) >= len(FORGOTTEN_MARK) + 40:
            forgotten += struct.unpack_from("<Q", b, len(FORGOTTEN_MARK))[0]
        elif b.startswith(REVOKE_MARK) and len(b) >= len(REVOKE_MARK) + 40:
            revokes.append((b[len(REVOKE_MARK):len(REVOKE_MARK) + 32], struct.unpack_from("<q", b, len(REVOKE_MARK) + 32)[0]))
    return truncates, forgotten, revokes


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


def check_segments(segs, truncates):
    """`declare.rs` `check_segments`: every segment (newest first) named exactly by an effective
    TRUNCATE, or by root as the prev_root of the declared one above it -- else 75."""
    vouched = None
    for s in segs:
        named = (s["root"], s["prev_root"], s["first"], s["last"], s["redacted"], s["declared"])
        if named not in truncates and vouched != s["root"]:
            raise Refuse(75)
        vouched = s["prev_root"]


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


def doc_counts(im, recs, heads, keys, log_tip, segs):
    """`doc::verify`'s eighteen counters as the MIT reader computes them (bebop-wasm-reader `ids.rs`,
    `Verifier::checks_signatures` false): ids under dowiz-hub's sha256 scheme or the pre-M0 FNV lanes
    (a legacy id newer than a chained record of its device is broken: the scheme moves once), a
    redaction by the kind byte's high bit, forks by `(actor, seq)`, every seal COUNTED (`checked` 0,
    `genesis` = keys with no ADMIT record), segments declared by TRUNCATEs on their chain alone or 75,
    `keys_disagree` the table against the REVOKEs on their chain alone, `unsealed` from every seal's tip."""
    c = dict(records=len(recs), chained=0, legacy=0, redacted=0, broken=0, forks=0, seals=0,
             checked=0, heads=len(heads), keys=len(keys), genesis=sum(1 for _, a, _ in keys if a is None),
             segments=len(segs), cut=sum(s["last"] - s["first"] + 1 for s in segs),
             cut_redacted=sum(s["redacted"] for s in segs), declared=0)
    at = {}
    for r in recs:
        at.setdefault((r["actor"], r["seq"]), set()).add(r["id"])
    c["forks"] = sum(1 for ids in at.values() if len(ids) > 1)
    by_actor = {}
    for r in recs:
        by_actor.setdefault(r["actor"], []).append(r)
    head_tip = {h[0]: h[2] for h in heads}
    for actor, rs in by_actor.items():
        # M0's forward-once rule continues from the frontier row's `scheme` (W-MG-M3b)
        row = frontier_of(segs, actor)
        tip, moved = head_tip.get(actor, log_tip), row is not None and row[3] == SCHEME_CHAINED
        for j, r in enumerate(rs):
            prev_ok = first_ok(segs, actor, r) if j == 0 else rs[j - 1]["id"] == r["prev"]
            next_names_me = rs[j + 1]["prev"] == r["id"] if j + 1 < len(rs) else tip == r["id"]
            if not prev_ok:
                c["broken"] += 1
            elif chained_id(r) == r["id"]:
                c["chained"] += 1
                moved = True
            elif is_legacy(r):
                c["broken" if moved else "legacy"] += 1
            elif is_redacted(r) and next_names_me:
                c["redacted"] += 1
            else:
                c["broken"] += 1
    cap, tips = len(recs) + 1, []
    for _, _, _, seal in heads:
        cur, steps = seal, 0
        while cur is not None:
            steps += 1
            c["seals"] += 1
            if steps > cap or not im.crc_ok(cur) or read_seal(im, cur) is None:
                break
            tips.append(read_seal(im, cur)[2])
            cur = read_seal(im, cur)[3]
    truncates, forgotten, revokes = effective(recs)
    check_segments(segs, truncates)
    c["declared"] = forgotten + sum(s["declared"] for s in segs)
    c.update(keys_disagree=keys_disagree(keys, revokes), unsealed=unsealed(recs, tips, segs))
    return c
