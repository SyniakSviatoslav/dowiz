# CRYPTO-P0-REMEDIATION — 2026-07-16

**Unit:** W2-8 crypto P0 remediation VERIFICATION (RED-LINE: verify + report only)
**Scope:** `/root/bebop-repo` (OpenBebop) + `/root/dowiz` (OpenBebop operator/UI)
**Operator gate:** READ-ONLY on all code except this report. No commit. No modification of auth/money/crypto source.
**Method:** real `search_files` (ripgrep) greps + Cargo.lock inspection. No source files modified.

---

## VERDICT: CLEAN (no P0 crypto red-line violation found)

All three P0 hypotheses (H-1..H-3) are refuted by evidence. No `openssl-sys`/`native-tls`
in either Cargo.lock; no textbook RSA / ECB / AES-CBC cipher usage in source; no
SHA1/MD5 in any security path (only SHA3/Blake in the crypto tree; stray `sha1` crate
is a transitive build-tool dep; `sha1`/`md5` doc mentions are non-security).

---

## H-1 — textbook RSA / ECB / AES-CBC anywhere? NO.

**Evidence:**
- `Cargo.toml` dependency scan (both repos): zero `aes`, `rsa`, `block-modes`, `ecb`,
  `aes-gcm`, `cbc` crate entries. Result: **0 matches** in `bebop-repo` and `dowiz`.
- `*.rs` source scan for `ecb|aes::|Aes\d+|.cbc(|block_encrypt|rsa::|Rsa` in `bebop-repo`:
  3 hits, all benign:
  - `bebop2/proto-crypto/src/pq_kem.rs:607-608` → `REF1_PK`/`REF1_SK` =
    ML-KEM **Known-Answer-Test vectors** (hex strings), not a cipher call.
  - `bebop2/core/src/hash.rs:95` → `0x4cc5d4becb3e42b6` is a SHA3 constant literal
    (substring "ecb" inside the hex), not a cipher.
- `*.rs` source scan in `dowiz`: **0 matches**.
- `RSA` string hits in `bebop-repo` are in `crates/bebop/src/research_patterns.rs`
  (a *secret scanner* that detects `-----BEGIN RSA PRIVATE KEY-----` strings in user
  text — a detection pattern, not a crypto implementation).
- `AES-CBC` mention in `bebop-repo/docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md:57`
  is a *prose research note* describing a 3rd-party scheme ("optional AES-CBC-256 outer"),
  explicitly superseded in the same doc by bebop's audited ML-DSA+Ed25519+SHA512 stack.
  Not code, not in the crypto tree.

**Conclusion:** No textbook RSA, no ECB mode, no AES-CBC usage. The post-quantum
substrate (ML-DSA / ML-KEM / Ed25519 / SHA3 / hybrid gate) is intact and KAT-gated.

## H-2 — all secrets use ring/rustls (no openssl/native-tls)? YES.

**Evidence — bebop-repo:**
- `Cargo.lock` scan for `name = "openssl-sys" | "native-tls" | "openssl"`: **0 matches**.
- Only `openssl-probe` appears (line 2074) — a harmless build-time env probe,
  not a crypto backend (it is not `openssl`/`openssl-sys`).
- `deny.toml` (lines 43-45) bans `openssl-sys` and `native-tls` as a property gate;
  RED fixture `scripts/fixtures/deny-bans-openssl.conf` exists.
- `bebop2/proto-wire` uses `quinn`+`rustls`+`tokio-rustls`+`rustls-platform-verifier`
  (pure-Rust TLS 1.3); `Cargo.toml` comment confirms native-tls feature deliberately off.

**Evidence — dowiz:**
- `dowiz` Rust tools depend on `ring` + `rustls` only:
  - `tools/async-spool/Cargo.toml` (rustls + webpki-roots/ring; "NO OpenSSL, NO native-tls")
  - `tools/telemetry/rust-spool/Cargo.toml` (rustls+ring)
  - `tools/native-spa-server/Cargo.toml` (tokio-rustls + rustls-pemfile)
- `Cargo.lock` scan across dowiz (`name = "openssl-sys|native-tls|openssl|ring|rustls"`):
  only `ring` + `rustls` present; **no openssl/native-tls**.

**Conclusion:** No OpenSSL/native-tls anywhere. Secrets/transport ride ring/rustls.

