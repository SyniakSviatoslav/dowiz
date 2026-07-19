# BLUEPRINT — Item 14: `rust-toolchain.toml` pin + structural compiler-bump trigger

> Roadmap: `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §B "Item 14", §G.8.
> Synthesis: `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §10/P5(b), item 14 (line 246):
> the §4 assembly spot-check "fires only if someone *notices* the bump — a dead pendulum by
> construction. Fix: pin the toolchain … and have CI derive 'bump happened' from the pinned version
> changing, failing the bump diff unless the spot-check artifact accompanies it."
>
> Proof condition (§G.8): *a toolchain-bump diff without the spot-check artifact fails CI; a
> non-bump diff never triggers the job.*

Status: BLUEPRINT (planning artifact). Nothing here is implemented yet.

---

## 1. Verified current state (2026-07-19, live checks — not index)

| Fact | Evidence |
|---|---|
| **No pin file exists anywhere.** No `rust-toolchain.toml` and no legacy `rust-toolchain` in the repo — root, `kernel/`, `engine/`, `tools/*`, or any `.worktrees/*` checkout. | `find /root/dowiz -name "rust-toolchain*"` → zero hits (only the two SPACE-GRADE docs mention the string). |
| **Local toolchain = `rustc 1.96.1 (31fca3adb 2026-06-26)`**, `cargo 1.96.1 (356927216 2026-06-26)`. | `rustc --version` / `cargo --version` on this box. |
| **CI has NO explicit toolchain step.** `ci.yml` is the only Rust-invoking workflow (`safety-floor.yml`, `heartbeat-monitor.yml`, `skill-security.yml` contain no cargo/rustc). No `dtolnay/rust-toolchain`, no `actions-rs/toolchain`, no `rustup` invocation anywhere in `.github/workflows/`. Every Rust job just runs `cargo …` on `ubuntu-latest`, i.e. **CI currently floats on whatever stable the GitHub runner image preinstalls**. | grep over `.github/workflows/` (zero toolchain-action hits); `ci.yml` jobs `eqc-proofs`, `cargo-test`, cargo-audit/deny, etc. |
| **Only one `rust-version` (MSRV) field in the whole repo:** `tools/nfc-pod-flipper/Cargo.toml:20` → `rust-version = "1.85.0"`. `kernel/Cargo.toml` and `engine/Cargo.toml` have `edition = "2021"`, no `rust-version`. | grep over all `Cargo.toml`. |
| **No nightly anywhere.** Zero `#![feature(...)]` in `kernel/src` / `engine/src`, zero `cargo +nightly` in workflows. | grep. |
| Per-crate lockfiles exist (`kernel/Cargo.lock`, `engine/Cargo.lock`, `tools/*` …); CI jobs `cd` into each crate dir (per the SCOPE RULE banner) — relevant because rustup discovers the pin by walking **up** from cwd, so one root file covers every crate. | `ci.yml:128-144`, lockfile find. |

Conclusion: the pin file is **genuinely absent**, and the verified version to pin is **1.96.1** —
the only concrete, verifiable version in the whole build story (CI's floating runner-stable cannot
be read out of the repo; see gotcha G1).

## 2. The exact pin

Create **`/root/dowiz/rust-toolchain.toml`** (repo root — nowhere else; rustup's upward search
makes it govern `kernel/`, `engine/`, and every `tools/*` crate, and it lands in each future
worktree checkout once committed):

