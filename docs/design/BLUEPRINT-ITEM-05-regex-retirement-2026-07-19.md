# BLUEPRINT — Item 5: Retire `regex`, the Kernel's Last External Crate

> Planning artifact for the final Tier-1 item of
> `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` (§B "Item 5", proof §G.10).
> Binding procedure: `PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md` (commit
> `8f4180279`) — walked explicitly in §4, same as items 4+29 did.
> Ground truth read from worktree `/root/dowiz-wt-space-grade-exec` @ `eb350464e`
> (branch `exec/space-grade-tier0-2026-07-19` — includes the items-4+29 tracing retirement,
> NOT yet on `main`). All file:line cites below are against that worktree.

## 0. Executive summary

`regex` is the kernel's last external dependency. Its entire production surface is **one
function** — `TrigramIndex::query_regex` (`kernel/src/retrieval/index.rs:167-174`) — which has
**zero production callers**; the only code in the repository that exercises it is its own test,
and the only pattern ever compiled anywhere is `note-.*-recall`. The crate's full generality
(alternation, classes, repetition, Unicode, DFA guarantees) is 100% unused capability costing
5 crates in the default no-dev tree. The ruling this blueprint prepares: **terminal state (a),
removed outright** — replace the seam with a kernel-owned matcher for the *actually-used*
pattern subset (literal bytes + `.` + `.*`, unanchored contains semantics), reject everything
outside the subset loudly (degrade-closed), and prove parity against the real `regex` crate on
the full corpus **before** the flip. Expected post-removal `cargo tree -e no-dev`: **0 external
crates** (verified §5, not assumed). Three commits, mirroring the items-4+29 shape
(`f04142f89` coexist → `4f4872a54` cutover → `eb350464e` remove+shrink).

## 1. Usage map — exhaustive (procedure step 2)

Sweep method: `grep -rn -E 'regex|Regex'` over `kernel/`, `engine/`, `apps/`, `tools/`,
`agent-loop/`, all `Cargo.toml`s, and `kernel/benches/` in the worktree.

### Code sites (the crate is actually linked/used)

| # | Site | What it is |
|---|------|------------|
| U1 | `kernel/Cargo.toml:145` — `regex = "1"` | The dependency declaration (resolved `1.13.1`, `kernel/Cargo.lock:1690`). Comment lines 142-144 already name it "item 5's retirement target". |
| U2 | `kernel/src/retrieval/index.rs:7` — `use regex::Regex;` | The only import in the whole kernel. |
| U3 | `kernel/src/retrieval/index.rs:167-174` — `pub fn query_regex(&self, pattern: &str) -> Result<Vec<u32>, regex::Error>` | **The one production seam.** Trigram-narrowed candidates verified by `Regex::new(pattern)?` + `re.is_match(doc)`. Unanchored contains-match semantics, byte-level docs (ASCII note names per module contract, `index.rs:3-5`). |
| U4 | `kernel/src/retrieval/index.rs:183-185` — `candidate_count_regex` | **Name only.** Calls hand-rolled `literal_trigrams`; never touches the crate. Rename + doc update, no logic change. |
| U5 | `kernel/src/retrieval/tests.rs:87-105` — test `regex_query_exact_and_zero_false_positives` + `tests.rs:92` `regex::Regex::new` | The only caller of U3 in the entire repo, using the crate as its own inline oracle. The only pattern ever compiled: `note-.*-recall`. |

### Callers of the seam

`query_regex` / `candidate_count_regex`: **zero callers** outside `tests.rs` — verified across
`kernel/`, `engine/`, `apps/`, `tools/`, `agent-loop/`, and all of `kernel/benches/` (benches
contain no regex references at all; `retrieval_geo.rs` uses the literal path only).

### Doc-comment-only mentions (no code — cutover updates these strings)

- `kernel/src/messenger.rs:3` — "Ports messenger.ts (…regex normalize) into Rust" — the port is
  **already hand-rolled**; comment is historical.
