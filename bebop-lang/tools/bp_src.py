#!/usr/bin/env python3
"""bp_src -- THE list of files the self-hosting compiler is built from, in codegen order.

2026-09-29 (W-DG4 phase 1, operator: "split in files"): bebop.bp was 9,210 lines against the
8,500-line ratchet, so its first 3,067 lines moved into bebop-lang/compiler/*.bp, pulled back
in by line-initial `use "compiler/..."` lines at the top of bebop.bp. use_expand places every
include AHEAD of the main text in order of first use, so the compiler's function order -- and
its binary -- is unchanged (gen2 == 4d52fe73 byte-identical).

Every tool that reads the compiler's TEXT (literal scans, trap census, fntab zone map, fn-name
order, mutation, diff-based word checks, source digests) must read ALL of these files, or it
silently measures a third less than it did (memory: bebop-instruments-that-measure-nothing).
They read them through here, so the list lives in ONE place: bebop.bp's own `use` lines.

`selfhost/prelude/*` includes are NOT compiler files (they are shared libraries the old tools
never read as part of "bebop.bp" either); only `compiler/` includes are followed.

  python3 tools/bp_src.py --files          # one relative path per line, codegen order
  python3 tools/bp_src.py --deps           # every file the build reads (preludes too), use order
  python3 tools/bp_src.py --cat            # the compiler text, concatenated in that order
  python3 tools/bp_src.py --flat           # ONE equivalent source (compiler uses commented out)
  python3 tools/bp_src.py --digest         # sha256 over (path, bytes) of every file
  python3 tools/bp_src.py --old2new N      # a pre-split `bebop.bp:N` citation -> file:line now

A LOUD failure when a `use "compiler/..."` names a file that does not exist, or when bebop.bp
has no compiler includes at all while compiler/ holds .bp files (the split was undone halfway).
"""
import hashlib, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MAIN = "bebop.bp"
USE = re.compile(r'^use "(compiler/[^"]+)"', re.M)


def die(msg):
    sys.stderr.write("bp_src: " + msg + "\n")
    sys.exit(2)


def files(root=ROOT, main=MAIN):
    """Relative paths (to root) of every compiler source file, includes first, main last."""
    mp = os.path.join(root, main)
    if not os.path.exists(mp):
        die("%s does not exist" % mp)
    inc = USE.findall(open(mp, errors="replace").read())
    seen, out = set(), []
    for p in inc:
        if p in seen: continue
        seen.add(p)
        if not os.path.exists(os.path.join(root, p)):
            die("%s names %s, which does not exist" % (main, p))
        out.append(p)
    cdir = os.path.join(root, "compiler")
    if not out and os.path.isdir(cdir) and any(f.endswith(".bp") for f in os.listdir(cdir)):
        die("compiler/ holds .bp files but %s includes none of them" % main)
    return out + [main]


def paths(root=ROOT, main=MAIN):
    return [os.path.join(root, p) for p in files(root, main)]


def text(root=ROOT, main=MAIN):
    """The compiler text, every file in codegen order, concatenated (no markers)."""
    return "".join(open(p, errors="replace").read() for p in paths(root, main))


def lines(root=ROOT, main=MAIN):
    """[(relpath, lineno, line_without_newline)] over every compiler file, in order."""
    out = []
    for rel in files(root, main):
        for i, ln in enumerate(open(os.path.join(root, rel), errors="replace").read().split("\n"), 1):
            out.append((rel, i, ln))
        if out and out[-1][2] == "": out.pop()      # the split's trailing empty element
    return out


def digest(root=ROOT, main=MAIN):
    h = hashlib.sha256()
    for rel in files(root, main):
        h.update(rel.encode() + b"\0")
        h.update(open(os.path.join(root, rel), "rb").read())
        h.update(b"\0")
    return h.hexdigest()


def is_compiler_main(path):
    """True when `path` is a compiler main file, i.e. it has `use "compiler/..."` lines."""
    try: return bool(USE.search(open(path, errors="replace").read()))
    except OSError: return False


def files_for(path):
    """Absolute paths of the compiler files behind `path` when it is a compiler main file
    (includes first), else [path]. What every `--src X` / `--fntab X` tool reads."""
    ap = os.path.abspath(path)
    if not is_compiler_main(ap): return [ap]
    d, b = os.path.dirname(ap), os.path.basename(ap)
    return [os.path.join(d, r) for r in files(d, b)]