```toml
# Item 14 (space-grade roadmap §B / §G.8): the compiler is a pinned, audited input.
# Bumping `channel` is a structural event: CI's toolchain-bump gate requires
# docs/audits/toolchain/spot-check-<new-version>.md in the same diff. See
# docs/design/BLUEPRINT-ITEM-14-toolchain-pin-2026-07-19.md.
[toolchain]
channel = "1.96.1"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Decisions, each one line:
- `channel = "1.96.1"` — exact version, never `"stable"` (a floating channel is precisely the dead
  pendulum item 14 exists to kill). Matches the verified local toolchain, so zero dev friction today.
- `profile = "minimal"` — no docs payload on CI runners; smallest install when rustup has to fetch.
- `components` — rustfmt + clippy are used interactively even though no CI job gates on them;
  without listing them, a pinned rustup refuses `cargo fmt`/`cargo clippy` locally. Two components,
  ~40 MB, once per machine. Nothing else (no `targets` section — no cross-compile target in CI;
  the wasm crates' needs can be added to this file when a CI job actually builds them).
- Legacy bare `rust-toolchain` format: rejected — TOML form is the current standard and supports
  `components`/`profile`.
- 1.96.1 satisfies the only MSRV in-tree (`nfc-pod-flipper` 1.85.0) and edition 2021. No conflict.

## 3. The structural compiler-bump trigger (CI job design)

### 3.1 Design choice: always-run job with an internal diff check — NOT a `paths:` filter

Two candidate mechanisms:

1. **Path-filtered workflow** (`on: pull_request: paths: ["rust-toolchain.toml"]`) — literally "the
   job never runs on a non-bump diff", but as a **required status check** it deadlocks every PR
   that doesn't touch the file (the check never reports, the PR waits forever). The known
   workaround (a second workflow with `paths-ignore` carrying a same-named no-op job) doubles the
   moving parts and rests on subtle `paths-ignore` all-files semantics. Rejected.
2. **Always-run job, internal `git diff` on the one file** — the job always reports (so it can be a
   required check), and the *enforcement branch* fires only when the pinned `channel` value
   actually changed. On a non-bump diff it exits 0 in <1s with an explicit
   `no toolchain bump — gate vacuously green` log line. **Chosen.**

Reading of §G.8 under design 2: "the job" that must never trigger on a non-bump diff is the
enforcement path (bump-detected → artifact-required), and its non-firing is verifiable in the log
line — the same reading the roadmap already accepts for item 1's always-on dependency gate.

### 3.2 Bump detection

Compare the `channel` value at the comparison base vs HEAD — value comparison, not mere
file-touched, so comment/whitespace edits to the file do **not** fire the enforcement path:
- **pull_request**: base = `git merge-base origin/$GITHUB_BASE_REF HEAD` (three-dot semantics).
- **push**: base = `github.event.before`, falling back to `HEAD~1` when it is the zero SHA
  (first push / force push). On `schedule` runs base resolves to `HEAD~1` → channel unchanged →
  vacuous green.
- **File absent at base** (i.e. the introduction commit of the pin itself) counts as a bump
  `<absent> → 1.96.1` — deliberate: the pin must land together with the **baseline** spot-check
  artifact `spot-check-1.96.1.md`, so the audit series starts at the pin, not at the first bump.

### 3.3 The spot-check artifact

**`docs/audits/toolchain/spot-check-<new-version>.md`** — version-named, one per compiler ever
used. Version-naming makes tree-presence sufficient (a file for the *new* version can only exist
if this bump's author added it; no staleness check needed). Two mandatory sections, shape-checked
by CI (grep for exact version string + the two `##` headings):

```markdown
# Toolchain spot-check — Rust <version>
Date / auditor / previous → new version.

## Assembly spot-check
Per synthesis §4 (post-compiler-bump binary audit; "Breaking Bad", arXiv:2410.13489 — `-O` has
broken constant-time code in Kyber/HQC): disassemble each branch-free / secret-dependent path
under the NEW compiler and confirm no secret-dependent branches or memory accesses appeared.
Current named surfaces (extend as §4 checklists add more):
- kernel/src/pq/dsa.rs   — NTT / ring_mul arithmetic
- kernel/src/pq/kem.rs   — FO implicit-rejection tag-compare (§1.6)
- kernel/src/pq/keccak.rs AND kernel/src/event_log.rs — both Keccak-f[1600] copies (until item 25 dedup)
- kernel/src/pq/x25519.rs / hybrid.rs — scalar-mult ladder / hybrid combine
Per surface: the objdump/cargo-asm command used, the verdict, and a hash of the disassembly examined.

## Full-suite re-run
Statement + CI run URL that kernel+engine suites ran green under the new pin.
```

Division of labor (this respects §10/P7's "CI must re-execute, never presence-check" for the parts
CI *can* re-execute): once the pin exists, **the full-suite re-run under the new compiler is
structurally guaranteed for free** — a bump PR changes `rust-toolchain.toml`, rustup honors the
PR's own file, so the existing `cargo-test` / `eqc-proofs` / dudect-and-oracle jobs (item 6) all
execute under the *new* toolchain in that same PR. The artifact's unique, non-automatable load is
the human assembly audit; CI checks its presence and shape, and re-executes everything else itself.

### 3.4 The job (exact YAML, to append to `.github/workflows/ci.yml`)

```yaml
  # ITEM 14 (space-grade roadmap §B / §G.8): structural compiler-bump trigger.
  # Always runs (required-check safe); the enforcement branch fires ONLY when the
  # pinned `channel` in rust-toolchain.toml changed vs the comparison base.
  toolchain-bump-gate:
    name: toolchain-bump gate (spot-check artifact required on bump)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Detect channel bump; require spot-check artifact
        run: |
          set -euo pipefail
          FILE=rust-toolchain.toml
          if [ "${{ github.event_name }}" = "pull_request" ]; then
            git fetch --no-tags origin "${{ github.base_ref }}"
            BASE=$(git merge-base "origin/${{ github.base_ref }}" HEAD)
          else
            BASE="${{ github.event.before }}"
            case "$BASE" in
              0000000000000000000000000000000000000000|"") BASE=$(git rev-parse HEAD~1 2>/dev/null || echo "") ;;
            esac
          fi
          head_channel=$(grep -oP '^channel\s*=\s*"\K[^"]+' "$FILE")
          base_channel=""
          if [ -n "$BASE" ] && git cat-file -e "$BASE:$FILE" 2>/dev/null; then
            base_channel=$(git show "$BASE:$FILE" | grep -oP '^channel\s*=\s*"\K[^"]+' || true)
          fi
          if [ "$head_channel" = "$base_channel" ]; then
            echo "no toolchain bump ($head_channel unchanged) — gate vacuously green"
            exit 0
          fi
          echo "TOOLCHAIN BUMP DETECTED: '${base_channel:-<absent>}' -> '$head_channel'"
          ART="docs/audits/toolchain/spot-check-$head_channel.md"
          [ -f "$ART" ] || { echo "::error::bump to $head_channel without $ART"; exit 1; }
          grep -q "$head_channel" "$ART" \
            || { echo "::error::$ART does not name $head_channel"; exit 1; }
          grep -q '^## Assembly spot-check' "$ART" \
            || { echo "::error::$ART missing '## Assembly spot-check'"; exit 1; }
          grep -q '^## Full-suite re-run' "$ART" \
            || { echo "::error::$ART missing '## Full-suite re-run'"; exit 1; }
          echo "bump to $head_channel accompanied by $ART — gate green"
```

