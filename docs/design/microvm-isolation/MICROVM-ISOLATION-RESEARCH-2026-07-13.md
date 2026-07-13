# microVM Isolation (Firecracker / Cloud Hypervisor / Kata) vs Docker for dowiz/bebop2 — RESEARCH

> Date: 2026-07-13 · Research only, **no code**. Feeds a future blueprint/ADR the same way
> [integration-ports/RESEARCH-CONSPECT.md](../integration-ports/RESEARCH-CONSPECT.md) fed
> [BLUEPRINTS-INTEGRATION-PORTS.md](../integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md).
> Scope: dowiz/bebop2 — decentralized post-quantum food-delivery mesh, Rust/WASM kernel,
> pgrust per-node, capability-ports (`scope.rs` Resource×Action), agent-governance layer.
> Operator leans toward microVM; this doc's job is to say honestly where that lean is right
> and where it isn't.

## 0 — Bottom line up front

microVM (Firecracker, or Kata-on-Firecracker/Cloud-Hypervisor) is the **right tool for exactly
one job** in this architecture: sandboxing genuinely untrusted, **non-WASM** code that a
third party or an agent supplies and that must run as a real OS process. It is the **wrong
tool** for the kernel/server request path, the static SPA, pgrust, and — categorically — the
courier's phone. Full reasoning below; synthesis in §6; blueprint units in §7.

---

## 1 — Firecracker / Cloud Hypervisor / Kata: what they are (2026)