def read(path):
    """The text a tool reading `path` must see: all compiler files for a compiler main, with the
    main file's `use "compiler/..."` lines commented out exactly as use_expand comments them --
    so a literal count or a `use` scan over this text sees what the compiler sees (for an unsplit
    file this is the file itself, byte for byte)."""
    ap = os.path.abspath(path)
    if not is_compiler_main(ap): return open(ap, errors="replace").read()
    return flat(os.path.dirname(ap), os.path.basename(ap))


def labeled_lines(path):
    """[(label, lineno, line)] over files_for(path); label is the path relative to the
    directory of `path` (bebop.bp, compiler/calls.bp, ...), so a finding names a real line."""
    base = os.path.dirname(os.path.abspath(path))
    out = []
    for p in files_for(path):
        rel = os.path.relpath(p, base)
        ls = open(p, errors="replace").read().split("\n")
        if ls and ls[-1] == "": ls.pop()
        out += [(rel, i, ln) for i, ln in enumerate(ls, 1)]
    return out


def expand_order(path, cwd=ROOT):
    """Every file the COMPILER reads for `path`, in use_expand's order: a mirror of
    use_scan (line-initial `use "p"`, paths relative to the compile's cwd, dependencies
    before dependents, each path once), then `path` itself. The fn order of a binary is the
    `^fn ` order over these files -- what perf.py size and pcprof.sh name fns by."""
    seen, out = set(), []
    def scan(p):
        for u in re.findall(r'^use "([^"]+)"', open(p, errors="replace").read(), re.M):
            if u in seen or u.startswith("cas://"): continue
            seen.add(u)
            fp = os.path.join(cwd, u)
            if not os.path.exists(fp): die("%s uses %s, which does not exist" % (p, u))
            scan(fp)
            out.append(fp)
    scan(path)
    return out + [path]


def flat(root=ROOT, main=MAIN):
    """ONE source file equivalent to the split compiler: every compiler file in order, the
    `use "compiler/..."` lines of the main text commented out in place (use_expand would
    otherwise include those files a second time). Other `use` lines (the sha256 prelude)
    stay, so compiling it gives the same binary. For tools that rewrite the source
    (mutate_compiler.py) or feed it to a use-less compiler (ddc.sh --full)."""
    parts = []
    for rel in files(root, main):
        t = open(os.path.join(root, rel), errors="replace").read()
        if rel == main: t = USE.sub(lambda m: "//" + m.group(0), t)
        parts.append(t)
    return "".join(parts)


# The split map, fixed at the split commit (parent 64d2d5cd): old bebop.bp line ranges -> file,
# first line of the body there (after that file's 6-line header). Old :2-:16 stayed at the top
# of bebop.bp. Valid for the tree AS SPLIT; later edits above a line move it, as for any file.
SPLIT = [("compiler/lex_diag.bp", 17, 485), ("compiler/tables.bp", 486, 1187),
         ("compiler/calls.bp", 1188, 1866), ("compiler/sys_builtins.bp", 1867, 2500),
         ("compiler/fnval_match.bp", 2501, 3083)]
HDR = 6


def old2new(n):
    if n == 1: return (MAIN, 1)
    if 2 <= n <= 15: return (MAIN, n + 5)
    if n == 16: return (MAIN, 26)
    for f, a, b in SPLIT:
        if a <= n <= b: return (f, n - a + 1 + HDR)
    return (MAIN, n - 3057)


if __name__ == "__main__":
    a = sys.argv[1:]
    if a == ["--files"]: print("\n".join(files()))
    elif a == ["--cat"]: sys.stdout.write(text())
    elif a == ["--digest"]: print(digest())
    elif a == ["--flat"]: sys.stdout.write(flat())
    elif a == ["--deps"]:
        # DG4 (2026-09-30): EVERY file the compiler build reads, preludes included -- since the
        # per-fn memo, bebop.bp `use`s selfhost/prelude/dagc.bp and through it store.bp, so a
        # change there changes bebop.bin. Text tools keep reading --files (compiler/ only); what
        # asks "did the compiler change?" (the pre-commit hook, bebopc.sh's dirty flag) reads this.
        print("\n".join(os.path.relpath(f, ROOT) for f in expand_order(os.path.join(ROOT, MAIN), cwd=ROOT)))
    elif len(a) == 2 and a[0] == "--old2new": print("%s:%d" % old2new(int(a[1])))
    else: die("usage: bp_src.py --files | --deps | --cat | --flat | --digest | --old2new N")