- `kernel/src/lib.rs:245` and `kernel/src/retrieval/mod.rs:4` — "exact byte+regex search"
  module docs → become "exact byte+pattern search (restricted wildcard subset)".
- `kernel/src/ports/llm.rs:343` — "no regex/crate" (already regex-free; no change).
- `kernel/src/fdr/mod.rs:11-12` — items-4+29 ruling text naming `{regex}` as the survivor →
  gains one line noting item 5 closed it.
- `kernel/Cargo.toml:142-144` comment, `kernel/ZERO-DEP-ALLOWLIST.txt` (5 crate names + header).

## 2. Honest pattern characterization (per site)

- **U3 `query_regex`** — the API *accepts the full regex language*, and hand-rolling a general
  regex engine would be genuinely risky and is **not proposed**. But the *exercised* pattern
  language — the procedure's "real current usage" test — is exactly one shape: literal runs
  joined by `.*`. There is no backreference, no alternation, no class, no repetition, no
  Unicode usage anywhere in the repo. The generality is speculative capability with zero
  callers, not real regex work. The honest replacement is therefore a **contract shrink**: a
  matcher for the used subset plus loud rejection of everything else — not regex emulation.
- **U4 `candidate_count_regex`** — trivially replaceable: it already is hand-rolled; only the
  name says regex.
- **U5 test** — replaceable by construction: the incumbent becomes the pre-cutover parity
  oracle (§4 step 7) and its verdicts are frozen as goldens.
