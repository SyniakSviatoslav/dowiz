# Research + real build attempt — Flipper Zero (flipperzero-rs) NFC for dowiz PoD

**Date:** 2026-07-18 · **Type:** exploratory research + real attempt log (NOT a committed roadmap phase — one step below a blueprint) · **Scope:** proof-of-delivery (PoD) NFC + hardware device-identity.

**Honesty constraint up front:** there is **no physical Flipper Zero** in this environment. Nothing below was flashed or tapped on real hardware. Every "it works" claim is a real host-side toolchain/compile/test result, and every hardware-dependent step is marked as such.

---

## What was attempted, and the real results

### Step 1 — Toolchain setup → WORKED

```
$ rustup target add thumbv7em-none-eabihf
info: downloading component rust-std
EXIT=0
$ rustup target list --installed | grep thumb
thumbv7em-none-eabihf
```

- Host toolchain: `rustc 1.96.1 (stable)`, `cargo 1.96.1`. Toolchains available: `stable` + `1.78` only — **no nightly, no `rust-src`**.
- The embedded target installed cleanly and pulled a **precompiled `core`/`alloc`** for `thumbv7em-none-eabihf`, so `-Z build-std` (which would need nightly + rust-src) is **not** required.
- Network nuance: the crates.io **HTML front returns 403** in this sandbox, but the **cargo registry index + crate download work fine** (`cargo info`, `cargo build` both fetch), and `github.com` / `raw.githubusercontent.com` are reachable (200).

### Step 2 — Real build against flipperzero-rs → WORKED (compile + link on stable)

Current crates.io versions (verified live, not guessed): **`flipperzero`, `flipperzero-sys`, `flipperzero-rt` all at `0.16.0`** (MIT, `rust-version = 1.85.0`).

Project: `tools/nfc-pod-flipper/` — a `no_std` FAP with the template's `.cargo/config.toml` (Cortex-M4, `panic=abort`, relocatable link against `flipperzero-rt.ld`) and a minimal NFC read skeleton.

**Key toolchain finding:** the upstream `flipperzero-rs/flipperzero-template` pins `nightly-2025-08-31`, but **only** to use the unstable `cargo-features = ["different-binary-name"]` — a cosmetic step that renames the output to `<name>.fap`. It is **not** a compile requirement. Dropping that one line, the real compile+link runs on **stable**:

```
$ cargo build --release            # target set to thumbv7em-none-eabihf via .cargo/config.toml
   Compiling flipperzero-sys v0.16.0
   Compiling flipperzero v0.16.0
   Compiling nfc-pod-flipper v0.1.0
    Finished `release` profile [optimized] target(s) in 5.94s
EXIT=0
```

Output artifact inspected (`target/thumbv7em-none-eabihf/release/nfc-pod-flipper`, 4216 bytes):
- Relocatable ARM ELF with a **`.fapmeta`** section (the FAP manifest emitted by the `manifest!` macro + linker script) + `.text` + `.rel.text` relocations — a proper partial link, the pre-`.fap` object.
- NFC symbols resolve as expected: `nfc_alloc`, `nfc_free`, `nfc_poller_alloc` are `U` (undefined) — resolved at **FAP-load time** against the firmware's Furi API, which is correct for a FAP.
- Confirmed the shipped `flipperzero-sys` 0.16.0 `bindings.rs` (27,501 lines, pre-generated bindgen) contains the **full NFC/ISO14443 poller API** (`iso14443_3a_poller_sync_read`, `nfc_poller_start/detect/trx`, `nfc_scanner_*`, listener/emulation symbols, 605 nfc lines). So read AND emulate are reachable from Rust.

**What was NOT done (honest gaps):** (a) the final `.fap` packaging step (`flipperzero-tools` / the nightly `different-binary-name` rename) was skipped — the compile+link is the load-bearing test; (b) the NFC **poller callback** (the code that reads bytes off a real tag on `NfcPollerEventTypeReady`) is left as an explicitly-marked device seam, **not faked**, because it cannot be exercised without hardware; (c) nothing was flashed or run.