## H-3 — no SHA1/MD5 for security (only SHA3/Blake in tree)? NO SECURITY USE.

**Evidence — bebop-repo:**
- `Cargo.lock` contains `sha1 0.10.7` (line 2973) pulled only by `tungstenite 0.23.0`
  (line 3584) — a WebSocket framing dependency, NOT a security/auth path.
- `bebop2/ports/github/src/lib.rs:475-476` (`malformed_signature_scheme_rejected`
  test) **rejects** GitHub's deprecated `X-Hub-Signature: sha1=...` and requires
  `sha256=` only — i.e. it explicitly disallows SHA-1 in the security path.
- `scripts/logic-gate.mjs:66` uses `createHash('sha1')` to truncate a *claim-id string*
  for a logic-gate state file — a non-security checksum, not a security primitive.
- No `md5` crate/deps. No SHA1/MD5 in proto-crypto (SHA3/Blake/subtle only).

**Evidence — dowiz:**
- `docs/phase5/anonymizer.md:71` uses `md5(random()::text)` to build an *anonymized*
  phone placeholder string (GDPR erasure, irreversible pseudonym, not auth/integrity).
  Not a security-path use of MD5.
- `docs/...key-rotation.md`, `.env.example`, `APPLY.md` show `openssl genrsa` / `genpkey`
  commands only as **key-generation shell instructions** for the operator's JWT RS256
  keys — CLI tooling guidance, not library code compiled into the tree.
- No `md5`/`sha1` crate in any dowiz `Cargo.lock`; no `createHmac('sha1'|'md5')`/
  `RS256`/`jsonwebtoken`/`jwt.sign` in any TS/JS source.

**Conclusion:** No SHA1/MD5 is used for authentication, integrity, or signing. The
sole `sha1` crate is a transitive WebSocket-framing dep; doc mentions are either
non-security (anonymization, logic-gate id) or explicit rejection (github port) or
operator CLI guidance (key gen).

---

## NON-BLOCKING NOTES (informational, outside P0 scope)
- The `MaybeTlsStream::Plain` / plaintext-`ws://` transport concern (red-team H6/H8,
  `bebop2/proto-wire`) is a *transport confidentiality* issue, not a P0 *crypto
  primitive* violation (no banned cipher/TLS backend is present in the lock). It is
  tracked separately under the rustls-migration work and is outside this unit's scope.
- `openssl` command-line invocations in dowiz docs (`.env.example`, `key-rotation.md`,
  `APPLY.md`) are operator key-gen guidance, not in-repo native-tls/openssl-sys code.
  If the operator wants a pure-Rust key-gen path (e.g. `rcgen`/Ed25519), that is a
  separate hygiene task, not a P0 red-line.

## GREP EVIDENCE RUN (reproducible)
```
# H-2 openssl/native-tls in locks
rg 'name = "(openssl-sys|native-tls|openssl)"' /root/bebop-repo/Cargo.lock   -> 0
rg 'openssl-sys|native-tls|openssl' /root/dowiz  (source)                    -> docs/cli-only, 0 in lock
rg 'name = "(openssl-sys|native-tls|openssl|ring|rustls)"' /root/dowiz/**/Cargo.lock -> ring,rustls only

# H-1 textbook RSA/ECB/AES-CBC
rg -g '*.toml' '^\s*(aes|rsa|block-modes|ecb|aes-gcm|cbc)\s*=' /root/bebop-repo /root/dowiz -> 0
rg -g '*.rs'  'ecb|aes::|Aes\d+|.cbc(|block_encrypt|rsa::|Rsa' /root/bebop-repo -> 3 (all benign KAT/const)
rg -g '*.rs'  'ecb|aes::|Aes\d+|.cbc(|block_encrypt|rsa::|Rsa' /root/dowiz -> 0

# H-3 SHA1/MD5 security
rg 'sha1|md5|Sha1|Md5' /root/bebop-repo  -> sha1 crate (tungstenite), github-port rejects sha1, logic-gate id
rg 'md5|sha1' /root/dowiz  -> anonymizer pseudonym (non-security), docs cli-only
```

**FINDING:** none (no real P0 red-line violation to escalate/stop on).
**STOP/ESCALATE:** not triggered — verification complete, no operator-gate breach.
