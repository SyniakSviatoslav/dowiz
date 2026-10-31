

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
    (truncates, forgotten)."""
    truncates, forgotten = [], 0
    for r in recs:
        if is_redacted(r) or chained_id(r) != r["id"]:
            continue
        b = body_of(r)
        if b.startswith(TRUNCATE_MARK) and len(b) >= len(TRUNCATE_MARK) + 96:
            x = b[len(TRUNCATE_MARK):]
            truncates.append((x[:32], x[32:64]) + struct.unpack_from("<QQQQ", x, 64))
        elif b.startswith(FORGOTTEN_MARK) and len(b) >= len(FORGOTTEN_MARK) + 40:
            forgotten += struct.unpack_from("<Q", b, len(FORGOTTEN_MARK))[0]
    return truncates, forgotten


def check_segments(segs, truncates):
    """`declare.rs` `check_segments`: every segment (newest first) named exactly by an effective
    TRUNCATE, or by root as the prev_root of the declared one above it -- else 75."""
    vouched = None
    for s in segs:
        named = (s["root"], s["prev_root"], s["first"], s["last"], s["redacted"], s["declared"])
        if named not in truncates and vouched != s["root"]:
            raise Refuse(75)
        vouched = s["prev_root"]


def doc_counts(im, recs, heads, keys, log_tip, segs):
    """`doc::verify`'s sixteen counters as the MIT reader computes them (bebop-wasm-reader `ids.rs`,
    `Verifier::checks_signatures` false): ids under dowiz-hub's sha256 scheme or the pre-M0 FNV lanes
    (a legacy id newer than a chained record of its device is broken: the scheme moves once), a
    redaction by its erased nonce, forks by `(actor, seq)`, every seal COUNTED (`checked` 0,
    `admitted` = genesis keys), segments declared by TRUNCATEs on their chain alone or 75."""
    c = dict(records=len(recs), chained=0, legacy=0, redacted=0, broken=0, forks=0, seals=0,
             checked=0, heads=len(heads), keys=len(keys), admitted=sum(1 for _, a in keys if a is None),
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
    cap = len(recs) + 1
    for _, _, _, seal in heads:
        cur, steps = seal, 0
        while cur is not None:
            steps += 1
            c["seals"] += 1
            if steps > cap or not im.crc_ok(cur) or read_seal(im, cur) is None:
                break
            cur = read_seal(im, cur)[3]
    truncates, forgotten = effective(recs)
    check_segments(segs, truncates)
    c["declared"] = forgotten + sum(s["declared"] for s in segs)
    return c