**Firmware-compatibility note (Momentum vs mainline — do NOT assume):** flipperzero-rs 0.16.x targets **API 87.1 = mainline flipperzero-firmware 1.3.4 / 1.4.3**. Momentum is a *superset fork*, generally rebased on mainline `dev`. FAPs are ABI-versioned by API version, so an 87.x FAP *typically* loads on a current Momentum build — but this is **not guaranteed** across arbitrary Momentum releases and must be checked against that build's `api_symbols.csv`. Momentum's own NFC Maker App / NTAG4xx additions are firmware-side and not required by this Rust app.

### Step 3 — Hardware-independent NDEF codec → WORKED (17/17 tests pass)

Crate: **`tools/nfc-pod-codec/`** (`std`, server-side). Reuses the kernel's **audited SHAKE256** (`dowiz_kernel::pq::keccak::shake256`, FIPS 202) as the MAC primitive — **no new crypto invented**, same `SHAKE256(key ‖ context)` shape the kernel already uses in `keccak::prf`/`xof_j`. Depends on the kernel via path + `features = ["pq"]` (the known-green pq-KAT build).

- **NDEF layer** (`src/ndef.rs`): single short record, TNF = 0x04 (NFC Forum **External Type**), type name `dowiz.io:pod`. Implemented against the **NFC Forum "NDEF" Technical Specification v1.0, §3 (NDEF Record)** — cited in-file. Rejects chunked / long / IL / wrong-TNF / truncated / type-mismatch records.
- **PoD payload** (`src/pod.rs`): `version(1) ‖ order_id(u64 BE) ‖ issued_at(u64 BE) ‖ mac(16)` = **33 bytes**. `order_id` is `u64`, bound to the real kernel event vocabulary (`bebop2/proto-cap/src/event_dict.rs` → `OrderPlacedPayload { order_id: u64, .. }`), big-endian like `event_dict`'s `put_u64`. MAC = `SHAKE256(key ‖ "dowiz/pod-mac/v1" ‖ fields)[..16]`, xor-accumulate compare (no early-exit timing leak).
- Full tag = 3 (NDEF hdr) + 12 (type) + 33 (payload) = **48 bytes → fits an NTAG213 (144 B) comfortably**.

```
$ cargo test          # in tools/nfc-pod-codec
   Compiling dowiz-kernel v0.1.0 (pq feature)
   Compiling nfc-pod-codec v0.1.0
test result: ok. 17 passed; 0 failed; 0 ignored
EXIT=0
```

Tests cover: NDEF round-trip; truncated header; truncated/lying payload length; wrong TNF; type mismatch; chunked-flag reject; PoD round-trip verify; tampered order_id → `BadMac`; tampered timestamp → `BadMac`; wrong key → `BadMac`; bad length; bad version; distinct-orders-distinct-MACs; full tag round-trip; tampered-tag reject; malformed-NDEF-header reject. All green.

---

## Step 4 — Verdict (from the evidence above, not decided in advance)

### Flipper Zero as PRODUCTION hardware for PoD / device-identity — **NO.**
Nothing in steps 1–2 changed the cost/universality calculus; it only confirmed the device *can* read the format. A ~$170 device per courier/venue to read a tag that **any modern phone already reads** (NDEF is a phone-native capability) is not defensible for a food-delivery MVP. The build itself surfaced a second disqualifier: a real public-key device signature (ML-DSA-65, **3309 B**) does not even fit a passive tag — the size-appropriate on-tag proof is a **symmetric MAC**, which the Flipper doesn't make more secure than a phone does.

**Recommended production mechanisms (unchanged — evidence supports them):**
- **PoD:** cheap passive **NTAG213/215/216** tags (~$0.10–0.50 each), written once by a server-side provisioning service using `nfc-pod-codec::encode_tag`, read by **any courier phone** (Web NFC / native NDEF). Verification is **server-side** (the phone/Flipper never holds the provisioning key), which the codec's trust model already reflects.
- **Hardware-bound device identity:** **WebAuthn/FIDO2** (purpose-built, ~$30 YubiKey-class, or platform authenticators already in every phone) — the industry-standard answer, and orthogonal to the tag. This belongs to the auth surface, not the tag.

