# G09 — bebop2 hand-rolled cryptography: assurance blueprint

> Gap ID: G09 (from `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` §4.5, §6.5, §7.10, §8)
> Scope: `/root/bebop-repo/bebop2/core/src/*` — the from-scratch, zero-dependency PQ crypto core.
> Status of this doc: RESEARCH + EXECUTION BLUEPRINT. Read-only scoping — **no code was modified**;
> `bebop-repo` (branch `feat/wire-native-core`, dirty) and `dowiz` left exactly as found.
> Date: 2026-07-11. Author-context: solo/OSS project, bus-factor 1.

---

## 1. Gap & evidence

**The gap in one sentence:** bebop2 re-implements six cryptographic primitives from scratch with
zero dependencies (SHA-2/SHA-3, ChaCha20/XChaCha20-Poly1305, Ed25519, ML-KEM-768, ML-DSA-65,
Argon2id), the discipline is well above hobby grade (RFC/FIPS KATs, RED cases, a three-model review
that genuinely caught an Ed25519 malleability bug), but **KAT-green ≠ constant-time ≠
side-channel-audited ≠ the years of scrutiny the crates it replaces have had** — and the
delivery-protocol design routes these exact identities into value-bearing paths (PoD, reputation,
escrow), which is where "acceptable for research" becomes "serious".

**Evidence (verified in the working tree, read-only):**

- **Mandate is explicit and from-scratch.** `bebop2/README.md:1-14` ("full from-scratch, no
  vendors, zero-dep, post-quantum era"); `bebop2/core/Cargo.toml` has no deps/dev-deps (audit §4.5
  "zero-dependency claim holds").
- **The core is real and green.** 90 tests in `bebop2-core`; full workspace 384/384 pass (audit
  §4.5, executed live). Per-primitive KATs present (see §2.2).
- **The repo already knows the risk.** `bebop2/README.md:12-14`: "Side-channel gate (accepted): KAT
  correctness + determinism … Full physical side-channel resistance is NOT provable without
  hardware; documented as a known gap, not claimed." `bebop2/ARCHITECTURE.md:131-135`: "Agents must
  NOT 'optimize' PQ crypto into insecurity — the KAT gate forbids it."
- **The identities guard value.** `crates/bebop/src/pod.rs:1-26,70-85`: Proof-of-Delivery =
  `vault.sign(SHA512(claim))` with a **hybrid ML-DSA-65 ⊕ Ed25519** signature over
  `vault::NodeIdentity`; `reputation.rs`/`ledger.rs` build on the same identity; `zkvm.rs` is a
  hash-commitment integrity boundary. Today `crates/bebop/src/vault.rs` uses the **host crates**
  (`ml-kem 0.3`, `ml-dsa 0.1`, `ed25519-dalek 3`, `x25519-dalek 3`, `argon2 0.5`,
  `chacha20poly1305 0.11`, `sha2 0.10` — see `crates/bebop/Cargo.toml:29-38`). bebop2's mandate is
  to **replace those host crates** with its own primitives ("old = oracle … then swapped",
  `README.md:4-5`). That swap is the moment hand-rolled crypto starts guarding real value.
- **Defensive posture already present.** `vault.rs:9-11` runs **hybrids** (PQ ⊕ classical) and
  states "we do NOT trust the unaudited PQ crates alone." Good instinct; the same instinct argues
  against trusting self-written primitives alone.
- **Planned-not-built.** `bebop2/README.md:45-48` lists `kernel/ cli/ reloop/` — none exist
  (audit §6.5). The **equivalence harness against the old-bebop oracle is 0%** (`README.md:64`
  promises `cargo test -p bebop2` equivalence tests; the `bebop2` host crate/harness does not
  exist — only `bebop2-core`).

**Why this is a gap and not just "unfinished":** the failure mode of a subtly-wrong or
timing-leaky primitive is silent and catastrophic (key recovery, forged PoD, drained escrow), and
it is invisible to the green KAT suite. The three-model review already demonstrated that a
review-plus-KAT process catches *functional* deviations (the S≥L malleability bug) — but it did
**not** catch, and is not designed to catch, timing/side-channel or spec-interop defects, several
of which are present today (§2.1). This blueprint scopes the ladder from "research-green" to
"may-guard-value".

---

## 2. Research findings

### 2.1 Per-primitive code scoping (implementation approach + timing hotspots)

> This is a **scoping pass**, not an audit. For each primitive: the approach, the constant-time
> discipline already present, and the obvious secret-dependent hotspots an auditor/harness must
> target. Citations are `file:line` in `bebop2/core/src/`.

**Constant-time discipline already present (the good news):**
- `aead.rs:345 constant_time_eq` — XOR-accumulate tag compare (correct CT equality). Used on the
  AEAD decrypt path (`aead.rs:299`).
- `sign.rs:201 fe_eq` — XOR-accumulate field equality (CT).
- `sign.rs:133 fe_invert` — Fermat inversion; branches only on the **constant** exponent `p-2`, so
  it is effectively input-independent (overlap reviewer confirmed, `.review/overlap-findings-sign.md:17`).
- No secret-dependent table lookups anywhere (no S-box / windowed-table style leaks) — the ciphers
  are ARX (add-rotate-xor), which is the naturally-CT design.
- There is **no `subtle`-style crate** and no repo-wide CT abstraction; CT is ad-hoc, two helpers.

