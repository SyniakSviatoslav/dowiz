# BLUEPRINT — Items 1+13: CI Zero-Dependency Gate (born deterministic)

Date: 2026-07-19 · Tier 1, first item of §B ordering · Author: planning agent (Fable) · Executor: Opus
Sources: `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §B (lines 117–120), §G.7 (line 205 area);
`SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §0.1 (lines 25–35), item 1 (line 165), item 13 (line 245), §10/P6 (line 196).

Proof condition (§G.7, verbatim): *"CI fails on any new dependency, allowlist shrinks monotonically;
gate verdict identical with networking disabled, lockfile hash unchanged."*

---

## 1. Verified current state (all commands run 2026-07-19 against `kernel/Cargo.lock` at HEAD)

### 1.1 The dependency baseline — GROUNDED, counted, not assumed

`cd kernel && cargo tree -e no-dev --locked --offline` ran clean (lockfile is current; offline
resolution succeeds). Unique external crates in the **default-feature, no-dev** build of
`dowiz-kernel` (crate name confirmed, `kernel/Cargo.toml:2`): **24 total = 3 direct + 21 transitive.**

Direct (the roadmap's "3-crate allowlist" — assumption **CONFIRMED**):

| crate | version | declared at |
|---|---|---|
| `regex` | 1.13.1 | `kernel/Cargo.toml` (`regex = "1"`) |
| `tracing` | 0.1.44 | `kernel/Cargo.toml` (`tracing = "0.1"`) |
| `tracing-subscriber` | 0.3.23 | `kernel/Cargo.toml` (features `env-filter`) |

Transitive (21): aho-corasick, cfg-if, lazy_static, log, matchers, memchr, nu-ansi-term, once_cell,
pin-project-lite, proc-macro2, quote, regex-automata, regex-syntax, sharded-slab, smallvec, syn,
thread_local, tracing-attributes, tracing-core, tracing-log, unicode-ident.

**Correction to the synthesis doc:** its line 25 says "25 unique transitive crates"; the actual
verified count is **24 unique external crates total** (21 transitive + 3 direct). Its own
enumeration on the same line sums to 24. Off-by-one in the prose, not in the tree.

All optional deps (`wasm-bindgen`, `serde`, `serde_json`, `serde_yaml`, `sqlx`, `tokio`, `aes-gcm`,
`curve25519-dalek`, `wgpu`, `pollster`, `thunderdome`) are feature-gated and verified **absent**
from the default no-dev tree — none appear in the 24.

Networking-disabled reproduction verified locally: `unshare -r -n bash -c '<gate pipeline>'`
(user+net namespace, no interfaces) produced the identical 24-crate verdict.

### 1.2 Existing CI dependency-tracking today (what the gate does NOT duplicate)

- **`.github/workflows/ci.yml:232` `supply-chain`** — `cargo audit` (advisories) + `cargo deny`
  against `/root/dowiz/deny.toml` (`deny.toml:26` `[bans]`: `wildcards = "deny"`,
  `multiple-versions = "warn"`) + zero-OCI script. Checks *advisories/licenses/wildcards* — it does
  **not** bound the dependency set; a new well-licensed crate passes it silently.
- **`.github/workflows/ci.yml:266` `decart-dep-lint`** — `scripts/decart-dep-lint.sh origin/main HEAD`
  diffs `**/Cargo.toml` and requires a DECART doc/marker for any **added direct dep entry**.
  Diff-shaped and direct-only: it never sees the transitive closure, never reads `Cargo.lock`, so a
  `cargo update` that pulls a *new transitive crate* through an existing direct dep passes it.
- **`.github/workflows/ci.yml:347` `firewall-agent-loop`** — manifest grep, `agent-loop/` scoped only.
- **No `cargo tree`-based gate exists anywhere in CI.** The only `cargo tree` mention in workflows
  is a comment (`ci.yml:341–344`) explaining why the firewall job deliberately does *not* use it.

So item 1+13 fills a real hole: today, nothing in CI fails when the kernel's default dependency
tree grows.

---

## 2. Design

### 2.1 The allowlist file — `kernel/ZERO-DEP-ALLOWLIST.txt` (new, checked in)

One file, next to the manifest it governs (so item 31's later per-crate extension is "add
`<crate>/ZERO-DEP-ALLOWLIST.txt`", not a schema change). Names only — versions are pinned by
`Cargo.lock` + `--locked`, so version bumps don't churn this file; only *new crate names* do.
Comment lines (`^#`) and blanks are stripped by the gate.

```
# Zero-dep allowlist for the dowiz-kernel DEFAULT no-dev build (roadmap items 1+13).
# INVARIANT: this file may only SHRINK. CI compares HEAD against origin/main and
# fails if any name present here is absent there (= growth). The ONLY sanctioned
# growth path is editing the `zero-dep-gate` job in .github/workflows/ci.yml in
# the same reviewed diff — that edit IS the explicit exception record.
# Rationale + retirement plan: docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md §0.1.
#
# --- 3 direct offenders (retirement: item 4 kills the tracing pair, item 5 kills regex) ---
regex
tracing
tracing-subscriber
# --- transitive closure of the above (locked by kernel/Cargo.lock) ---
aho-corasick
cfg-if
lazy_static
log
matchers
memchr
nu-ansi-term
once_cell
pin-project-lite
proc-macro2
quote
regex-automata
regex-syntax
sharded-slab
smallvec
syn
thread_local
tracing-attributes
tracing-core
tracing-log
unicode-ident
```

Why the full 24 and not only the 3 direct: §G.7 says *"CI fails on any new dependency"* — a new
**transitive** crate arriving via `cargo update` of `regex` is a new dependency, is invisible to
`decart-dep-lint` (see §1.2), and must fail. Listing names-only keeps the file stable across
version bumps. The "3-crate allowlist" of the roadmap is the direct-offender section; the
transitive section exists so the proof condition is literally true. When item 4 lands, this file
drops ~17 names (roadmap §G.9 "cargo tree drops 13+ crates"); when item 5 lands it drops the
remaining regex family — the monotonic-shrink check makes those removals permanent for free.

### 2.2 The CI job — new job `zero-dep-gate` in `.github/workflows/ci.yml`

Placed after `supply-chain` (edit the existing workflow; do not create a new workflow file).
Include the repo's standard `### SCOPE RULE BANNER` comment header like sibling jobs.

```yaml
  # ### SCOPE RULE BANNER (verbatim — see cargo-test job header above)
  # Roadmap items 1+13 (SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19 §B/§G.7):
  # the kernel default no-dev dependency tree must be a subset of
  # kernel/ZERO-DEP-ALLOWLIST.txt, the allowlist may only shrink vs origin/main,
  # the verdict is computed with networking disabled (unshare -n), and
  # kernel/Cargo.lock must be byte-identical after the check (item 13 / §10-P6).
  zero-dep-gate:
    name: kernel zero-dep gate (items 1+13)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0          # needed for the origin/main allowlist comparison
      - name: Pre-fetch crate metadata (networked, before the offline gate)
        run: cargo fetch --locked --manifest-path kernel/Cargo.toml
      - name: Zero-dep gate (offline, no-network namespace)
        run: sudo -E env "PATH=$PATH" unshare -n bash scripts/zero-dep-gate.sh
```

`sudo -E env "PATH=$PATH" unshare -n` = the networking-disabled proof is **continuous**, not a
one-off: every run executes inside a network namespace with no interfaces (only loopback, down).
GitHub ubuntu runners have passwordless sudo; `-E` + explicit PATH keeps the runner's rustup cargo
and `CARGO_HOME`/`HOME` visible. Verified locally that the identical pipeline under
`unshare -r -n` (rootless equivalent) yields the same 24-crate verdict — executor should keep the
rootless form as a fallback if the sudo form misbehaves on the runner
(`unshare -r -n bash scripts/zero-dep-gate.sh`), same guarantee either way. The `cargo fetch
--locked` step runs *before* the namespace so the registry cache is warm; `--offline` inside the
gate then proves resolution never needs the network.

### 2.3 The gate script — `scripts/zero-dep-gate.sh` (new)

Exact logic (executor writes this file; keep `set -euo pipefail`):

```bash
#!/usr/bin/env bash
# Roadmap items 1+13 — kernel zero-dep gate. See BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md
set -euo pipefail
ALLOW=kernel/ZERO-DEP-ALLOWLIST.txt
DOC=docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md

# (13) lockfile hash BEFORE — the gate must not mutate resolution state
h0=$(sha256sum kernel/Cargo.lock | cut -d' ' -f1)

# (1) actual tree: default features, no dev edges, locked, offline, names only
cargo tree --manifest-path kernel/Cargo.toml -e no-dev --locked --offline --prefix none \
  | awk '{print $1}' | sort -u | grep -v '^dowiz-kernel$' > /tmp/zdg-actual.txt

grep -vE '^\s*(#|$)' "$ALLOW" | sort -u > /tmp/zdg-allow.txt

# GATE A — any dependency not in the allowlist ⇒ RED (fails on any new dependency)
new=$(comm -23 /tmp/zdg-actual.txt /tmp/zdg-allow.txt)
if [ -n "$new" ]; then
  echo "ZERO-DEP GATE RED: crate(s) in the kernel default tree but not allowlisted:" >&2
  echo "$new" >&2
  echo "The kernel is contractually zero-dep — see $DOC §0.1. New deps go through the item-25 procedure." >&2
  exit 1
fi

# GATE B — monotonic shrink: HEAD allowlist must be a subset of origin/main's
if base=$(git show origin/main:"$ALLOW" 2>/dev/null); then
  grown=$(comm -13 <(echo "$base" | grep -vE '^\s*(#|$)' | sort -u) /tmp/zdg-allow.txt)
  if [ -n "$grown" ]; then
    echo "ZERO-DEP GATE RED: allowlist GREW vs origin/main (shrink-only invariant):" >&2
    echo "$grown" >&2
    echo "Growth requires an explicit reviewed exception: edit the zero-dep-gate job itself in the same diff. See $DOC §0.1." >&2
    exit 1
  fi
fi   # first-ever commit of the allowlist: no baseline yet, Gate B vacuously green

# GATE C (13) — lockfile hash AFTER must be unchanged (P6: verdict is a function of the repo only)
h1=$(sha256sum kernel/Cargo.lock | cut -d' ' -f1)
if [ "$h0" != "$h1" ]; then
  echo "ZERO-DEP GATE RED: Cargo.lock changed during the check ($h0 -> $h1) — nondeterminism leak (item 13 / §10-P6)." >&2
  exit 1
fi

echo "zero-dep-gate GREEN: $(wc -l < /tmp/zdg-actual.txt) external crates, all allowlisted; allowlist shrink-only OK; lockfile hash stable ($h0)."
```

Notes on exactness:
- `-e no-dev` — dev/bench tooling (criterion etc.) is explicitly *not counted* per synthesis line 21.
- `--locked` — refuses to update `Cargo.lock`; `--offline` — refuses the network. Together with
  Gate C and the namespace they discharge item 13 completely: the verdict depends on the repo, not
  registry state.
- `--prefix none | awk '{print $1}' | sort -u` — normalizes duplicates and `(*)`/`(proc-macro)`
  markers; verified to produce exactly the 24 names of §1.1.
- Default features only, deliberately: the optional feature graphs (pq/gpu/pgrust/wasm/slot-arena)
  are governed by their own feature-flag discipline (synthesis §18(a)) and, later, item 31.
- "Shrinks monotonically" is enforced against **`origin/main`** (same baseline convention as
  `decart-dep-lint`, `ci.yml:274`); removals pass, additions fail. The explicit-exception path is
  editing the gate/workflow itself in the same diff — inherently review-visible, and consistent with
  this repo's suspended-governance posture (no extra approval machinery invented).

### 2.4 How this satisfies §G.7, clause by clause

| Proof clause | Mechanism |
|---|---|
| "CI fails on any new dependency" | Gate A: full-tree name set ⊄ allowlist ⇒ exit 1 (catches direct *and* transitive, which decart-lint cannot) |
| "allowlist shrinks monotonically" | Gate B: `comm -13 base HEAD` non-empty ⇒ exit 1 |
| "gate verdict identical with networking disabled" | The gate *always* runs under `unshare -n` + `--offline`; verified locally (identical 24-crate verdict in-namespace) |
| "lockfile hash unchanged" | Gate C: sha256 of `kernel/Cargo.lock` before vs after |

Red-proof obligations for the executor (do these once, in the PR that lands the gate):
1. Add a dummy crate to `kernel/Cargo.toml` `[dependencies]` on a throwaway branch → gate must go RED (Gate A).
2. Add a fake name to the allowlist on a throwaway branch → gate must go RED (Gate B).
3. Confirm the green run's log line reports 24 crates and a stable hash.

---

## 3. Scope boundary — read before executing

**This gate is `dowiz-kernel`-only.** Roadmap §C item 31 (enactment half, Tier 2, "depends on
items 1 and 25") is the explicit later step that makes the gate *per-crate workspace-wide* — "a
checked-in allowlist of each crate's permitted default-build dependencies, failing on any
addition" (roadmap line 462 area; synthesis §25 table already classifies every workspace crate,
e.g. `tools/ci-truth`/`eqc-rs`/`ops-alert` = "exemplary zero-dep", engine's `cosmic-text` pin
already fixed `c2d0f306a`, `rusqlite`+`sha2` pre-ruled KEEP for item 31's allowlists).

Therefore, for THIS item the executor must NOT:
- add allowlists or tree checks for `engine/`, `tools/*`, `agent-loop/` (has its own firewall gate,
  `ci.yml:347`), `mesh-adapter/`, or the Node `apps/*`;
- gate non-default kernel features or dev-dependencies;
- touch `deny.toml`, `decart-dep-lint`, or the `supply-chain` job — they are complementary layers,
  not competitors.

Under-scoping guard: the gate MUST cover the transitive closure (not just the 3 direct crates) and
MUST run offline-in-namespace — both are the item-13 half of the bundle; shipping only the direct
check would be item 1 alone, which the roadmap explicitly bundled away.

## 4. Handoff to Opus executor — exact deliverables

1. **`kernel/ZERO-DEP-ALLOWLIST.txt`** — verbatim from §2.1 (24 names, comments included).
2. **`scripts/zero-dep-gate.sh`** — per §2.3, `chmod +x`, matching the style of
   `scripts/decart-dep-lint.sh`.
3. **`.github/workflows/ci.yml`** — add the `zero-dep-gate` job per §2.2 after the `supply-chain`
   job (line 232 area). Edit the existing workflow; no new workflow file. Do NOT touch
   `safety-floor.yml` (red-line path).
4. Run the three red-proofs in §2.4 and record their outcomes in the PR/commit body.
5. If the sudo-namespace step fails on the runner, fall back to `unshare -r -n` (rootless, verified
   locally) before falling back to bare `--offline`; only the last option weakens the continuous
   networking-disabled proof, and if taken must be flagged in the commit body as a deviation.
6. Baseline facts to re-verify at execution time (cheap, one command): the 24-name set of §1.1 —
   if `kernel/Cargo.lock` moved since 2026-07-19, regenerate the allowlist's transitive section
   from the §2.3 pipeline output, keep the 3-direct section as is unless `Cargo.toml` changed.