### Flipper Zero + flipperzero-rs as a DEV-TOOLING aid — **YES, low-friction, worth adopting (optional, single shared device).**
The build genuinely worked on **stable**, in seconds, with **zero exotic setup** (one `rustup target add`; the only nightly requirement in the upstream template is a cosmetic `.fap`-rename flag that we dropped). A **single ~$50–170 Flipper** (one per team, not per courier) becomes a reusable, portable NFC **read / write / emulate** harness to author and validate the PoD NDEF format and the courier-tap read flow **before** any production stickers or courier phones exist. The `no_std` app is architecturally consistent with the repo's own kernel direction. This is a genuine, evidence-backed dev-tooling recommendation — *not* a committed phase.

**Cost-honest caveat:** the same validation can be done with a **$3 phone NFC-writer app + a pack of NTAG213 stickers**, which is cheaper and closer to production reality. The Flipper's marginal advantage is **tag *emulation*** (test the read flow with no physical sticker at all) and being a single durable bench tool. Adopt it only if that emulation/portability is actually wanted; otherwise the phone-writer path is sufficient. Either way, the **`nfc-pod-codec` crate is the real deliverable** and stands on its own regardless of which reader is used.

---

## Files created (all under `tools/`, no collisions with concurrent work)

| Path | What | Status |
|---|---|---|
| `tools/nfc-pod-codec/Cargo.toml` | server-side codec crate (own workspace, kernel `pq` dep) | builds |
| `tools/nfc-pod-codec/src/ndef.rs` | NDEF v1.0 single short external-record codec | tested |
| `tools/nfc-pod-codec/src/pod.rs` | PoD payload + SHAKE256 keyed MAC (reuses kernel) | tested |
| `tools/nfc-pod-codec/src/lib.rs` | `encode_tag` / `decode_and_verify_tag` + tests | **17/17 pass** |
| `tools/nfc-pod-flipper/Cargo.toml` | Flipper FAP crate (stable-compatible) | builds |
| `tools/nfc-pod-flipper/.cargo/config.toml` | template linker/target config (Cortex-M4, relocatable) | — |
| `tools/nfc-pod-flipper/src/main.rs` | NFC read skeleton + no_std NDEF/PoD decode | **compiles+links** |

---

## Proposed addenda for the lead to apply (NOT applied here — avoid collision)

> These are text proposals only. `BLUEPRINT-P52-courier-working-surface.md` and
> `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` were **intentionally not touched** — a
> concurrent task may be editing P52. The lead should apply after checking for collisions.

**Addendum candidate → `BLUEPRINT-P52-courier-working-surface.md` (PoD section):**
> Proof-of-delivery uses passive **NTAG213/215/216** tags carrying a dowiz NDEF
> External-Type record (`dowiz.io:pod`), encoded/verified by the tested
> `tools/nfc-pod-codec` crate (`order_id: u64 + issued_at + 16-byte SHAKE256 MAC`,
> reusing `kernel::pq::keccak`). Couriers tap with their **phone** (Web NFC / native
> NDEF); the MAC is verified **server-side** (no key on the device). **Optional dev
> tooling:** `tools/nfc-pod-flipper` is a flipperzero-rs FAP (builds on stable) that
> reads the same format on a single shared Flipper Zero for pre-production validation
> — a bench aid, explicitly **not** production courier hardware.

**Addendum candidate → `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (device-identity section):**
> Hardware-bound device identity is **WebAuthn/FIDO2** (platform authenticators + optional
> ~$30 FIDO2 keys), not NFC tags and not Flipper Zero. Rationale recorded in
> `RESEARCH-NFC-FLIPPER-ZERO-DEV-TOOLING-2026-07-18.md`: a passive tag cannot hold a
> public-key signature (ML-DSA-65 = 3309 B ≫ NTAG213's 144 B), so the tag layer is a
> symmetric-MAC PoD artifact, deliberately distinct from the public-key auth layer.
