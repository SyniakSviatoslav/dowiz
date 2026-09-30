#!/usr/bin/env python3
"""Make DEST an exact copy of the repo's HEAD tree, touching only what changed.

    python3 tools/deploy/sync.py <repo> <dest>

WHY NOT A FRESH COPY EVERY TIME. cargo decides whether a path dependency
(kernel, dowiz-core, dowiz-hub, ...) is stale by FILE MTIME. A fresh
`git checkout-index -a` stamps every file with "now", so every deploy would
recompile every local crate. Here a file is rewritten only when its blob id or
mode differs from the last sync, so an unchanged kernel keeps its mtime and its
build. (`git archive` is refused on this box; checkout-index is the same bytes.)

WHAT "EXACT" MEANS, and it is checked, not assumed:
  * every tracked path of HEAD is present with HEAD's bytes: after writing, the
    whole tree is re-hashed with ONE `git hash-object --no-filters --stdin-paths`
    and compared with the index's blob ids; symlinks are compared by target;
  * nothing else is present: any file under DEST that HEAD does not track is
    deleted -- a stale file under workers/api/public would be UPLOADED as an
    asset -- except the build's own outputs listed in KEEP;
  * HEAD's tree, not the working tree: blobs come from the index only after the
    caller has proved index == HEAD == working tree (deploy.sh refuses a dirty
    tree first), and `ls-tree -r HEAD` is what is listed.
A mismatch after one repair pass exits 1 naming the paths. Nothing here writes
to the repository: `ls-tree`, `hash-object` without -w and `checkout-index` are
reads of it.
"""
import os
import subprocess
import sys

KEEP = ("workers/api/build/", "workers/api/.wrangler/", ".deploy-manifest")
MANIFEST = ".deploy-manifest"


def git(repo, *args, data=None):
    r = subprocess.run(["git", "-C", repo, *args], input=data, capture_output=True)
    if r.returncode != 0:
        sys.exit(f"sync: FAIL git {' '.join(args[:2])}: {r.stderr.decode(errors='replace').strip()}")
    return r.stdout


def head_tree(repo):
    """{path: (mode, blob)} for HEAD. Submodules (160000) are refused."""
    out = {}
    for rec in git(repo, "ls-tree", "-r", "-z", "--full-tree", "HEAD").split(b"\0"):
        if not rec:
            continue
        meta, path = rec.split(b"\t", 1)
        mode, kind, blob = meta.decode().split()
        if kind != "blob":
            sys.exit(f"sync: FAIL {path.decode()} is a {kind}; submodules are not deployable")
        out[path.decode()] = (mode, blob)
    return out


def load_manifest(dest):
    try:
        with open(os.path.join(dest, MANIFEST), encoding="utf-8") as f:
            return {p: (m, b) for m, b, p in (ln.rstrip("\n").split(" ", 2) for ln in f if ln.strip())}
    except FileNotFoundError:
        return {}


def checkout(repo, dest, paths):
    if paths:
        git(repo, "checkout-index", "-f", "-z", "--stdin", f"--prefix={dest.rstrip('/')}/",
            data=b"\0".join(p.encode() for p in paths))


def mismatches(repo, dest, tree):
    """Paths whose bytes/target/mode in dest differ from HEAD."""
    bad = []
    regular = []
    for p, (mode, blob) in sorted(tree.items()):
        full = os.path.join(dest, p)
        if mode == "120000":
            want = git(repo, "cat-file", "blob", blob).decode()
            if not os.path.islink(full) or os.readlink(full) != want:
                bad.append(p)
        elif os.path.islink(full) or not os.path.isfile(full):
            bad.append(p)
        elif (mode == "100755") != bool(os.stat(full).st_mode & 0o100):
            bad.append(p)
        else:
            regular.append(p)
    if regular:
        got = git(repo, "hash-object", "--no-filters", "--stdin-paths",
                  data="\n".join(os.path.join(dest, p) for p in regular).encode() + b"\n").decode().split()
        bad += [p for p, h in zip(regular, got) if h != tree[p][1]]
    return bad


def extras(dest, tree):
    """Files and dirs under dest that HEAD does not track (KEEP excepted)."""
    out = []
    for root, dirs, files in os.walk(dest):
        rel = os.path.relpath(root, dest)
        rel = "" if rel == "." else rel + "/"
        for d in list(dirs):
            if (rel + d + "/") in KEEP:
                dirs.remove(d)
            elif os.path.islink(os.path.join(root, d)):
                dirs.remove(d)
                if rel + d not in tree:
                    out.append(rel + d)
        out += [rel + f for f in files if rel + f not in tree and rel + f not in KEEP]
    return out


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: sync.py <repo> <dest>")
    repo, dest = sys.argv[1], os.path.abspath(sys.argv[2])
    os.makedirs(dest, exist_ok=True)
    tree = head_tree(repo)
    old = load_manifest(dest)
    changed = [p for p, v in tree.items() if old.get(p) != v]
    gone = extras(dest, tree)
    for p in gone:
        full = os.path.join(dest, p)
        os.unlink(full) if os.path.islink(full) or os.path.isfile(full) else None
    # A path whose type flipped (file <-> symlink) must be unlinked before checkout.
    for p in changed:
        full = os.path.join(dest, p)
        if os.path.lexists(full) and (os.path.islink(full) != (tree[p][0] == "120000")):
            os.unlink(full)
    checkout(repo, dest, changed)
    bad = mismatches(repo, dest, tree)
    repaired = len(bad)
    if bad:  # something outside this script edited the copy: repair once, then insist
        for p in bad:
            if os.path.lexists(os.path.join(dest, p)):
                os.unlink(os.path.join(dest, p))
        checkout(repo, dest, bad)
        bad = mismatches(repo, dest, tree)
    if bad:
        sys.exit(f"sync: FAIL {len(bad)} path(s) still differ from HEAD: {' '.join(bad[:10])}")
    with open(os.path.join(dest, MANIFEST), "w", encoding="utf-8") as f:
        f.writelines(f"{m} {b} {p}\n" for p, (m, b) in sorted(tree.items()))
    print(f"sync: {len(tree)} files == HEAD; wrote {len(changed)}, removed {len(gone)} extra, repaired {repaired}")


if __name__ == "__main__":
    main()
