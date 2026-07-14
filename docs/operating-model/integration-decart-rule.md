# Integration Decart Rule — evaluate before you adopt

> Standing rule (operator, 2026-07-14). Applies to every agent, every project. Companion to the
> Task-Exit Rule. Encodes the operator's technology-selection stance: **agnostic, innovative, ethical —
> zero ideological attachments. Always compare & probe.**

## Principle

Decide by **honest, falsifiable, critical comparison** — never by appeal to authority. Modern /
Rust-native is the **default and the tiebreak**; a proven classical/mature method wins **only when an
honest comparison proves it genuinely better on the merits.** No ideological attachment to *either* the
new or the old.

## The rule

Any **new integration** MUST pass a decart evaluation **first**, and leave a **decart comparison report**
in the commit/PR that introduces it. No silent adoption.

**"New integration" =** a new dependency/crate/package · a new external service or API · a new
transport / provider / backend / protocol · **or replacing one of these with another.**
**Not** covered: internal refactors, in-line version bumps, dev-only tooling that never ships.

## The decart table (comparison report)

Fill one row per criterion, one column per candidate. Cite **evidence** (a number, a link, a test
name) — not social proof.

| Criterion | Modern / Rust-native default | Proven / classical alt | (other) |
|---|---|---|---|
| Fit to sovereign bare-metal core (Rust/WASM; `no_std`+alloc where it matters) | | | |
| Correctness & security — with *falsifiable* proof (KAT/ACVP, constant-time, verifier-actually-rejects) | | | |
| Performance — *measured*, not assumed | | | |
| Supply-chain & license (cargo-deny/npm-audit clean; no banned C build unless justified) | | | |
| Maintainability & clarity (readable, easy to change) | | | |
| Reversibility — can it be a port / adapter / fallback instead of a core commitment? | | | |
| Evidence cited (link / number / test) — NOT "everyone uses it" | | | |

**`DECISION: <chosen> — <honest falsifiable reason>.`**
- **Tiebreak:** criteria tie, or the alternative's advantage is unproven → **modern / Rust-native wins.**
- **Older-as-adapter:** if a non-default is chosen, or an older tech is kept alongside, state plainly that
  it is a **bridge / fallback / port — not purged.** (No dogmatic elimination.)
- **Probe (mandatory):** state the **strongest honest argument AGAINST** the decision and why it didn't
  win. If you cannot state one, you have not probed — go back.

## Banned as a *deciding* reason

"Industry standard / more mature / battle-tested / community-approved / everyone uses it." Social proof
and tradition are **not evidence**. (An honest *technical* case for a mature tool is welcome — and if it
wins on the merits, it is chosen. The ban is on using popularity *as the argument*.)

## Worked example (a real decart)

**Choice:** TLS crypto provider for the bebop wss/iroh transport — `rustls + ring` vs `aws-lc-rs` vs
`native-tls (openssl-sys)`.

| Criterion | rustls + ring (chosen) | aws-lc-rs | native-tls / openssl-sys |
|---|---|---|---|
| Rust-native fit | pure-Rust provider, sovereign default | C library (aws-lc) | C library + system OpenSSL |
| Correctness proof | `hardened_verifier_rejects_self_signed_cert` (verifier *actually* rejects) | same rustls API | not exercised |
| Supply-chain | in-lock, deny-clean | C build, acceptable | drags openssl-sys — deny would flag |
| Reversibility | primary provider (`builder_with_provider`) | **kept as compiled fallback (bridge)** | rejected |

**DECISION:** `rustls + ring` primary — chosen as the Rust-native default and proven by a falsifiable
negative test. **Older-as-adapter:** `aws-lc-rs` stays compiled as an accepted fallback (a bridge, **not
purged**). **Probe:** the honest case *against* was "aws-lc-rs is FIPS-validated / more battle-tested" —
rejected as a *deciding* reason (appeal to authority); no falsifiable requirement here needs FIPS
validation, and ring's correctness is proven by KAT + the verifier test. Commits c837442 / 405a3a8 /
a24127b.

## Enforcement

- **Now (guidance):** this rule is a standing order in `AGENTS.md`; every agent applies it before adding
  or swapping an integration, and attaches the decart report to the change.
- **Follow-up (script gate):** a deterministic pre-commit check (new `dependencies` line in
  `Cargo.toml`/`package.json` ⇒ require a linked decart report) is a scoped next step. Advisory until the
  gate lands; the standing rule is authority in the meantime.