### 3.5 Red→green proof procedure (maps 1:1 onto §G.8)

On a scratch branch, in order:
1. **Non-bump diff never triggers**: any docs-only PR → job green with the vacuous-green log line.
2. **RED**: change `channel` to `"1.96.2"` (or any string) with no artifact → job fails with the
   `::error::bump … without …` line. This red run is the proof the gate bites.
3. **GREEN**: add `docs/audits/toolchain/spot-check-1.96.2.md` (template above) → job green.
4. Discard the scratch branch. The introduction PR itself (pin + baseline
   `spot-check-1.96.1.md` + this job in one diff) is a live end-to-end green run of the
   bump-detected path (`<absent> → 1.96.1`).

## 4. Gotchas — stated honestly

- **G1 — CI floats today; the pin may move CI's compiler in either direction.** Nothing in the
  repo records which stable the `ubuntu-latest` image currently ships — that is *unknowable from
  in-repo evidence* and is itself the disease being cured. If the runner's stable is newer than
  1.96.1, this pin **downgrades CI**; if older, it upgrades it. Either way CI becomes identical to
  the verified dev box, and any breakage the pin surfaces in the introduction PR is a *pre-existing
  version-skew bug made visible*, not a regression. The introduction PR's full CI run is the test.
- **G2 — rustup install cost.** When the pinned version ≠ the runner's default, every Rust job
  (~12 in `ci.yml`) auto-installs 1.96.1 (minimal profile, roughly 30–60 s/job). Accept it first;
  add toolchain caching later only if it measurably hurts. Do NOT dodge it with `RUSTUP_TOOLCHAIN=stable` — that would silently unpin CI.
- **G3 — pinning freezes security point-releases too.** A 1.96.2 fixing a rustc/std CVE no longer
  arrives silently, and the daily `cargo audit` cron does **not** cover toolchain advisories
  (it audits crate deps). Owed: whoever watches RUSTSEC also watches Rust release announcements;
  the cost of a bump is exactly one spot-check artifact — deliberately cheap so bumps stay routine.
- **G4 — the file only binds rustup.** A distro rustc or a Dockerfile installing its own toolchain
  ignores it. CI on the GitHub runner (which fronts cargo with rustup) is the enforcement point;
  any future container build must COPY the pin and install via rustup, or it drifts.
- **G5 — required-check registration is server-side.** Marking `toolchain-bump-gate` required lives
  in branch protection (GitHub settings / `gh api`), not in any repo file. Until the operator (or
  executor, via `gh`) flips it, the gate is advisory. Same caveat already applies to every other
  gate in `ci.yml`; note also this repo's Ship-Discipline suspension means much work lands on
  local `main` without PRs — the push-event branch of §3.2 covers that path.
- **G6 — worktrees.** Existing `.worktrees/*` checkouts predate the pin and won't have the file
  until they rebase/merge; new worktrees get it automatically. Known, harmless.

## 5. Handoff note — Opus executor

One PR (or one commit on the exec branch), five files, this exact order:
1. Write `/root/dowiz/rust-toolchain.toml` — byte-for-byte §2.
2. Write `docs/audits/toolchain/spot-check-1.96.1.md` — template §3.3; the baseline assembly
   audit of the four named pq/event_log surfaces under 1.96.1 is real work, do it, don't stub the
   verdicts. This is the artifact the introduction diff itself requires (§3.2 absent→1.96.1 rule).
3. Append the §3.4 job to `.github/workflows/ci.yml` (keep the repo's SCOPE-RULE banner style).
4. Run the §3.5 red→green sequence on a scratch branch; record the red and green run URLs in the
   commit message or the baseline artifact.
5. Ask the operator to mark `toolchain-bump-gate` as a required status check (G5) — or do it via
   `gh api` if authorized. Do not skip: without it the "fails CI" half of §G.8 is advisory only.
Do NOT: pin `"stable"`; use `on.paths` filtering (§3.1); add per-crate pin files; touch any
`Cargo.lock`; add `rust-version` fields to kernel/engine (separate decision, not item 14).