| Primitive | File | Approach | CT status / hotspots (scoping) |
|---|---|---|---|
| **SHA-512** | `hash.rs:82` | FIPS 180-4, straightforward schedule + compression, `Vec` padding | **Constant-time by construction** (ARX, no data-dependent branch/lookup). Length is public. Lowest-risk primitive. |
| **SHA3-256/512, SHAKE** | `hash.rs:175` (+ dup in `pq_kem.rs:71`) | FIPS 202 Keccak-f[1600,24], sponge | **CT by construction.** Note: **two independent Keccak implementations** exist (`hash.rs` and `pq_kem.rs:71-215`) — divergence risk; consolidate. |
| **ChaCha20 / HChaCha20** | `rng.rs:22-105` | RFC 8439 §2.1/2.3.1 + draft-xchacha §2.2, ARX quarter-round | **CT by construction.** CSPRNG counter wraps at 2^32 blocks — documented (`rng.rs:110-114`), caller must reseed. |
| **Poly1305** | `aead.rs:28-251` | 5×26-bit limb (Donna layout), `2^130≡5` fold | Mostly CT. **Hotspot: `reduce`/`sub_once` (`aead.rs:65-100`)** — final reduction loops `for _ in 0..3 { match sub_once … break }`; iteration count is **value-dependent** (bounded ≤3). Donna does a single constant conditional; low severity but flag. Tag compare is CT (`:345`). |
| **XChaCha20-Poly1305 AEAD** | `aead.rs:262-305` | draft-xchacha §A.3 construction | CT compare on decrypt. Inherits Poly1305 reduce hotspot. |
| **Ed25519** | `sign.rs` | RFC 8032, twisted-Edwards, **32-byte LE bignum field via bit-by-bit division** | **MAJOR hotspots.** (1) `scalar_mul` (`sign.rs:487-500`) is **double-and-add with `if bit==1 { point_add }`** — a **secret-scalar-dependent branch** (leaks the private scalar `a` and nonce `r` via timing); it is **not** a Montgomery ladder / constant-time. (2) Field ops `mod_p_be`/`fe_sub`/`cmp_be`/`sub_be` (`:61-103,344-386`) branch on operand values, `Vec`-allocate, and trim leading zeros (**length-leaking, variable-time**). (3) `point_eq` (`:325`) uses short-circuit `==`. Reviewer flagged `verify` as variable-time — acceptable for public inputs, but **signing is not public-input**. This primitive is correctness-first, not CT-first. |
| **ML-KEM-768** | `pq_kem.rs` | FIPS 203, **coefficient-domain schoolbook `poly_mul` (NTT removed as buggy, `:301-307`)** | **MAJOR: (a) NOT FIPS-interoperable.** `kpke_encrypt`/`keygen_internal` store `t`/`s` in the **coefficient domain**, not the NTT domain FIPS 203 mandates for `ek`/`dk` (`:443-446,576`) — so bebop2 `ek/dk/ct` bytes will **never match** the `ml-kem` crate or NIST ACVP vectors. **(b) `poly_mul` (`:268-291`) has `if a[i]==0 { continue }` / `if b[j]==0` — secret-coefficient-dependent branches** (timing leak on `s`,`y`). **(c) `compress`/`decompress` (`:359-367`) divide by `q=3329` on secret-derived data — the KyberSlash division-timing class.** **(d) FO re-encrypt compare `if cprime == *ct` (`:665`) is short-circuit, not CT** (FIPS 203 requires CT implicit rejection). |
| **ML-DSA-65** | `pq_dsa.rs` | FIPS 204, schoolbook mul, **self-admittedly NOT NIST-bit-exact (`:10-14`)** | **MAJOR: (a) spec-deviating sampling.** `expand_a` samples the public matrix **A via CBD** (`:218-232`) — FIPS 204 requires **uniform RejNTTPoly** (A must be uniform in Z_q; CBD gives tiny [-8,8] coeffs). `expand_s` uses CBD not RejBounded uniform. These change the distribution and **are not the standard** (interop-breaking and a **security-margin question an auditor must rule on**). **(b)** challenge hash `c_t` is **32 bytes** but ML-DSA-65 uses λ/4 = **48 bytes** (`LAMBDA=192`), weakening challenge collision resistance; `sample_in_ball` reads a **cyclic 32-byte** stream (`:281`). **(c)** hint packing is non-standard (`:393-416`), `verify` omits the hint-weight ≤ ω check. **(d)** `poly_mul_schoolbook` `if a[i]==0` secret-dependent branch (timing). Honestly flagged as prototype in the header — but this is a **bespoke scheme inspired by ML-DSA, not ML-DSA**. |
| **Argon2id (+BLAKE2b)** | `kdf.rs` | RFC 9106, faithful port of the PHC reference `ref.c` (data-independent addressing for id's first half) | Closest to the reference. `index_alpha`/`fill_segment` mirror `ref.c` (`:325-447`). BLAKE2b from scratch (RFC 7693). Argon2**id** by design has data-dependent memory addressing in its second half (inherent to the algorithm, not a bebop2 defect). **Interoperable** (matches RFC 9106 §5.3 KAT), so differentially testable vs the `argon2` crate. Uncommitted (audit §6.5). |

**Two headline scoping conclusions:**
1. **Interoperable set** {SHA-512, SHA3/SHAKE, ChaCha20/Poly1305/AEAD, Ed25519, Argon2id/BLAKE2b}
   are byte-compatible with the standard → **differentially testable** against host crates,
   Wycheproof, and official vectors. Cheap, high-value assurance is available *today*.
2. **PQ set** {ML-KEM-768, ML-DSA-65} is **NOT byte-interoperable** with FIPS 203/204 by
   construction (coefficient-domain KEM encoding; CBD-sampled DSA matrix + non-standard packing).
   They **cannot** be validated against ACVP or the `ml-kem`/`ml-dsa` crates as oracles until
   re-derived to true interop. Until then they are **bespoke lattice schemes**, not the FIPS
   standards their names claim — the single most important finding for both the harness (§4-P2)
   and the policy (§4-P4).

### 2.2 KAT coverage inventory (what's present vs authoritative sets)

| Primitive | Present in repo | Authoritative set | Gap |
|---|---|---|---|
| SHA-512 | `""`, `"abc"`, `"abc"×1000` multi-block (`kat/vectors.rs:15-20`) | NIST CAVP SHA-512 (short/long/Monte-Carlo) | Adequate for correctness; add CAVP long+MC for rigor |
| SHA3-256 | `""`, `"abc"` (`vectors.rs:23-26`) | NIST CAVP SHA3-256 | Thin — no byte-length sweep |
| SHA3-512 | `""` only (`hash.rs:337`) | NIST CAVP | Thin |
| SHAKE128/256 | `""` only (`pq_kem.rs:705-711`) | CAVP SHAKE variable-out | Thin — no variable-length outputs |
| ChaCha20 | RFC 8439 A.1 #1-3 + §2.3.2 (`vectors_long.rs:15-54`) | RFC 8439 App A | Good |
| HChaCha20 | 1 vector (`vectors.rs:30-34`) | draft-xchacha §2.2.1 | Minimal (1) |
| Poly1305 | §2.5.2 + A.3 #1/#2 (`aead.rs:361-383`) | RFC 8439 §2.5.2 + App A.3 (multiple) | OK (2-3) |
| XChaCha20-Poly1305 | §A.3.1 + 1 committed (`aead.rs:386-428`) | draft-xchacha §A.3 | Minimal |
| **Ed25519** | RFC 8032 §7.1 **TEST 1 only** asserted inline (`sign.rs:620`); `vectors.rs:60-66` defines TEST1/2 but is **unused by tests**, and its TEST1 sig hex looks non-canonical (verify) | RFC 8032 §7.1 (7 vectors incl. 1024-byte, SHA(abc)) + **"Taming the many EdDSAs"/Chalkias edge-case vectors** + **Wycheproof EdDSA** | **Significant gap** — 1 of 7 RFC vectors; zero cofactor/malleability/edge vectors |
| **ML-KEM-768** | **No official KAT** — only self-referential dual-impl (`pq_kem.rs:846`, now both schoolbook) + FIPS 202 hash anchors | **NIST ACVP ML-KEM** + C2SP CCTV negative/unlucky-NTT vectors | **Major gap** — and non-interop means ACVP can't pass without a rewrite (§2.1) |
| **ML-DSA-65** | **No official KAT** — SHAKE256 anchors + roundtrip only (`pq_dsa.rs:664`) | **NIST ACVP ML-DSA** | **Major gap** — non-interop; can't pass ACVP as-is |
| Argon2id | RFC 9106 §5.3 (1) + BLAKE2b `""`/`"abc"` (`kdf.rs:560-589`) | RFC 9106 §5.3 (the canonical vector) | OK (the one authoritative vector) |

**Wycheproof: not referenced anywhere** (grep across `bebop2/` = 0 hits). This is the cheapest
high-value add for the interoperable set (Ed25519, ChaCha20-Poly1305; and Wycheproof now ships
ML-KEM/ML-DSA vectors — usable only after PQ interop is achieved). **ACVP: explicitly unreachable**
in the build sandbox (`pq_kem.rs:30-33`), which is *why* the PQ modules fell back to
self-referential testing — a network-access problem that made a correctness gap.

### 2.3 External-assurance menu (2026, solo/OSS)

> Web-researched (July 2026); citations are URLs.

**Professional crypto-audit firms.** No firm publishes a rate card; the only hard public
per-week number is Trail of Bits' **~$25k/engineer-week** (from their public Arbitrum ARDC
proposal, 24 eng-weeks = $600k; https://www.7blocklabs.com/blog/smart-contract-audit-cost-range-2026-and-trail-of-bits-smart-contract-audit-cost-benchmarks).
The commonly cited "$15-40k/week" band is consistent but otherwise anecdotal.

| Firm | Crypto-impl fit + directly relevant public reports | Cost signal |
|---|---|---|
| **Trail of Bits** | Dedicated crypto practice (https://trailofbits.com/services/software-assurance/cryptography/, reports https://trailofbits.com/reports/). **Audited liboqs** (2024, report Apr 2025: https://github.com/trailofbits/publications/blob/master/reviews/2025-04-quantum-open-safe-liboqs-securityreview.pdf) and OpenSSL libcrypto (via OSTIF, 9 eng-weeks, 24 findings, Alpha-Omega-funded: https://openssl-library.org/post/2024-05-02-ostif/) | ~$25k/eng-week (public) |
| **NCC Group** Cryptography Services | https://cryptoservices.github.io/about/. **Zcash FROST** (Rust threshold-sig, 3 consultants/25 person-days: https://www.nccgroup.com/us/research-blog/public-report-zcash-frost-security-assessment/); Keyfork (2024) | ~$50-90k for 25 pd (est., not published) |
| **Cure53** | Audited **rustls/ring/webpki** (2020, CNCF: https://cure53.de/pentest-report_rustls.pdf), **Monocypher** (https://cure53.de/pentest-report_monocypher.pdf), Threema Rust crypto (2022). Note: rustls maintainer spent ~2 yrs finding a sponsor (https://jbp.io/2020/06/14/rustls-audit.html) | person-days disclosed, rates not |
| **Kudelski** | ~120 public secure-code reviews, crypto-heavy Rust/Go (LitProtocol FROST Feb 2025, Uniwire TSS-ECDSA May 2025: https://kudelskisecurity.com/blockchain); own Go Kyber/Dilithium research | not published |
| Others | **Quarkslab** (Notary Project crypto audit for OSTIF 2025), **X41** (Git audit for OSTIF, 2 criticals), **Least Authority** (200+ audits, crypto-protocol focus), **Radically Open Security** (non-profit, bundled with NLnet — see below). **Zellic/Neodyme** are web3-focused → poor fit for a general library | — |

*Correction to the task premise:* **no evidence Kudelski audited PQClean or liboqs** — PQClean
states it has **never** been audited (https://github.com/PQClean/PQClean/security); liboqs's audit
was Trail of Bits. And the "NCC audited ed25519-dalek" example could not be verified (closest real
analogues: NCC's Zcash FROST, Cure53's rustls/ring).

**Realistic professional cost for this scope** (6 primitives incl. 2 PQ schemes + constant-time
review): 3-6 person-weeks ≈ **$60-150k+** at a top firm (extrapolated from the ToB rate + NCC's
FROST engagement; generic "IT security audit $3-50k" surveys are not representative of specialist
crypto review).

**Subsidized / free OSS paths — and the eligibility wall.** Every major program gates on
**adoption / criticality that a 3-day-old solo repo does not have**:
- **OSTIF** — brokers ~$1.5M/yr of audits (https://ostif.org/why-ostif/, catalog
  https://ostif.org/audits/); intake via OSTIF or LFX crowdfunding
  (https://docs.linuxfoundation.org/lfx/crowdfunding/apply-for-crowdfunding/add-a-project-for-security-audit).
  Budgets: OpenVPN ≈ $71k (https://ostif.org/openvpn-audit-updates-news-and-more/). **Targets
  widely-used critical projects — not eligible until adoption.**
- **OTF Red Team Lab / Security Lab** — free audits for internet-freedom tools
  (https://www.opentech.fund/labs/security-lab/). Caveats: must serve internet-freedom users, and
  **funding availability is uncertain** post-2025 USAGM grant-termination litigation (injunction
  won June 2025).
- **Sovereign Tech Agency (Germany) — "Sovereign Tech Resilience"** funds audits (delivered via
  OSTIF: Rails, zlib, cURL, LLVM), rolling applications
  (https://www.sovereign.tech/programs/bug-resilience). Aimed at established infrastructure —
  adoption-gated.
- **Alpha-Omega (OpenSSF)** — $5.8M/2025 but grants go to **foundations/ecosystems**
  (https://alpha-omega.dev/grants/grantrecipients/), not solo projects.
- **Google OSS VRP** — only Google-maintained OSS; **OSS-Fuzz Reward Program sunsets May 1, 2026**
  (https://bughunters.google.com/about/rules/open-source/oss-fuzz-reward-program-rules).
- **★ Most realistic open door for a new solo project: NLnet / NGI Zero** — grants (~€5k-50k) where
  grantees get a **free security audit from Radically Open Security** as a bundled support service
  (https://nlnet.nl/NGI0/services/). **NGI Zero funds on merit, not adoption** — the one eligibility
  exception found. This is the recommended first application (§4-P3).

**Community / academic / bounty.** IACR ePrint (https://eprint.iacr.org/about.html) is
**publication, not review** ("almost no refereeing"); real PQ implementation issues get triaged on
the **NIST pqc-forum** (where KyberSlash and clangover surfaced). Direct academic hire has
precedent (libsodium audit by Matthew Green's firm, PIA-funded). **Bug bounties are a poor fit
here:** huntr is now AI/ML-only; HackerOne's Internet Bug Bounty is for *established* critical OSS;
Immunefi is web3 self-funded — and bounty hunters rarely do timing/side-channel work, which is
exactly bebop2's risk surface.

**2025-2026 PQ implementation-audit reality (the failure classes an audit must target).** Recent
history shows the danger is **implementation side-channels in reference-quality code**, not the
math:
- **KyberSlash (2023-24)** — secret-dependent **division** timing in ML-KEM decapsulation
  (`poly_tomsg`/`poly_compress`) in the *reference* code and many derivatives; **key recovery in
  ~4 min**. https://kyberslash.cr.yp.to/, paper https://eprint.iacr.org/2024/1049. **bebop2's
  `pq_kem.rs:359-367 compress/decompress` divide by q=3329 on secret-derived data — the exact
  KyberSlash class.**
- **clangover / CVE-2024-37880 (2024)** — Clang compiled *constant-time* ML-KEM **source** into a
  secret-dependent branch (`poly_frommsg`); ML-KEM-512 key recovery <10 min; **invisible to
  source-level analysis**. https://github.com/advisories/GHSA-58rr-w6gv-4p4h. Lesson (applies to
  Rust/LLVM too): **verify the binary, not the source** → drives P1.4's binary-level tooling.
- **ML-DSA/Dilithium signing leaks (2025-26)** — key recovery via **rejected signatures** + NTT
  leakage (https://eprint.iacr.org/2025/214, /2025/582, /2026/056). Mostly power/EM (physical) —
  relevant to embedded, less to a server-side Rust lib, but note bebop2's ML-DSA **rejection loop**
  (`pq_dsa.rs:516-596`) is the structure these target.
- **Verified-from-scratch PQ exists**: PQCA **mlkem-native** (CBMC memory proofs + HOL-Light s2n-
  bignum, https://github.com/pq-code-package/mlkem-native), **AWS-LC** (first ML-KEM in FIPS 140-3,
  formally-verified secret-independent timing at object-code level:
  https://github.com/awslabs/aws-lc-verification), **Cryspen libcrux** (§2.4). This is the bar D1
  (§6) would be chasing.

### 2.4 Tooling menu (in-repo assurance)

> Web-researched (2026); citations are URLs.

**Test-vector suites.**
- **Project Wycheproof** — now community-run under **C2SP**: https://github.com/C2SP/wycheproof
  (old `google/wycheproof` "v0" vectors preserved at the `wycheproof-v0-vectors` tag). Coverage
  relevant here: **EdDSA/Ed25519, ChaCha20-Poly1305** (`testvectors_v1/chacha20_poly1305_test.json`),
  XChaCha20-Poly1305, HKDF/HMAC — and, importantly, **ML-KEM and ML-DSA vectors are now in**
  (`mldsa_44/65/87_verify_test.json`, added via PR #112; ML-KEM per README). **No Argon2/SHA-2
  digest vectors** — use RFC/NIST for those. Format = JSON, each case `valid`/`invalid`/`acceptable`
  + `flags` naming the attack class (https://github.com/C2SP/wycheproof/blob/main/doc/formats.md).
  Rust consumption: the **`wycheproof` crate** (https://docs.rs/wycheproof) typed structs, or
  RustCrypto's vendored `blobby` blobs (`wycheproof2blb`, https://github.com/RustCrypto/utils).
  ToB guide: https://appsec.guide/docs/crypto/wycheproof/.
- **NIST ACVP** vectors for FIPS 203/204:
  https://github.com/usnistgov/ACVP-Server/tree/master/gen-val/json-files — `ML-KEM-keyGen-FIPS203`,
  `ML-KEM-encapDecap-FIPS203` (incl. `encapsulation/decapsulationKeyCheck`),
  `ML-DSA-keyGen/sigGen/sigVer-FIPS204`. Python wrapper for the NIST golden code:
  https://github.com/mjosaarinen/py-acvp-pqc. **(Usable for bebop2 PQ only after interop — §2.1-b.)**
- **C2SP CCTV** (Community Cryptography Test Vectors) — https://github.com/C2SP/CCTV: ML-KEM
  negative-decapsulation / invalid-key vectors, the **"unlucky" NTT** vectors that force >575 XOF
  bytes in SampleNTT (~2⁻³⁸), strcmp-trap vectors; also ML-DSA + ed25519 collections.
- **Ed25519 edge cases** (the class the three-model review already cares about): RFC 8032 §7.1
  (https://www.rfc-editor.org/rfc/rfc8032#section-7.1); **"Taming the many EdDSAs"** (Chalkias et
  al., https://eprint.iacr.org/2020/1244) with vectors at
  https://github.com/novifinancial/ed25519-speccheck (small/mixed-order points, non-canonical
  encodings, cofactored vs cofactorless); **"It's 255:19AM"** + ZIP-215
  (https://hdevalence.ca/blog/2020-10-04-its-25519am/). RFC 9106 §5 Argon2, RFC 8439 §2.8.2/App A
  ChaCha20-Poly1305, draft-irtf-cfrg-xchacha App A.

**Timing-leak testing.**
- **dudect** (Reparaz/Balasch/Verbauwhede, "Dude, is my code constant time?") — black-box Welch
  t-test on two input classes, |t|>~5 ⇒ leak. Rust port **`dudect-bencher`**
  (https://github.com/rozbb/dudect-bencher, maintenance-mode but functional; **run in `--release`**,
  https://hybridkey.me/2019/04/21/rust-dudect-constant-time-crypto.html).
- **ctgrind / TIMECOP** — mark secrets uninitialized via Valgrind memcheck client requests; any
  secret-dependent branch/index is reported (agl,
  https://www.imperialviolet.org/2010/04/01/ctgrind.html; TIMECOP,
  https://appsec.guide/docs/crypto/constant_time_tool/timecop/). **MemSan variant** with Rust
  `-Zsanitizer=memory` is near-zero-integration and rides existing tests/fuzzers.
- **Binary-level** matters because **compilers introduce leaks LLVM-IR tools miss** (Binsec/Rel,
  https://arxiv.org/abs/1912.08788). `haybale-pitchfork` is effectively dormant (LLVM 9-12) — don't
  plan around it. Microsoft's SymCrypt-in-Rust uses Aeneas/Lean + an extended Revizor for
  microarchitectural CT testing (https://www.microsoft.com/en-us/research/blog/rewriting-symcrypt-in-rust-to-modernize-microsofts-cryptographic-library/).
- **Reality:** Rust/LLVM gives *no* CT guarantee — use `core::hint::black_box` + the `subtle`
  crate's `Choice` (volatile read + optimization barrier) pattern, and verify at the **binary**
  level, since source-level CT can be compiled away.

**Fuzzing / UB / bounded proofs.**
- **`cargo-fuzz`** (libFuzzer, https://github.com/rust-fuzz/cargo-fuzz), `honggfuzz-rs`, `afl.rs`;
  **`proptest`/`quickcheck`** for property + differential tests.
- **Miri** works for `no_std` (`MIRI_NO_STD=1`; https://github.com/rust-lang/miri) — catches UB in
  unsafe/index code (no FFI/inline-asm). **`cargo-careful`**
  (https://github.com/RalfJung/cargo-careful) as the cheap always-on middle ground.
- **Kani** (AWS bounded model checker, https://github.com/model-checking/kani) — best for
  panic-freedom/no-overflow/serialization-roundtrip harnesses on small bit-manip kernels (used in
  Firecracker, **s2n-quic**), *not* full-algorithm functional correctness. Crypto-grade functional
  proofs in the wild use SAW/Cryptol (aws-lc-verification) or hax→F* (libcrux), not Kani.
  **Creusot/Verus:** no mainstream crypto adoption as of 2026.

**Differential-testing precedents (templates to copy).**
- **Graviola** (rustls' native crypto, https://github.com/ctz/graviola): Wycheproof as a submodule
  + **differential fuzzing against other implementations** + formally-proven s2n-bignum assembly —
  the best template for a hand-rolled library.
- **libcrux** (Cryspen, https://github.com/cryspen/libcrux): **formally verified ML-KEM** in Rust
  (hax/F*, https://cryspen.com/post/ml-kem-verification/) — panic-freedom, correctness, **secret
  independence**. Adoption proof that verified-from-scratch PQ is real: **OpenSSH 9.9's
  `mlkem768x25519-sha256` uses C extracted from libcrux**. (rustls' PQ KX now ships via
  **aws-lc-rs** in `rustls-post-quantum`.)
- **RustCrypto**: Wycheproof blobs + direct JSON tests (e.g. the `ml-dsa` Wycheproof fix commit).
- **Oracle-based equivalence pattern** (exactly §2.5): push fuzz/proptest inputs through bebop2 *and*
  a mature oracle (`sha2`/`sha3`, `chacha20poly1305`, `ed25519-dalek`, `argon2`, and — post-interop
  — `libcrux-ml-kem`/RustCrypto `ml-kem`/`ml-dsa`), assert byte-identical — same structure Graviola
  and py-acvp-pqc use against reference code.

### 2.5 The equivalence-harness proof shape

The old-bebop crate (`crates/bebop`) already depends on the host crates bebop2 wants to replace, so
the differential oracle is **already in the dependency tree** — no new trust introduced.

**Falsifiable proof shape (per interoperable primitive):**
```
∀ inputs drawn from {KAT set ∪ Wycheproof ∪ N random seeds}:
    bebop2_core::PRIM(input)  ==  <host-crate>::PRIM(input)     (byte-exact)
  AND both agree with the official vector where one exists.
RED: mutate one byte of bebop2 output → assertion fires.
```
Oracle mapping (host crate ← bebop2 module):
- `sha2::Sha512` ← `hash::sha512`; `sha3` ← `hash::sha3_*`
- `chacha20poly1305::XChaCha20Poly1305` ← `aead::aead_xchacha20_poly1305_*`
- `ed25519-dalek` ← `sign::{keygen,sign,verify}` (incl. dalek's `verify_strict` for malleability parity)
- `argon2` ← `kdf::argon2id`

**PQ set has no valid oracle today.** `ml-kem`/`ml-dsa` crates and ACVP will disagree with bebop2
by construction (§2.1-b). The harness for {ML-KEM, ML-DSA} is therefore **blocked on a prior
decision**: either (i) re-derive them to true FIPS 203/204 interop (large effort, unlocks
ACVP + crate oracle), or (ii) declare them research-only bespoke schemes and never let them guard
value. This is Operator Decision D1 (§6).

---

## 3. Options & tradeoffs

**Option A — "Freeze as research core; keep host crates in the value path."**
Do not swap bebop2 primitives into `vault.rs`/PoD/reputation; keep `ml-kem`/`ed25519-dalek`/etc.
bebop2 stays a from-scratch learning/verification exercise. *Pro:* zero risk to value; audited
crates guard money. *Con:* the from-scratch mandate never reaches production; bebop2's reason to
exist stays unrealized. **Lowest cost, lowest ambition.**

**Option B — "In-repo assurance ladder only, then swap the interoperable set."**
Complete Wycheproof + official vectors + differential-vs-oracle + timing smoke-tests + fuzzing;
then allow the **interoperable** primitives (SHA/ChaCha-Poly/Ed25519/Argon2id) to guard value,
**keeping host crates for ML-KEM/ML-DSA** (or hybridizing so the classical/audited half always
holds). *Pro:* high assurance-per-dollar, no external spend, keeps hybrids honest. *Con:* still no
external eyes; PQ set stays on host crates (mandate partially met).

**Option C — "Full external audit before any value-bearing use."**
Ladder + a professional or subsidized external review, then swap everything. *Pro:* the only path
that credibly lets *self-written PQ* guard value. *Con:* cost + a solo/3-day-old project is not yet
audit-ready (needs interop, users, funding eligibility — §2.3).

**Recommended posture: B now, C-as-gated-goal, A for the PQ set until interop+audit.** Concretely:
adopt the in-repo ladder immediately (§4-P1), build the equivalence harness (§4-P2), keep hybrids
so the audited classical half always guards value, and treat an external audit as the **gate** the
PQ set must pass before it *alone* guards value (§4-P3/P4). This maximizes assurance-per-effort
while removing the single-point-of-failure risk (self-written PQ silently guarding escrow).

**Tradeoff summary:**

| | A Freeze | B Ladder+swap interop | C Full audit |
|---|---|---|---|
| $ cost | 0 | 0 | see §2.3 |
| Effort | ~0 | weeks | ladder + months (audit cycle) |
| Mandate met | no | partial | yes |
| PQ guards value alone | never | never (hybrid) | after audit |
| Residual risk | none (uses audited) | low (hybrid + differential) | lowest |

---

## 4. Recommended execution blueprint (phased)

> Each step: **action / gate marker / VbM proof / effort.** Gate markers reuse the repo's own
> vocabulary: `KAT-green`, `RED+GREEN`, three-model review (`AGENTS.md §1`), `reloop` empty-import.
> Effort in solo-days (design-through-green), rough.

### Phase 1 — In-repo assurance ladder (no external spend)

**P1.1 Complete official-vector coverage.**
- *Action:* add NIST CAVP short/long/Monte-Carlo for SHA-512/SHA3/SHAKE; full RFC 8032 §7.1 (all 7)
  for Ed25519; wire the already-defined-but-unused `vectors.rs` ED25519 const (and fix its
  suspect TEST-vector hex); more Poly1305/AEAD vectors.
- *Gate:* `KAT-green` extended set; three-model review of the added vectors.
- *VbM proof:* each new vector asserts byte-exact; **RED** = flip one expected byte → test fails.
- *Effort:* 1-2 d.

**P1.2 Add Project Wycheproof for the interoperable set.**
- *Action:* consume Wycheproof JSON (via the `wycheproof` crate or vendored JSON) for Ed25519 and
  ChaCha20-Poly1305 — includes malleability, small-order, non-canonical, edge-nonce cases.
- *Gate:* `KAT-green` (Wycheproof "acceptable/invalid" flags honored).
- *VbM proof:* every `"invalid"` case must be **rejected**, every `"valid"` **accepted**; **RED** =
  force-accept an invalid case → suite fails. This is the falsifiable teeth that plain KATs lack.
- *Effort:* 2-3 d (harness + triage of expected-fail edge cases; may surface real bugs — that's the point).

**P1.3 Differential/property harness vs oracle crates (interoperable set).**
- *Action:* `proptest`/`quickcheck` generating random inputs; assert bebop2 == host crate byte-exact
  (mapping in §2.5). Lives in a **test-only harness crate** so `bebop2-core` keeps zero deps.
- *Gate:* `RED+GREEN` per `AGENTS.md §2`; three-model review.
- *VbM proof:* N≥10⁴ random cases agree with oracle; **RED** = inject a one-bit mutation into a
  bebop2 output copy → divergence detected. Include Ed25519 `verify` vs dalek `verify_strict`.
- *Effort:* 3-4 d.

**P1.4 Timing-leak smoke tests (dudect-style) on the secret-dependent hotspots.**
- *Action:* dudect/`dudect-bencher`-style two-class timing test targeting the §2.1 hotspots:
  Ed25519 `scalar_mul` (secret scalar), ML-KEM `poly_mul` + `compress` (KyberSlash class),
  ML-DSA `poly_mul_schoolbook`. Under `--release` with `black_box` barriers.
- *Gate:* new `timing-smoke` gate (advisory, not blocking) — documents measured leakage.
- *VbM proof:* Welch t-test |t| statistic reported; **RED** = a known-leaky reference (e.g. an
  early-exit `memcmp`) must trip the detector, proving the harness can see a leak. Note honestly:
  a *green* dudect run is **not** proof of constant-time (LLVM can compile CT away; dudect is
  best-effort) — it is a **leak-finder**, and these hotspots are expected to **fail** it today.
- *Effort:* 3-5 d (tooling is finicky; expect it to confirm the known Ed25519/Kyber leaks).

**P1.5 Fuzzing (parsers/decoders).**
- *Action:* `cargo-fuzz`/libFuzzer targets on the attacker-facing decoders: `point_decompress`,
  Ed25519 `verify`, ML-KEM `decaps`/`byte_decode`, AEAD `decrypt`, Argon2 param parsing. Add `Miri`
  run on the test suite for UB.
- *Gate:* `fuzz-clean` (N hours no crash) + `miri-clean`.
- *VbM proof:* corpus + zero crashes/UB over a fixed budget; **RED** = a deliberately OOB index
  in a scratch branch is caught by the fuzzer/Miri.
- *Effort:* 2-4 d.

### Phase 2 — kernel/cli/reloop + old-bebop-as-oracle equivalence

**P2.0 (Decision gate D1 — see §6): choose PQ interop path.** Everything below for {ML-KEM,ML-DSA}
is blocked on D1. The interoperable set proceeds regardless.

**P2.1 Build the missing `bebop2/{kernel,cli,reloop}` scaffolding** (audit §6.5: 0%).
- *Action:* create the three dirs per `README.md:45-48`; `reloop` first (it is the verification
  harness: execute wasm bit-exact + empty-import gate).
- *Gate:* `reloop` empty-import assertion green (README build/verify §).
- *VbM proof:* wasm artifact import section is empty (grep/`wasm-objdump`); **RED** = introduce a
  `getrandom` call → import appears → reloop fails.
- *Effort:* 3-5 d (reloop), + CLI later.

**P2.2 Equivalence harness vs old-bebop oracle (`crates/bebop`).**
- *Action:* deliver the `cargo test -p bebop2` equivalence suite `README.md:64` promises: same
  inputs → old-bebop (host-crate-backed) vs bebop2-core, for the **interoperable set**. This is the
  §2.5 differential harness wired against the actual old-bebop API surface (vault sign/verify,
  AEAD, KDF), not just raw crates.
- *Gate:* `RED+GREEN` + three-model review.
- *VbM proof:* per primitive, old==new byte-exact across KAT∪Wycheproof∪random; **RED** = mutate.
- *Effort:* 4-6 d.

**P2.3 (only if D1 = "re-derive to interop") — true FIPS 203/204 ML-KEM/ML-DSA.**
- *Action:* re-derive a correct constant-time NTT (the removed one was buggy, `pq_kem.rs:301`);
  store `ek/dk` in NTT domain; fix ML-DSA A-sampling to uniform RejNTTPoly, 48-byte challenge,
  standard packing. Only then can ACVP + `ml-kem`/`ml-dsa` crates be oracles.
- *Gate:* **NIST ACVP KAT-green** + differential vs `ml-kem`/`ml-dsa` crates + three-model review.
- *VbM proof:* ACVP vectors pass byte-exact; **RED** = any ACVP expected byte flipped fails.
- *Effort:* large — 15-25 d, and still not side-channel-audited. This is the "is the mandate worth
  it" fork (D1).

### Phase 3 — External assurance path

**P3.1 Prereqs (make the project audit-ready).** Land P1 + P2 (interop set green, harness, fuzz,
docs). External auditors and OSS-audit funders both expect a stable, documented, adopted target —
a 3-day-old solo repo with no users is not yet eligible (§2.3). Get the primitives interoperable
(so an auditor can diff against references) and get *some* adoption/story.

**P3.2 Choose an assurance tier (Operator Decision D2, §6):**
- *Subsidized/free (recommended first):* apply to an OSS-audit funder (OSTIF / Sovereign Tech
  Fund / OTF) — they broker + fund a professional firm for eligible OSS. Effort = application +
  cycle; $ = 0 to the project. See §2.3 for eligibility realities and citations.
- *Community/academic:* IACR ePrint note on the from-scratch PQ schemes + targeted bug bounty for
  the interoperable set. Low $, variable coverage; good for the classical set, weak for
  implementation side-channels.
- *Professional (paid):* engage a crypto-review firm (Trail of Bits / NCC / Cure53 / Kudelski / …)
  for a scoped review of the interoperable set (and PQ only if interop-complete). See §2.3 for
  cost ranges + citations. This is the only tier that credibly clears self-written PQ for
  solo-guard-value use.
- *Gate:* published audit report with findings remediated + re-review.
- *VbM proof:* each audit finding → a RED+GREEN regression test committed; report references them.
- *Effort:* application/scoping weeks + audit cycle (weeks-months) + remediation.

### Phase 4 — Value-bearing policy (ready-to-adopt text)

**P4.1 Adopt the tiered policy below** (Operator Decision D3, §6). Wire it as a doc-claims gate
(the repo already has `scripts/verify-doc-claims.mjs`, `AGENTS.md §2`) so the tier of any primitive
is machine-checked before it can back a value path.

> ### bebop2 Cryptography Value-Bearing Policy (v1, proposed 2026-07-11)
>
> Every bebop2 primitive carries exactly one **assurance tier**. A primitive may only be used at or
> below the guard-level its tier permits. Tiers are additive gates; a primitive advances only when
> the named gate is green and recorded (three-model review + committed RED+GREEN proof).
>
> **Tier 0 — RESEARCH-USE ONLY.** Default for every from-scratch primitive. May be used for
> experiments, benchmarks, and non-value test vectors. **MUST NOT** encrypt, sign, or authenticate
> anything a user relies on. *(All of bebop2-core is Tier 0 today.)*
>
> **Tier 1 — SIGNED-ARTIFACTS / INTEGRITY-ONLY.** Requires: (a) official-vector KAT-green, (b)
> Project Wycheproof green (invalid-cases rejected), (c) differential parity byte-exact vs the
> audited host crate over KAT∪Wycheproof∪≥10⁴ random, (d) fuzz-clean + Miri-clean on decoders, (e)
> three-model review. A Tier-1 primitive MAY protect **non-adversarial-value** integrity
> (local-at-rest MACs, content-address hashes, reproducible-build signatures) where a break is
> recoverable and not directly monetizable. **MUST NOT** solely guard money/escrow/identity.
>
> **Tier 2 — VALUE-BEARING (hybrid-guarded).** Tier 1 + (f) timing-smoke evidence with the known
> hotspots either constant-timed or accepted-and-documented, and (g) the primitive is used **only
> as one half of a hybrid** whose other half is an externally-audited implementation (per
> `vault.rs`'s existing PQ⊕classical design). A break in the self-written half cannot alone forge
> PoD / drain escrow because the audited half still holds. This is the **maximum tier a
> non-externally-audited bebop2 primitive may reach.**
>
> **Tier 3 — VALUE-BEARING (sole guard).** Tier 2 + (h) a completed external security audit
> (professional or funder-brokered) of that primitive with all findings remediated and
> re-reviewed, and (i) for PQ schemes, NIST-ACVP interoperability. Only a Tier-3 primitive may be
> the **sole** cryptographic guard of real value. **No bebop2 primitive is Tier 3 today; ML-KEM-768
> and ML-DSA-65 cannot reach Tier 1 until they are FIPS-interoperable (§2.1-b), and therefore may
> only ever appear as the PQ half of a Tier-2 hybrid until D1+audit.**
>
> **Enforcement.** The tier of every primitive is declared in-repo and checked by the doc-claims
> gate. Raising a value path to use a primitive above its tier fails the pre-commit gate. Red-line
> per `AGENTS.md §3` (crypto-constant / auth / money changes) still requires human confirmation.

---

## 5. Risks

- **False confidence from green KATs.** The whole gap. KAT-green hides the §2.1 timing leaks and
  the PQ non-interop. Mitigation: Wycheproof + differential + dudect make the hidden failures
  *falsifiable* (Phase 1).
- **PQ non-interoperability is silent.** ML-KEM/ML-DSA look done (green tests) but are bespoke
  schemes; anyone assuming FIPS interop (e.g. talking to a standard peer, or trusting the FIPS
  security proof) is wrong. Mitigation: policy Tier gating + D1.
- **dudect green ≠ constant-time.** LLVM can erase CT constructs; a passing timing test is
  best-effort, not proof. Mitigation: frame P1.4 as leak-finding, keep hybrids (Tier 2).
- **Two Keccak implementations** (`hash.rs`, `pq_kem.rs`) can diverge under future edits.
  Mitigation: consolidate; cross-test both against CAVP.
- **Bus factor 1, uncommitted crown jewels** (audit §6.1/6.5: `kdf.rs`/`pq_dsa.rs` uncommitted).
  A lost working tree loses Argon2id + ML-DSA. Mitigation: commit through the three-model gate
  (operator action, outside this read-only task).
- **Audit-readiness gap.** A 3-day-old solo repo is not yet eligible for most OSS-audit funding or
  a productive paid audit. Mitigation: Phase 1+2 first; treat Phase 3 as gated.
- **Scope gravity** (audit §7.10): the ladder itself is large; risk of it becoming another
  unfinished layer. Mitigation: the interoperable set (Option B) delivers most of the value in
  ~2 weeks and is independently shippable; do it first, decide D1 later.
- **Review-philosophy fork** (proxy 3-model review here vs purged in dowiz) — orthogonal to G09 but
  note the three-model gate is load-bearing for every gate marker above.

---

## 6. Operator decision points

- **D1 — Do the PQ primitives get re-derived to FIPS-203/204 interoperability?**
  - *Yes:* unlocks ACVP + `ml-kem`/`ml-dsa` differential oracle + a future path to Tier 3; costs
    ~15-25 d (P2.3) plus audit. The from-scratch PQ mandate becomes real.
  - *No:* ML-KEM/ML-DSA stay bespoke, permanently capped at "PQ half of a Tier-2 hybrid"; the
    audited host crates remain the interop/standard-facing path. Cheaper, honest, mandate partial.
  - *Precedent:* verified-from-scratch PQ *is* achievable — Cryspen's **libcrux** ML-KEM is formally
    verified (hax/F*, incl. secret-independence) and its extracted C ships in **OpenSSH 9.9** — but
    that took a formal-methods team, not a solo dev, which is exactly why D1's honest cost is high.
  - *Recommendation:* **No for now** — ship the interoperable set to Tier 2, keep host-crate PQ,
    revisit D1 only if the delivery protocol actually needs standard-interop PQ on the wire.

- **D2 — Spend money on an external audit? (or pursue subsidized/community first?)**
  - *Recommendation:* **Do not pay yet.** Land Phase 1+2, then apply to an OSS-audit funder
    (OSTIF/STF/OTF) — $0 to the project if eligible. Reserve a paid firm engagement for the moment
    a self-written primitive genuinely needs to be a **sole** value guard (Tier 3), which the
    hybrid design lets you avoid. Costs/citations in §2.3.

- **D3 — Adopt the value-bearing policy (§4-P4) as a binding gate now?**
  - *Recommendation:* **Yes, adopt v1 immediately.** It costs nothing, encodes the honest status
    (everything Tier 0 today), and prevents the actual harm (a future edit quietly routing escrow
    through a Tier-0 self-written primitive). Wire it into `verify-doc-claims.mjs`.

- **D4 — Commit the uncommitted crown jewels** (`kdf.rs` Argon2id, `pq_dsa.rs` ML-DSA-65) through
  the three-model gate? *Recommendation:* **Yes** — bus-factor-1 + uncommitted is the highest-harm,
  lowest-effort item; but it is an operator action (this task is read-only).

---

*Scoping only — not an audit. `file:line` references are entry points for a real audit/harness, not
a claim of exhaustive coverage. No files in `bebop-repo` or `dowiz` were modified.*