**Firecracker** — a Rust VMM (~83K LOC) built by AWS, open-sourced under **Apache-2.0**.
Uses KVM directly; exposes a minimal device model (virtio-net, virtio-block, serial console,
one-button reset — five devices total), no BIOS/UEFI, no GPU, no USB, direct kernel boot only.
Cold boot ≈**125ms**, memory overhead **<5MiB per microVM**, up to **150 microVMs/second** on
a single host [firecracker-microvm.github.io](https://firecracker-microvm.github.io/),
[GitHub](https://github.com/firecracker-microvm/firecracker). Snapshot/restore (pre-booted
image resumed rather than cold-booted) cuts this further, to low single-digit ms in some
reported setups [dev.to](https://dev.to/adwitiya/how-i-built-sandboxes-that-boot-in-28ms-using-firecracker-snapshots-i0k).
Production home: **AWS Lambda and Fargate**, handling trillions of invocations/month
[AWS blog](https://aws.amazon.com/blogs/aws/firecracker-lightweight-virtualization-for-serverless-computing/).
Also backs **Fly.io's Fly Machines** [Northflank](https://northflank.com/blog/what-is-aws-firecracker),
Koyeb, Northflank, appfleet, OpenNebula, Qovery — i.e. this is the same primitive dowiz
already runs *on top of* today via Fly.io hosting, without dowiz itself operating it.

**Cloud Hypervisor** — sibling Rust VMM from the same `rust-vmm` lineage, also **Apache-2.0**,
~106K LOC. General-purpose rather than serverless-density-optimized: CPU/memory hotplug,
GPU passthrough (VFIO), live migration, vhost-user, Windows guest support, x86_64 **and**
aarch64. Boots ≈**200ms** — the extra ~75ms over Firecracker buys hotplug/live-migration/
broader hardware compatibility Firecracker deliberately omits
[Northflank](https://northflank.com/blog/firecracker-vs-cloud-hypervisor),
[PandaStack](https://www.pandastack.ai/blog/firecracker-vs-cloud-hypervisor/).

**Kata Containers** — not a VMM; an **OCI-compatible container runtime** (OpenInfra
Foundation, Apache-2.0) that wraps QEMU, Cloud Hypervisor, or Firecracker so that a pod/
container gets its **own guest kernel** instead of sharing host kernel namespaces. Drops
into containerd/CRI-O with one shim per pod. 2026 framing: **"the VM is the security
boundary, not the container"**
[Kata architecture docs](https://kata-containers.github.io/kata-containers/design/architecture/),
[AWS — Kata for K8s isolation](https://aws.amazon.com/blogs/containers/enhancing-kubernetes-workload-isolation-and-security-using-kata-containers/).
This is the pragmatic on-ramp for teams that already package workloads as OCI images and
want a VM boundary "for free" without hand-building a jailer/orchestrator the way AWS did
for Lambda.

**Overhead ladder (2026 consensus across sources)**:

| Technology | Boot | Memory overhead | Isolation boundary | License / maturity |
|---|---|---|---|---|
| Docker (shared kernel) | ~50ms | minimal | none — kernel exploit = full escape | — |
| gVisor (userspace kernel intercept) | ms | +10–30% I/O (up to ~50% on I/O-heavy) | syscall interception, not HW | Apache-2.0, Google, mature |
| **Firecracker** | **~125ms** | **<5MiB/VM** | dedicated guest kernel (HW-enforced) | Apache-2.0, AWS, hyperscale-proven since 2018 |
| **Cloud Hypervisor** | **~200ms** | similar class | same, + hotplug/live-migration | Apache-2.0, rust-vmm/Linux Foundation, mature |
| **Kata (wraps above)** | **~200ms** | same class as backend | same, OCI-packaged | Apache-2.0, OpenInfra Foundation, mature, used in regulated multi-tenant K8s |
| WASM (Wasmtime/WasmEdge) | **microseconds** | negligible | capability/import-scoped, software-enforced | varies by runtime, BSD/Apache |

Sources: [Northflank — Firecracker vs gVisor](https://northflank.com/blog/firecracker-vs-gvisor),
[Northflank — Kata vs Firecracker vs gVisor](https://northflank.com/blog/kata-containers-vs-firecracker-vs-gvisor),
[Northflank — AI agent sandboxing 2026](https://northflank.com/blog/how-to-sandbox-ai-agents),
[arXiv — WASM and Unikernels comparative study](https://arxiv.org/html/2509.09400v1).
The gap between WASM instantiation and a microVM cold boot is reported as **3–4 orders of
magnitude** — "Firecracker boots in ~125ms, and it's still ~25x slower than WebAssembly."

ARM support exists in all three (Firecracker and Cloud Hypervisor both target aarch64 with
hardware virtualization; Kata wraps whichever backend the host supports) — but "ARM support"
means **server/SBC-class aarch64 with KVM enabled**, not mobile SoCs (see §5).

---

## 2 — The genuine fit: strong isolation of untrusted code

The prompt's hypothesis was: *dowiz's agent-governance layer + skill/hook execution runs
untrusted agent-generated code, so that's the microVM candidate.* Checked this against the
actual code (`/root/dowiz/agent-governance/index.ts`) rather than assuming:

- **`agent-governance/index.ts` is a zero-dependency, pure-function TEXT/VOICE policy layer**
  (gender/profanity/archetype/GodRelation axes, HARD-BAN list, drift detection). It does not
  execute arbitrary code at all — it classifies and constrains agent *output*, running in the
  same trust domain and same process as the rest of the app. There is **nothing to sandbox
  here**; wrapping it in a microVM would isolate trusted code from itself.
- Claude Code's own skill/hook execution (the harness this research was written under) is
  agent-tooling running on the **operator's own machine/CI**, not a multi-tenant production
  surface serving other people's requests. Different threat model, lower stakes, arguably not
  dowiz's isolation problem to solve with product infrastructure.
- The **real** untrusted-code-execution surface, per the existing integration-ports design
  (`docs/design/integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md`, IP-01/IP-02), is the
  planned **"Dev-agent" tier** of capability-port adapters and any third-party MCP server /
  plugin that is *not* first-party Rust compiled into the `KernelFacade`-firewalled workspace.
  That tier is explicitly meant to accept adapters the dowiz team didn't write and can't fully
  vet — code from an unknown or adversarial author, running server-side, on behalf of an
  owner/venue. **That** is the textbook Firecracker/Kata use case: hardware-enforced
  isolation for code you must assume is hostile, where a 125–200ms boot is irrelevant because
  the adapter runs as a background job/webhook handler, not a hot request path.
- One more honest wrinkle: dowiz/bebop2 is a **decentralized, per-node/per-venue**
  architecture, not a centralized multi-tenant FaaS platform. Firecracker's headline
  selling point — 150 microVMs/sec, <5MiB overhead, built for Lambda-scale density — is
  solving a hyperscale multi-tenancy problem dowiz doesn't have. A single owner's node
  running a handful of Dev-agent-tier adapters needs **boundary strength**, not **density**.
  That argues for **Kata-on-containerd** (reuses OCI packaging, less bespoke ops for a small
  self-hosted node) over hand-rolling a Firecracker jailer+API server the way AWS did — unless
  the node itself is resource-constrained enough that Firecracker's smaller footprint matters
  more than Kata's easier packaging story.

**Verdict for §2: yes, this is the real microVM use case in dowiz's architecture — but it's
the *future* Dev-agent-tier port adapters, not the *existing* agent-governance module, which
turns out not to execute untrusted code at all.**

---

## 3 — The overkill cases (be direct)

**Static SPA (nginx/static hosting).** No code executes at request time beyond serving
bytes. microVM is not merely overkill here, there is no isolation problem to solve — a CDN
or static host is strictly correct. Even a container is arguable overkill for this surface.

**The Rust/WASM kernel + capability-ports (`kernel/`, `server/` axum+tokio+rusqlite,
`KernelFacade`, `HybridGate`).** This is first-party, compiled, memory-safe Rust that dowiz
already tested (17/17 VbM). The isolation boundary that actually matters here is the
**compile-time firewall** (adapter crates cannot import `dowiz-kernel` — IP-01) and the
**capability/scope system** (`scope.rs` Resource×Action, IP-02), not runtime sandboxing.
Wrapping every `decide`/`fold` call in a microVM would add ~125ms+ to every order-state
transition for a code path that is already memory-safe and already never executes anything
it didn't compile itself. That is isolation theater: VM-isolating trusted code from itself.

**pgrust (per-node Postgres driver/service, source-of-truth).** A first-party, trusted
component. It doesn't run third-party code; its actual risk model is SQL injection / RLS
bypass / credential leakage — none of which a process/VM isolation boundary touches. A
microVM only becomes relevant if pgrust *itself* is treated as a blast-radius container so a
compromise of it can't pivot to co-located services — that is the **"hardened container /
one-VM-per-service" ops-hygiene argument**, a legitimate but *different* justification
(defense-in-depth for a trusted-but-compromisable service) than "isolate code we don't
trust," and a much lower priority than RLS/credential hardening for this component.

---

## 4 — The crux: microVM vs WASM for untrusted code

This is the actual decision axis, not "container vs VM."

**Reach for a full microVM (Firecracker/Kata) when:**
- The untrusted payload is **not WASM** — an arbitrary Linux binary, a Python/Node
  interpreter, anything you cannot compile through a controlled WASM toolchain.
- The code needs **real OS primitives** (spawn processes, open arbitrary files, raw sockets)
  that can't be meaningfully attenuated through a capability/import table.
- The trust model is genuinely adversarial multi-tenant and a kernel 0-day is a realistic
  threat you must survive without pivoting to other tenants.
- 125–200ms boot latency is acceptable — i.e. it's a background job/webhook, not a hot path.

**A WASM sandbox suffices when:**
- You control the **compilation pipeline** — the untrusted party supplies logic, but it's
  compiled to `.wasm` by a pipeline you trust, so "untrusted" means "untrusted judgment," not
  "untrusted toolchain." [wasm-sandbox](https://github.com/ciresnave/wasm-sandbox),
  [Wasmtime security model](https://docs.wasmtime.dev/security.html): WASM code can only call
  what the host explicitly imports; if the host doesn't expose a filesystem function, the
  module cannot read files, full stop.
- Capability scoping maps cleanly onto **WASI Component Model** imports — and dowiz's own
  `scope.rs` `Resource`/`Action` enum (IP-02) is already exactly that shape: a WIT world's
  import list *is* a capability grant. The Component Model crossed from "experimental demo" to
  production-targetable in 2026 (Wasmtime 22+, Spin 2.0+, wasmCloud 1.0+)
  [techbytes.app](https://techbytes.app/posts/wasm-component-model-2026-cloud-interop-deep-dive/),
  [eunomia.dev](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/).
- Start latency matters (interactive path, not just batch/webhook).
- Memory-safety-of-the-guest is the actual concern, not "needs a real kernel."

**Why this matters concretely for dowiz:** the kernel is *already* Rust→WASM, capability-ports
are *already* `scope.rs`-enum-based, `KernelFacade` is *already* a compile-time firewall.
Extending that same model to Dev-agent-tier adapters as **WASM Components** (WIT world =
exactly the existing `Resource`/`Action` scope) is architecturally free — it reuses machinery
that exists today. Standing up a microVM subsystem (jailer, seccomp profiles, guest kernel
image + patching, network bridge/tap, orchestration) shares **zero** code with anything
currently in the repo. microVM only becomes *necessary* the moment dowiz commits to accepting
untrusted code in a **non-WASM** form (e.g. "owners can upload an arbitrary script," or a
third-party MCP server distributed as a native process rather than a WASM component).

Caveat for intellectual honesty: WASM sandboxes are not infallible either — CVE-2026-34971
(a Cranelift codegen bug letting WASM guest code read/write arbitrary host memory on aarch64,
affecting Graviton/Apple-Silicon/ARM edge) shows runtime bugs happen at this layer too
[systemshardening.com](https://www.systemshardening.com/articles/wasm/wasip3-security-roadmap/).
That argues for keeping the WASM runtime patched and possibly still running the WASM host
process itself inside a container, not for abandoning WASM as the default — it's a runtime
bug fixed at the runtime layer, not a structural argument for microVM given dowiz's actual
threat model (§2).

---

## 5 — Edge/phone/node fit: can this run on the decentralized-node hardware?

Firecracker and Cloud Hypervisor both **hard-require KVM**: a Linux host with hardware
virtualization extensions enabled (Intel VT-x/AMD-V on x86, ARM virtualization/EL2 on
aarch64) [Firecracker FAQ](https://github.com/firecracker-microvm/firecracker/blob/main/FAQ.md).

**Owner's small self-hosted box (mini-PC, NUC, decent SBC):** plausible but not free. A
Raspberry Pi 4/5 with 8GB **can** run KVM in principle, but the community's own assessment is
blunt: "practical application remains quite questionable due to several serious limitations"
— CPU-compatibility gaps, thin ecosystem support
[ostrich.kyiv.ua](https://ostrich.kyiv.ua/en/2025/05/08/raspberry-pi-as-a-kvm-hypervisor/),
[Raspberry Pi forums](https://forums.raspberrypi.com/viewtopic.php?t=366860). A mid-range
mini-PC/NUC-class box is fine. The honest framing: microVM is *available* on the upper end
of "owner's small box" hardware, but it adds real host requirements (KVM enabled, root/
`CAP_SYS_ADMIN` for the jailer, a guest kernel image to build and patch, network bridge/tap
setup) to a **single-owner, single-tenant node** whose actual adversary is closer to "one
untrusted Dev-agent-tier plugin the owner opted into," not "a thousand hostile tenants
sharing a host." That's precisely the situation a WASM Component sandbox already fits better:
no KVM dependency, works on anything the Rust/WASM kernel already runs on.

**Courier's phone:** **not possible on stock hardware.** Modern mobile SoC firmware
explicitly drops privilege to EL1 when jumping into the kernel and never returns to EL2 —
the level KVM needs — and OEMs burn signature keys into the SoC at manufacture, so only
vendor-signed firmware boots. Without an OEM-provided unlock path, there is no user-space way
to enable KVM on a stock consumer phone
[blog.lyc8503.net](https://blog.lyc8503.net/en/post/android-kvm-on-mediatek/). This isn't
"heavier than WASM" the way the owner's-box case is — it's categorically unavailable. The
courier app surface is WASM/native-app code only; microVM is not a candidate there under any
circumstance.

---

## 6 — Synthesis (honest verdict)

microVM (Firecracker, or Kata wrapping Firecracker/Cloud Hypervisor) is the right tool for
**one specific job** in this architecture: hardware-enforced isolation of genuinely untrusted,
**non-WASM** code that dowiz cannot compile through its own trusted pipeline — concretely,
the future "Dev-agent tier" of capability-port adapters or a hostile/unknown third-party MCP
server that ships as a native OS process rather than a WASM component. That surface doesn't
exist in the codebase yet, and the module the operator specifically flagged
(`agent-governance/index.ts`) turns out to be a pure-function text/voice policy filter, not a
code-execution sandbox — it needs no isolation at all. So the "genuine fit" is real, but it's
future and narrower than the initial framing suggested.

Everywhere else in the stack, microVM is the wrong tool, for concrete and different reasons
each time: the kernel/server request path is first-party compiled Rust already protected by a
compile-time firewall (IP-01) and a capability-scope system (IP-02) — VM-wrapping it adds
125ms+ per order transition to isolate trusted code from itself. The static SPA has no
code-execution surface to isolate. pgrust's risk is SQL/RLS/credential-shaped, not
process-isolation-shaped. And on the decentralized-node runtime specifically, microVM's
KVM dependency is a real tax on an owner's small box (available but fragile on Pi-class
hardware, fine on a mini-PC) and **categorically impossible** on a courier's phone, whose
firmware structurally blocks hypervisor privilege.

The deeper reason to prefer WASM as the default untrusted-extension path isn't just that it's
cheaper (microseconds vs 125ms, no KVM dependency) — it's that dowiz has *already built* the
capability infrastructure WASM Components want: `scope.rs`'s `Resource`×`Action` enum is
structurally a WIT world's import list. Reusing it costs nothing. A microVM subsystem
(jailer, seccomp, guest kernel, bridging, orchestration) would be new machinery sharing zero
code with anything in the repo today.

**Recommendation:** do not adopt microVM as a Docker replacement. Adopt it as a **targeted,
opt-in isolation tier** (MV-03 below) reserved for the one case WASM structurally cannot
cover — untrusted non-WASM code — gated behind an explicit host-capability check (MV-04) so
a node without KVM fails closed instead of silently running unsandboxed code. Everything
else in the stack keeps its current isolation story (Rust memory-safety + compile-time
firewall + RLS), or moves to WASM Components where "untrusted but ours-to-compile" code needs
a boundary.

---

## 7 — Blueprint units (MV-01..05)

Format matches [BLUEPRINTS-INTEGRATION-PORTS.md](../integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md):
Goal · Boundary (touch / don't touch) · Form · RED test · Tier. 🔴 = red-line/human-gated;
🟢/🟡 = T1–T3 delivery tiers. **This is a blueprint, not an implementation — no code in this
doc.**

### MV-01 · Untrusted-code classifier (prerequisite gate)
- **Goal:** force an explicit decision — "does this adapter execute non-WASM/non-first-party
  code" — before any Dev-agent-tier or third-party MCP adapter ships, instead of defaulting
  into either "trust it" or "microVM everything."
- **Boundary:** touch — Dev-agent-tier adapter manifest schema (integration-ports plan). Don't
  touch — `KernelFacade`/`scope.rs` (already correct per IP-01/IP-02).
- **Form:** each adapter manifest declares `execution: wasm-component | native-process`.
  `wasm-component` routes to MV-02; `native-process` routes to MV-03.
- **RED:** an adapter manifest that omits `execution`, or declares `wasm-component` while its
  build artifact is a native ELF/PE binary → adapter registration MUST fail CI.
- **Tier:** 🔴 (gates everything downstream; policy/schema only)

### MV-02 · WASM Component sandbox — default untrusted-extension path
- **Goal:** make Dev-agent-tier / third-party adapters capability-scoped WASM Components by
  default, reusing `scope.rs` as the WIT world — no microVM needed for this path.
- **Boundary:** touch — new adapter-host embedding (wasmtime or equivalent). Don't touch —
  `kernel/`, `KernelFacade`.
- **Form:** an adapter's WIT world may only import the exact `Resource`×`Action` pairs its
  grant contains; host denies any ungranted import at **instantiation** time, not first call.
- **RED:** a WASM component whose WIT world imports `Resource::Order × Action::CreateOrder`
  while granted only `Resource::Notify × Action::Notify` → instantiation MUST fail closed.
- **Tier:** 🟢 T2/T3 — primary deliverable; most "untrusted code" effort belongs here, not in
  MV-03.

### MV-03 · microVM sandbox — fallback tier for non-WASM untrusted code only
- **Goal:** give the one case WASM structurally cannot cover (arbitrary non-WASM binaries/
  interpreters) a real isolation boundary, explicitly scoped as a fallback, not a default.
- **Boundary:** touch — new opt-in native-adapter sandbox, Kata-on-containerd preferred over
  hand-rolled Firecracker+jailer (single-tenant per-node scale doesn't need Lambda-style
  density; OCI packaging reuses existing adapter-distribution tooling). Don't touch — MV-01/
  MV-02; this unit is additive and off by default.
- **Form:** `execution: native-process` adapters ship as an OCI image; Kata runtime (backed by
  whichever of Firecracker/Cloud Hypervisor the host advertises) gives each its own guest
  kernel; network egress default-deny except the adapter's declared target(s).
- **RED:** a native-tier adapter attempts a syscall/network call outside its declared egress
  allowlist → the guest network policy MUST block it, and the block MUST appear in the host
  audit log (not silently swallowed).
- **Tier:** 🟡 T3, gated by MV-04 host-capability check — the only unit in this set that runs
  genuinely non-memory-safe untrusted code server-side; human-reviewed before enabling per
  node.

### MV-04 · Host-capability probe (fail closed, never silently unsandboxed)
- **Goal:** a node without KVM must never be allowed to accept a `native-process` adapter —
  no silent downgrade from "isolated" to "unsandboxed."
- **Boundary:** touch — node bootstrap/capability advertisement. Don't touch — MV-02 (no KVM
  dependency, unaffected).
- **Form:** on node start, probe `/dev/kvm` + virtualization CPU flags; advertise
  `native-adapter-sandbox: available | unavailable` to the adapter registry.
- **RED:** a node without `/dev/kvm` receives a `native-process` adapter install request →
  registration MUST reject with an explicit "host cannot provide the isolation this adapter
  requires" error; it must never fall back to running the adapter unsandboxed.
- **Tier:** 🔴 (fail-closed safety net for MV-03)

### MV-05 · Explicit non-goals (durable rejection record)
- **Goal:** make the "don't build this" calls as durable as the "build this" calls, so a
  future contributor doesn't re-propose microVM-wrapping the kernel or the courier app.
- **Boundary:** touch — this doc + a short ADR. Don't touch — nothing in code; documentation
  only.
- **Form:** explicit rejected-surfaces list: (1) kernel/server request path — first-party,
  compiled, already firewalled (IP-01/IP-02), no per-request VM; (2) static SPA — no
  execution surface to isolate; (3) pgrust — RLS/credential hardening is the correct lever,
  not VM-per-query; (4) courier phone client — categorically impossible, no KVM path on stock
  mobile firmware, WASM/native app only.
- **RED:** none (documentation unit) — falsifiability = this ADR is cited and the proposal
  closed as already-decided the next time someone proposes microVM for one of these four
  surfaces.
- **Tier:** 🟢 (documentation only, zero engineering risk)

---

## Sources

- [Firecracker microVMs — official site](https://firecracker-microvm.github.io/)
- [firecracker-microvm/firecracker — GitHub](https://github.com/firecracker-microvm/firecracker)
- [Firecracker FAQ — KVM/host requirements](https://github.com/firecracker-microvm/firecracker/blob/main/FAQ.md)
- [AWS Blog — Firecracker: Lightweight Virtualization for Serverless Computing](https://aws.amazon.com/blogs/aws/firecracker-lightweight-virtualization-for-serverless-computing/)
- [Northflank — What is AWS Firecracker?](https://northflank.com/blog/what-is-aws-firecracker)
- [Northflank — Firecracker vs Cloud Hypervisor](https://northflank.com/blog/firecracker-vs-cloud-hypervisor)
- [Northflank — Guide to Cloud Hypervisor in 2026](https://northflank.com/blog/guide-to-cloud-hypervisor)
- [PandaStack — Firecracker vs Cloud Hypervisor](https://www.pandastack.ai/blog/firecracker-vs-cloud-hypervisor/)
- [Kata Containers — Architecture docs](https://kata-containers.github.io/kata-containers/design/architecture/)
- [AWS Blog — Enhancing Kubernetes workload isolation using Kata Containers](https://aws.amazon.com/blogs/containers/enhancing-kubernetes-workload-isolation-and-security-using-kata-containers/)
- [Your Container Is Not a Sandbox: The State of MicroVM Isolation in 2026](https://emirb.github.io/blog/microvm-2026/)
- [Northflank — Kata Containers vs Firecracker vs gVisor](https://northflank.com/blog/kata-containers-vs-firecracker-vs-gvisor)
- [Northflank — Firecracker vs gVisor](https://northflank.com/blog/firecracker-vs-gvisor)
- [Northflank — How to sandbox AI agents in 2026](https://northflank.com/blog/how-to-sandbox-ai-agents)
- [dev.to — 4 ways to sandbox untrusted code in 2026](https://dev.to/mohameddiallo/4-ways-to-sandbox-untrusted-code-in-2026-1ffb)
- [wasm-sandbox — GitHub](https://github.com/ciresnave/wasm-sandbox)
- [Wasmtime — Security](https://docs.wasmtime.dev/security.html)
- [systemshardening.com — WASI Security Roadmap (incl. CVE-2026-34971)](https://www.systemshardening.com/articles/wasm/wasip3-security-roadmap/)
- [techbytes.app — Wasm Component Model in 2026: Cloud Interop](https://techbytes.app/posts/wasm-component-model-2026-cloud-interop-deep-dive/)
- [eunomia.dev — WASI and the WebAssembly Component Model: Current Status](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/)
- [HN — fly.io uses Firecracker, Apache 2 license discussion](https://news.ycombinator.com/item?id=40354323)
- [blog.lyc8503.net — Running Linux/Windows on ARM via KVM on Android](https://blog.lyc8503.net/en/post/android-kvm-on-mediatek/)
- [ostrich.kyiv.ua — Raspberry Pi as a KVM hypervisor](https://ostrich.kyiv.ua/en/2025/05/08/raspberry-pi-as-a-kvm-hypervisor/)
- [Raspberry Pi Forums — Virtualisation on RaPi5](https://forums.raspberrypi.com/viewtopic.php?t=366860)
- [arXiv 2509.09400 — WebAssembly and Unikernels: A Comparative Study for Serverless at the Edge](https://arxiv.org/html/2509.09400v1)
- [dev.to — How I built sandboxes that boot in 28ms using Firecracker snapshots](https://dev.to/adwitiya/how-i-built-sandboxes-that-boot-in-28ms-using-firecracker-snapshots-i0k)
