

def read(raw):
    """(family, entries-or-records, doc-or-None); raises Refuse. A DOC is asked FIRST: to the KV
    path its version cell reads 'above 4'. A DOC's counters are the caller's second question
    (`doc_counts`), as `bw_verify` is the reader's: the status line comes first either way."""
    im = Image(raw)
    at, used, pt = pick(im)
    last_commit(im, at, used)
    root = im.cell(pt + 18) if pt else 0
    if root <= 0:
        raise Refuse(2)
    if is_doc(im, root):
        recs, heads, keys, spec, _entries, tip, segs, anchors = doc_open(im, root)
        return "doc", [(r["seq"], r["payload"]) for r in recs], (im, recs, heads, keys, tip, segs, anchors, spec)
    if im.digest(root) == DIGEST_EVLOG_ROOT:
        return "log", log_records(im, root), None
    im.need_crc(root)
    ver = im.get(root, 5) if im.obj_cells(root) >= 6 and im.get(root, 5) > 0 else 1
    return "kv", (kv4_entries(im, root) if ver == 4 else kv_entries(im, root)), None


COUNT_LINE = ("verify records=%(records)d chained=%(chained)d legacy=%(legacy)d redacted=%(redacted)d broken=%(broken)d "
              "forks=%(forks)d seals=%(seals)d checked=%(checked)d heads=%(heads)d keys=%(keys)d admitted=%(admitted)d "
              "segments=%(segments)d cut=%(cut)d cut_redacted=%(cut_redacted)d declared=%(declared)d")


def main(argv):
    flags = [a for a in argv[1:] if a.startswith("--")]
    paths = [a for a in argv[1:] if not a.startswith("--")]
    if not paths or "--help" in flags:
        print("usage: python3 bebop-verify.py <image> [--dump] [--verify]", file=sys.stderr)
        return 2
    try:
        family, items, doc = read(open(paths[0], "rb").read())
    except Refuse as r:
        print("kv status=%d n=0 root=0" % r.args[0])
        print(WORDS.get(r.args[0], "status %d" % r.args[0]), file=sys.stderr)
        return r.args[0]
    # the two folds (FORMAT §9.1; lib.rs): LOG = seq8 || len8 || payload oldest first,
    # KV = len8 || key || len8 || value in key order; FNV-1a 64 over the bytes. A DOC's is its log's.
    h = FNV_OFFSET
    for a, b in items:
        head = struct.pack("<Q", a) if family in ("log", "doc") else struct.pack("<Q", len(a)) + a
        h = fnv(h, head + struct.pack("<Q", len(b)) + b)
    print("%s status=0 n=%d root=%d" % (family, len(items), to_i64(h)))
    what = {"log": "EvLog", "doc": "DOC (root-version 5)", "kv": "KV"}[family]
    print("OK; %s; every reached object whole, the last commit anchored" % what, file=sys.stderr)
    if doc is not None:
        im, recs, heads, keys, tip, segs, anchors, spec = doc
        out = sys.stdout if "--verify" in flags else sys.stderr
        try:
            c = doc_counts(im, recs, heads, keys, tip, segs)
        except Refuse as r:
            print("verify status=%d" % r.args[0], file=out)
            print(WORDS.get(r.args[0], "status %d" % r.args[0]), file=sys.stderr)
            return r.args[0]
        print(COUNT_LINE % c, file=out)
        print(UNCHECKED, file=sys.stderr)
        for kind, over, ms, plen in anchors:
            print("anchor kind=%d over=%s time_ms=%d proof_len=%d (present, NOT verified by this reader)" % (kind, over.hex(), ms, plen), file=out)
        if spec is None:
            print("SPEC: none (a working image; archives carry one)", file=sys.stderr)
        else:
            print("SPEC: embedded reader abi %d sha256 %s (the file's CLAIM; this file runs no wasm), format text ref %s"
                  % (spec[3], spec[2].hex(), spec[1]), file=sys.stderr)
        if "--dump" in flags:
            for actor, seq, tip_, _ in heads:
                print("head actor=%s seq=%d tip=%s" % (actor.hex(), seq, tip_.hex()))
            for i, r in enumerate(recs):
                print("#%d seq=%d id=%s payload=%r" % (i, r["seq"], r["id"].hex(), r["payload"]))
    elif "--dump" in flags:
        for i, (a, b) in enumerate(items):
            print("#%d seq=%d payload=%r" % (i, a, b) if family == "log" else "%r = %r" % (a, b))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