- `literal_trigrams` (`index.rs:32-75`) — already pure hand-rolled; its conservative
  metacharacter superset (`* + ? ( ) [ ] { } | ^ $ . \`) stays sound: any meta byte still
  terminates a literal run, so candidate sets remain a safe superset for subset patterns.

## 3. Replacement design — kernel-owned restricted pattern matcher

New code in `kernel/src/retrieval/` (either inside `index.rs` or a sibling `pattern.rs`,
~80-110 lines + tests):

- **Pattern language** (documented in the module doc as a closed contract):
  - literal bytes (byte-level, matching the byte-level trigram index);
  - `.` — any single byte;
  - `.*` — any gap (the only quantifier, only as this two-byte token);
  - **anything else from the meta set → `Err(PatternError::UnsupportedMeta { byte, pos })`** —
    parse-then-match; never a silent wrong answer (degrade-closed, `arena.rs` discipline).
- **Semantics**: unanchored contains-match, bit-identical to `Regex::is_match` restricted to
  this subset (proven by parity tests, not asserted).
- **Algorithm**: split the pattern into fixed-length byte-mask segments at `.*` boundaries
  (masks = literals with `.` holes); greedy left-to-right leftmost placement of each segment
  (masked substring scan), unanchored tail. Correctness: the classic glob lemma — with no
  nested quantifiers, greedy leftmost placement preserves matchability; no backtracking exists,
  so no pathological blowup is possible. Worst case O(|doc|·|pattern|) per candidate doc,
  after trigram narrowing — trivial at corpus scale (20 fixture docs / 2000 synthetic).
- **API**: `query_regex` → `query_pattern(&self, pattern: &str) -> Result<Vec<u32>, PatternError>`;
  `candidate_count_regex` → `candidate_count_pattern`. Zero callers ⇒ clean rename, no alias.
  The error-type change is a public-API break with a verified-empty blast radius.
- **Seam rule (procedure step 6)**: no call site may ever name a matcher implementation
  directly — `TrigramIndex` methods are the single seam, so a future re-adoption of `regex`
  (reopening trigger, §4 step 10) slots behind the same signature.

## 4. The standing procedure, walked (10 steps — BINDING per procedure §3)

1. **Trigger.** Roadmap §B item 5 + synthesis §0.1 zero-dependency push; §18(a) mandates this
   exact procedure. Items 4+29 shrank the allowlist `{regex, tracing, tracing-subscriber}` →
   `{regex}` (`ZERO-DEP-ALLOWLIST.txt`, `fdr/mod.rs:11-12`); item 5 is the named final shrink
   `{regex}` → `{}`.
2. **Sweep.** §1 above: one manifest line, one import, one production seam with zero
   production callers, one misnamed hand-rolled sibling, one self-testing test, one pattern
   ever compiled (`note-.*-recall`).
3. **Edge verified in-house.** What `regex 1.13.1` actually gives THIS kernel today:
   (a) `is_match` correctness for one test pattern; (b) full RE syntax, Unicode classes, and a
   linear-time DFA guarantee against pathological patterns — **exercised by nobody**; the
   subset replacement has no pathological cases by construction. Cost, measured (§5): 5 crates
   = 100% of the kernel's remaining external no-dev tree. Honest loss accounting: we give up a
   battle-tested general engine and future full-regex queries without a re-adoption step; the
   corpus is ASCII note names by module contract, so Unicode loss is nil today.
4. **In-kernel alternative compile-checked BEFORE ruling.** Commit 1/3 lands the matcher +
   the full parity suite while `regex` is still present and green (the `f04142f89` coexistence
   shape). The flip happens only after parity is green.
5. **Terminal state: (a) removed outright.** Not (b) opt-in — a feature flag would preserve a
   dead API for zero callers (the exact outcome the procedure warns of); not (c) — a pattern
   matcher is not a syscall/wire/ABI boundary.
6. **Rollback path.** Callers bind only to the `TrigramIndex` seam (already true modulo the
   test oracle). Three isolated commits: revert of commits 2-3 restores the incumbent
   wholesale; the last artifact with `regex` linked is commit 1's tree. Lockfile diff is
   confined to the 5 departing entries.
7. **Test coverage BEFORE cutover** (parity against the incumbent, not "looks right"):
   - **Cross-product parity (commit 1)**: for a pattern battery — literal-only; single `.`;
     multiple `.` holes; leading/trailing/multiple `.*`; degenerate `.*`, `.*.*`, empty
     pattern; patterns < 3 bytes (no-trigram fallback ⇒ all-docs scan path); the historical
     `note-.*-recall` — over BOTH the frozen 20-doc `FIXTURE` and the 2000-doc synthetic
     corpus: assert `hand_rolled(p, d) == regex::Regex::new(p).unwrap().is_match(d)` for
     **every (pattern, doc) pair**, and `query_pattern == query_regex` doc-id vectors.
   - **Rejection tests**: every unsupported metacharacter (`+ ? ( ) [ ] { } | ^ $ \`, bare
     `*`) → typed `Err`, position reported.
   - **proptest differential** (dev-dep already present, `Cargo.toml` dev-deps): random
     subset patterns × random ASCII docs, hand-rolled vs `regex` crate (commits 1-2); after
     removal, hand-rolled vs an independently-written naive recursive reference matcher —
     two independent implementations must agree forever.
   - **Golden freeze**: commit 1 captures the regex-crate verdict vectors for the battery
     over `FIXTURE` as literals in the test file; post-removal these goldens keep asserting
     the incumbent's verdicts permanently (the items-4+29 "byte-compatible" analogue).
   - **Full kernel suite green in both configurations** (pre-flip with regex, post-flip
     without) — the roadmap's "existing parsing tests green" clause.
8. **Mechanical absence.** `cargo tree --manifest-path kernel/Cargo.toml -e no-dev --locked
   --offline --prefix none | grep -vc '^dowiz-kernel'` → **0**; the command written into the
   `Cargo.toml` tombstone comment at the removed dependency site (`Cargo.toml:74-76`
   precedent). `ZERO-DEP-ALLOWLIST.txt` shrinks 5 names → 0 names in the same change (gate B
   shrink-only GREEN; gate A vacuously green on an empty actual set; gate C lockfile-hash
   stable — `scripts/zero-dep-gate.sh` prints "0 external crates").
   **Variant considered and rejected**: demoting `regex` to `[dev-dependencies]` as a
   permanent live oracle would also satisfy the no-dev proof, but leaves 5 entries in
   `Cargo.lock` and breaks symmetry with the 4+29 full removal; goldens + the independent
   reference matcher provide the same assurance without the residue.
9. **Ruling recorded in three places.** (i) `retrieval/index.rs` (or `pattern.rs`) module
   doc — "Why this exists", steps 1-5 summarized, `fdr/mod.rs:1-40` format; (ii) the
   `Cargo.toml` comment where `regex = "1"` stood — invariant + step-8 one-liner;
   (iii) this blueprint + an UPDATE line in the allowlist header noting the final shrink.
10. **Reopening trigger.** A **real production caller** (not a test, not "might be handy")
    needing pattern features beyond {literal, `.`, `.*`} — alternation, classes, bounded
    repetition, Unicode. Resolution then chooses extend-the-subset vs re-adopt `regex` as
    opt-in state (b), through this same procedure. Nothing else reopens it.

## 5. Verified crate count — now and after (measured in the worktree, not assumed)

`cargo tree -e no-dev --locked --offline` in `/root/dowiz-wt-space-grade-exec/kernel` today:

```
dowiz-kernel v0.1.0
└── regex v1.13.1
    ├── aho-corasick v1.1.4 ── memchr v2.8.3
    ├── memchr v2.8.3
    ├── regex-automata v0.4.16 ── {aho-corasick, memchr, regex-syntax v0.8.11}
    └── regex-syntax v0.8.11
```

= exactly **5 external crates** (`regex`, `regex-automata`, `regex-syntax`, `aho-corasick`,
`memchr`), all in regex's own subtree — matching the allowlist's 5 lines. `dowiz-kernel` is
the path-local root package, explicitly filtered by the gate (`grep -v '^dowiz-kernel$'`) and
not "external" by any reading. After removal the tree is the single root line ⇒ **0 external
crates** — the roadmap §G.10 proof condition, literally. Dev-only deps (`criterion 0.5`,
`paste 1.0`, `proptest 1.11` and their lock closure — `Cargo.lock` holds 321 packages total)
are outside the no-dev proof surface by items-1+13 design and are unchanged by this item.

## 6. Scope boundary — what item 5 must NOT touch

- **`tools/skillspector-rs/Cargo.toml:15`** declares its **own** `regex = "1"` — a separate
  tool crate with a separate tree. Out of scope, verbatim per the gate's scope rule
  (`scripts/zero-dep-gate.sh`: "dowiz-kernel default no-dev build ONLY. Per-crate /
  workspace-wide gating is item 31"). Its ruling belongs to item 31's enactment half.
- No other `Cargo.toml` in the workspace declares `regex` (full-tree grep) — `engine/`,
  `apps/`, `agent-*`, all other `tools/` are clean.
- Dated design docs that say "byte+regex search" (M1/W1-3 blueprints) are historical records —
  not rewritten, per the items-4+29 precedent; only live module docs (§1 list) change.

## 7. Execution shape + definition of done

Three commits on `exec/space-grade-tier0-2026-07-19`, mirroring items 4+29:
1/3 matcher + parity suite coexisting with `regex` (all green, incumbent untouched);
2/3 seam cutover (`query_pattern`, error type, renames, doc-string updates, goldens frozen);
3/3 `regex = "1"` removed, `Cargo.lock` regenerated, allowlist 5→0, tombstone comment +
three-place ruling records.

**Done =** gate prints "0 external crates" · full kernel suite green post-flip · step-7
parity/rejection/proptest/golden tests all landed · step-9 records written · step-10 trigger
named. Estimated new code ~110 lines + ~150 lines of tests; risk concentrated entirely in
matcher correctness, which the pre-cutover cross-product parity against the live incumbent
retires before the flip.
