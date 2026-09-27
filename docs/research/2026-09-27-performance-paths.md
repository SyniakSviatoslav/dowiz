# Max-performance paths (brief C) against dowiz's real deployment: unikernels, kernel bypass, io_uring, native AOT

Research, 2026-09-27. Lane W-RESEARCH, READ-ONLY. Tree `/root/lanes/w-research` at `5b77de64`; every code claim
carries a file:line read on this date. Labels: **MEASURED** (a command here, or a dated measurement document
quoted with its section), **DOC** (a vendor or standards page fetched 2026-09-27), **EST** (arithmetic on named
inputs), **HYPOTHESIS** (not measured). The brief's four items are hypotheses and are tested against where
dowiz actually runs, not against where they would shine.

Companions: `2026-09-27-dag-architecture.md` (what actually cuts CPU), `2026-09-27-sel4-wasm.md` (the Box),
`2026-09-27-binary-size.md`.

---

## 0. The answer in eight lines

1. **dowiz runs in a V8 isolate as a wasm32 module, with no OS, no threads and no CPU of its own** (`workers/api/wrangler.toml`
   `main = "build/worker/shim.mjs"`, `[build] command = "worker-build --release"`; DOC: "Threading is not possible in
   Workers. Each Worker runs in a single thread"). Unikernels, DPDK/SPDK, io_uring and `-C target-cpu=native`
   all presuppose owning a kernel, a NIC, a disk or a CPU. **None of the four applies to the deployment that
   serves customers today.**
2. **Where they could apply is the Box** (`tools/native-spa-server`, a static Rust binary; `Dockerfile` DK-08
   "the artifact is a single static binary"), which serves ONE restaurant at ≈ 30 orders/day. Its latency is the
   phone's radio and the internet; its I/O is a few kilobytes per order. Kernel bypass on it is a spinning core
   heating a fanless box for nothing (§2.2).
3. **Measured latency structure** (`docs/design/BLUEPRINT-OPTIMIZATION-AND-EVALS-2026-09-24.md` §A.1): edge ≈ 50 ms,
   Worker + warm object ≈ 105–135 ms, cold object ≈ 260 ms; Worker CPU p50 2.7 ms (FREE-TIER §1.2). Compute is
   ≈ 2–3 % of what a user waits for (EST: 2.7 / 105).
4. **The CPU that hurts is not arithmetic.** The 622 kills at 10,000 µs come from whole-image pulls (538 KB
   catalogue via 40 `load_catalog` sites, MEASURED) and the minute cron (p50 9.7 ms). A faster CPU path does not
   fix a wrong data path; moving the fold to the object does (companion §4).
5. **wasm itself costs 45–55 % over native on SPEC** (Jangda et al., USENIX ATC '19, DOC), peaks 2.08–2.5×. That
   tax is paid today and is the price of the platform; it is only recoverable on the Box, where the same crates
   already run natively (`tools/native-spa-server/Cargo.toml:38-45` links `dowiz-kernel` and `dowiz-core`).
6. **The Worker is built for SIZE, not speed**: `opt-level = "z"`, `lto = true`, `codegen-units = 1`
   (`workers/api/Cargo.toml:57-59`), while the kernel's own native profile is `opt-level = 3`, `lto = "thin"`
   (`kernel/Cargo.toml:361-363`). Whether `s` or `2` beats `z` on the fold is UNMEASURED and is the one cheap
   probe worth running (§3, P1).
7. **`-C target-cpu=native` is wrong in both places.** On wasm there is no host CPU (DOC: the "cpu" is `generic`
   or `mvp`, and "WebAssembly binaries must only contain code the engine understands"); on the Box it bakes the
   BUILD host's ISA into a binary that will run on a different board — the SIGILL shape of the
   `termux-pkgconfig-poisons-glibc-builds` incident. Use a named baseline (`cortex-a76` for a Raspberry Pi 5)
   when the board is chosen.
8. **Verdict:** none of (C) is a performance path for dowiz. The paths that are, in order of measured
   effect, are in §3 — projections in the object, alarms instead of the cron, byte pass-through, the platform
   object read once per minute, and the $5 Paid plan that removes the 10 ms kill outright.

---

## 1. The deployment, as facts

| Fact | Source |
|---|---|
| One Worker, one wasm module, `worker-build --release` | `wrangler.toml` `[build]`; `workers/api/Cargo.toml:44-46` `crate-type = ["cdylib"]` |
| Free plan: 10 ms CPU per request and per cron; Paid: 30 s default, 5 min max | DOC https://developers.cloudflare.com/workers/platform/limits/ ("Last updated Sep 5, 2026") |
| "Waiting on network requests ... does not count toward CPU time" | same page, "CPU time" |
| Memory 128 MB per isolate, both plans | same page |
| No threads; SIMD supported ("the same set of features that are available in Google Chrome") | DOC https://developers.cloudflare.com/workers/runtime-apis/webassembly/ ("Last updated Apr 23, 2026") |
| rustc `wasm32-unknown-unknown` default features: `multivalue`, `mutable-globals`, `reference-types`, `sign-ext`, `nontrapping-fptoint`, `bulk-memory`; SIMD only via `-Ctarget-feature=+simd128` or per-function `#[target_feature]` | DOC https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html |
| Default `-Cpanic=abort` on wasm32; `unwind` needs `-Zbuild-std` on nightly | same page |
| Durable Object: single-threaded, soft 1,000 req/s per object, CPU 30 s default | DOC https://developers.cloudflare.com/durable-objects/platform/limits/ ("Last updated Jun 1, 2026") |
| Worker CPU p50 2.7 ms / p90 20 / p99 91; 622 kills at 10,000 µs; object 0 `exceededCpu` in 50,155 requests, wall p99 246 ms | MEASURED, `docs/design/BLUEPRINT-FREE-TIER-2026-09-26.md` §1.2 |
| Edge ≈ 50 ms; Worker + object memory ≈ 105–135 ms; object cold ≈ 260 ms | MEASURED, OPT §A.1 |
| Kill hours have ≥ 100 KB of object bytes per Worker request; 40 `load_catalog(` sites | MEASURED, FREE-TIER §2.2; `grep -rn "load_catalog(" workers/api/src` = 40 on this date |
| bebop compute is 2.6–10.6× slower than Rust | MEASURED, `docs/measurements/COSTS-NOW-AND-BEBOP-2026-09-26.md` §0.5 |
| The Box's server: axum 0.8 + tokio "full" + rustls/tokio-rustls, HTTP/1.1 and HTTP/2-over-TLS listeners | `tools/native-spa-server/Cargo.toml:21-51`; `src/main.rs:288-299`, `:309-345` |
| The Box's store is files with a mutex as the single writer | `tools/native-spa-server/src/hub.rs:8-11`, `:40-58` |

---

## 2. Each item of (C), where dowiz actually runs

### 2.1 Unikernels — RustyHermit / Hermit, Unikraft, MirageOS

- **Hermit** (DOC https://doc.rust-lang.org/rustc/platform-support/hermit.html): rustc Tier 3; targets
  `x86_64-`, `aarch64-`, `aarch64_be-`, `riscv64gc-unknown-hermit`; "only support cross-compilation"; "do support
  std"; "Rust does not yet ship pre-compiled artifacts for these targets" (build-std or a custom toolchain);
  images boot under Hermit's loader or the Uhyve hypervisor, or QEMU; "do not yet support C code and Rust code at
  the same time". The last sentence matters: `native-spa-server` links `rustls`, whose default crypto provider
  carries C/assembly (`aws-lc-rs`; `ring` likewise) — a Hermit build needs a pure-Rust provider **(unverified
  which provider the lock resolves to)**.
- **Unikraft** (DOC https://doc.rust-lang.org/rustc/platform-support/unikraft-linux-musl.html): "Unikraft
  pretends to behave exactly like Linux"; a Rust target exists; the catalogue lists Rust Actix and Rocket servers
  (https://github.com/unikraft/catalog). Its kernel is C.
- **MirageOS** is OCaml; not a target for a Rust tree. (Tarides announced a Unikraft backend for MirageOS,
  2025-11-13, https://tarides.com/blog/2025-11-13-announcing-unikraft-support-for-mirageos-unikernels/.)

**Where it would help:** only the Box, and only as a VM image under a hypervisor — which means a host kernel
exists anyway (Linux/KVM, or seL4 as hypervisor, companion `2026-09-27-sel4-wasm.md` §3). **What it buys:** a
smaller attack surface and a millisecond boot. **What it costs:** no shell, no `sshd`, no S3 CLI, no `journalctl`
on a device in a restaurant's back room; Tier 3 toolchains; a C-free TLS stack. **What it does not buy:**
latency — a 30-orders/day process is idle 99.9 % of the time (EST: 30 × 6 events × ≈ 10 ms of work per day).
**Verdict: HYPOTHESIS AGAINST for now.** The Box's first form is what `Dockerfile` DK-08 already builds: a single
static binary on a minimal rootfs (`docker/mkosi-rootfs.sh` is named there). That gets most of the surface
reduction with none of the toolchain risk. Re-entry condition: a Box pilot exists and its threat model names the
host kernel as the weakest layer.

### 2.2 Kernel bypass — DPDK / SPDK

DOC (https://doc.dpdk.org/guides-17.08/prog_guide/poll_mode_drv.html): a poll-mode driver "must not use any
asynchronous notification mechanisms" — it spins a core. DPDK/SPDK are built for line-rate packet and NVMe I/O at
millions of operations per second. dowiz's Box handles a handful of requests per minute and writes a few
kilobytes per order (an append is chunk 0 + the tail, `workers/api/src/hubdo.rs:1887-1893`; the whole hot log is
bounded by a 30-day rotation, `workers/api/src/hubstore.rs:1015`). A spinning core on a fanless ARM board is watts
and heat for no request. **Verdict: never.** And on Cloudflare there is no NIC to bypass.

### 2.3 io_uring

DOC (https://man7.org/linux/man-pages/man7/io_uring.7.html): Linux-specific asynchronous I/O via shared
submission/completion rings; introduced in 5.1 (Oracle's introduction, https://blogs.oracle.com/linux/an-introduction-to-the-io-uring-asynchronous-io-framework).
Its gain is measured at tens of thousands of operations per second and above. `native-spa-server` uses tokio
(`Cargo.toml:49`, epoll-based on Linux). A restaurant's Box does one file write per order event and a few reads
per poll. **Verdict: no measurable gain; keep tokio.** If the Box ever serves many venues (it is P67's
one-restaurant shape today, `hub.rs:3-5`), the bound will still be the network, not the syscall.

### 2.4 Native AOT Rust with `-C target-cpu=native`

- The Box IS native AOT Rust already (`native-spa-server`, one static binary; the same `dowiz-hub`,
  `dowiz-core`, `dowiz-kernel` crates as the Worker, `Cargo.toml:38-45`). The item reduces to one flag.
- DOC (https://doc.rust-lang.org/rustc/codegen-options/index.html): `-C target-cpu` "instructs rustc to generate
  code specifically for a particular processor ... `native` can be passed to use the processor of the host
  machine"; `-C target-feature` "Using this flag is unsafe and might result in undefined runtime behavior".
- **On wasm32 it is meaningless**: there is no host processor; the "cpu" is LLVM's `generic` feature set (DOC,
  wasm32 page), and any feature the engine lacks is a validation failure, not a slowdown.
- **On the Box it is a trap**: the binary is built on this box (Cortex-A78 big cores, `bebop-lang/tools/slot.sh`
  header) and would run on a Raspberry Pi 4 (Cortex-A72) or 5 (Cortex-A76). `native` bakes the build host's ISA
  and the first unsupported instruction is a SIGILL in the restaurant. Use a named baseline
  (`-C target-cpu=cortex-a72` is safe for both Pis; `cortex-a76` when the board is fixed).
- **What it would speed up**: vectorisable arithmetic. dowiz's hot path is JSON parsing (`serde_json`, 2,529 use
  sites under `workers/api/src`, MEASURED) and SHA-256 content ids (`crates/dowiz-hub/src/lib.rs:412-416`). The
  `sha2` crate selects hardware SHA at RUN time through `cpufeatures` (present in `workers/api/Cargo.lock`),
  so the native Box already gets the AArch64 SHA extension without any flag. Memory `d4-floor-is-not-arithmetic`
  records the same lesson for bebop: the decode was 1–2 ns/slot and the 53 ns/entry was elsewhere.
- **Verdict: not a lever.** Pin a baseline CPU when the board is chosen; never `native`.

---

## 3. What would actually make dowiz faster, ranked by measured effect

| # | Change | Effect | Evidence | Where |
|---|---|---|---|---|
| P0 | Workers Paid ($5/mo) | removes the 10 ms kill for 1.4 % of requests (622 of 44,209) outright | MEASURED, COSTS §0.2 | account setting |
| P1 | Catalogue and orders as projections in the object; Worker forwards bytes | catalogue-pulling requests ≈ 10 ms → ≈ 1 ms (EST, FT1); kill hours end | MEASURED cause, FREE-TIER §2.2 | companion §3–4, R1/R2 |
| P2 | Minute cron → per-object alarms | −1,440 Worker invocations/day, −8,640 object requests/day at V = 2; cron CPU line gone | MEASURED, FT2 | companion §4.3 |
| P3 | Platform registry read once per minute, not per request | one fewer serialised hop per request; ≈ 9 platform-object req/s at 200 venues avoided | MEASURED shape, OPT §A.2 | FT8 |
| P4 | Ping auto-response on the object | the 25 s client ping stops waking the object | MEASURED, FREE-TIER §0.7, FT6 | `hubdo.rs` socket accept |
| P5 | `opt-level = "s"` or `2` vs `"z"` on the Worker | UNMEASURED; the rustwasm book: "`opt-level = "s"` can sometimes result in smaller binaries than `"z"`. Always measure!" (DOC https://rustwasm.github.io/book/reference/code-size.html); speed effect unknown | one probe: build three profiles, run `/fold/rebuild` on the QA hub 20× each, compare wall | a lane, through `slot.sh` |
| P6 | Keep the object warm for the dinner window | cold 260 ms → warm ≈ 105 ms on the first request after idle | MEASURED, OPT §A.1 | alarms make this free: a due outbox entry wakes it anyway |
| P7 | Edge cache on `/menu` (exists, 30 s) and `/media` (exists) | ≈ 50 ms edge hits | MEASURED, OPT §A.1 | done; the trap is `?fresh=1` (memory `dowiz-inventory-recipes`) |

P5 is the only item on this list that is about code generation, and it is a measurement, not a decision.

---

## 4. Wasm vs native, so the tax is stated once

Jangda, Powers, Berger, Guha, "Not So Fast: Analyzing the Performance of WebAssembly vs. Native Code", USENIX
ATC '19 (DOC https://www.usenix.org/conference/atc19/presentation/jangda): "applications compiled to WebAssembly
run slower by an average of 45% (Firefox) to 55% (Chrome), with peak slowdowns of 2.08× (Firefox) and 2.5×
(Chrome)" on SPEC CPU; causes include register pressure, extra branches for safety checks and missing
optimisations — "some ... inherent to the WebAssembly platform". That is the ceiling on what any wasm-side
codegen work can recover, and it is smaller than the 10× data-path effect of P1. The native Box pays none of it.

---

## 5. What was NOT verified

- Which TLS crypto provider `native-spa-server`'s lock resolves to (decides Hermit feasibility).
- P5's numbers: the three-profile probe was not run here (a cargo build is a heavy job; the slot was busy with
  another lane for the whole of this session).
- Per-route CPU for `/api/owner/orders` and `/api/menu` (Workers Logs are dashboard-only on Free).
- Whether the object's CPU is really 30 s on the Free plan (FREE-TIER OPEN 2). P1 assumes it is; a 15 ms probe
  in `/fold/rebuild` settles it.
