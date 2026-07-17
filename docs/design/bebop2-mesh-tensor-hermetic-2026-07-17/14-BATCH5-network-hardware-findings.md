# BATCH 5 v2 — Network / Hardware Cluster, RE-DERIVED against the REAL target (2026-07-17)

> **This v2 supersedes the Fly-scoped v1** (preserved verbatim in the appendix at the bottom, tagged
> `SUPERSEDED — evaluated against the wrong target`). The v1 rejected RDMA / DPDK / custom-L2 /
> RSS / eBPF / hardware-attestation / NUMA as *physics-impossible* — but it evaluated all of them
> against **Fly.io Firecracker microVMs**. The operator has since stated, clearly and definitively:
>
> **Fly is no longer used at all. The deployment target is fully decentralized local nodes:**
> **(1) courier devices (phones), (2) owner-operated local hub servers (real hardware the owner**
> **controls), (3) client devices.** This is a fundamentally different hardware reality, and the
> REJECT-ON-PHYSICS verdicts had to be **re-derived, not defended.**
>
> **Binding methodology (operator, verbatim standing directive):** *"building & testing first, any
> claims or authority opinions next."* Every verdict below leads with a **probe I actually built and
> ran on this hardware**; literature and vendor numbers are secondary. Where I could not get the real
> target hardware (no physical courier phone, no owner-hub mini-PC, no TPM in this sandbox), I say so
> and specify **exactly what probe would settle it**, rather than reasoning from analogy.
>
> **Verdict tags (new in v2):**
> - `[PROBED]` — I built and ran something on this hardware; a real number backs the verdict.
> - `[RESEARCHED]` — web-grounded / documented API surface; no probe possible from this sandbox.
> - `[NEEDS-REAL-TARGET-HARDWARE]` — this sandbox honestly cannot settle it; the exact settling probe
>   is named.
>
> **Old epistemics tags retained:** `(live)` command I ran · `(file:line)` read from source ·
> `(prior-art)` a sibling blueprint's decided result · `(training-knowledge)` general knowledge,
> flagged · `(inference)` my derivation.

---

## §0. The corrected substrate model — THREE real node classes, not one microVM

The whole cluster resolves against *where the code physically runs*. v1 modelled ONE substrate (an
unprivileged Firecracker guest). The corrected reality has **three node classes with radically
different hardware capability**:

| Node class | Real hardware | Root? | Raw NIC / CAP_NET_RAW | CAP_BPF / XDP | TEE / secure enclave | NUMA |
|---|---|---|---|---|---|---|
| **Courier device** | Android / iOS **phone** | **No** (OS-sandboxed app) | **No** (no AF_PACKET; iOS blocks raw sockets entirely) | **No** | **YES** — StrongBox / TEE Keymaster (Android), Secure Enclave (iOS) | none (single SoC) |
| **Owner hub** | bare-metal Linux **mini-PC / NUC / small tower** the owner controls | **Yes** | **Yes** (physical NIC, root) | **Yes** (mainline kernel + root) | usually no discrete TPM/enclave unless fitted | single socket → **1 NUMA node** |
| **Client device** | phone / laptop / browser | No | No | No | phone: yes; browser: WebAuthn/passkey only | none |

**The single most important structural fact for this whole batch:** the mesh is **heterogeneous by
construction** — a phone and a hub are *different machines on different networks*. Any technique that
requires *both endpoints* to have a capability (RDMA both ends, DPDK both ends, raw-L2 both ends) is
gated by the **weakest** endpoint, and the weakest endpoint is always **a sandboxed phone on a mobile
network**. This is what actually kills most of the network-acceleration items — not "Firecracker."

### §0.1 This sandbox as a stand-in for the OWNER-HUB class (honest calibration)

I cannot get a courier phone or the owner's real mini-PC here. But the dev host **is** a full,
rooted, mainline-kernel Linux, which makes it a **partial but honest stand-in for the owner-hub
class** — with two caveats named up front:

| Property | This sandbox (live-probed) | A real owner-hub mini-PC | Representative? |
|---|---|---|---|
| CPU | AMD EPYC-Milan, 8 vCPU = 4c×2SMT, **1 socket, 1 NUMA node**, 31 GB | Intel N100 / i5 / Ryzen, 1 socket, 1 NUMA | **YES** (single-NUMA is the realistic hub) `(live lscpu)` |
| Root + caps | uid=0; `cap_bpf`,`cap_net_raw`,`cap_net_admin`,`cap_perfmon`,`cap_sys_nice`,`cap_ipc_lock`,`cap_sys_admin` all present | root on owner's own box → same | **YES** `(live capsh)` |
| eBPF/XDP kernel | `CONFIG_BPF=y`,`BPF_SYSCALL=y`,`XDP_SOCKETS=y`,`BPF_JIT=y` | mainline distro kernel → same | **YES** `(live /proc/config.gz)` |
| NIC | **virtio_net**, 1 combined queue, `ntuple-filters off [fixed]`, `receive-hashing off [fixed]` | **physical** Intel i225/i226 / Realtek: a few RX queues + basic RSS, still no rich ntuple | **PARTIAL** — a real NIC has *more* queue capability than virtio, *less* than a datacenter NIC `(live ethtool -l/-k)` |
| RDMA | **absent** (no `/dev/infiniband`, no `/sys/class/infiniband`) | absent unless the owner buys an RNIC | **YES** (consumer hub has no RNIC either) `(live ls)` |
| TPM / enclave | **absent** (no `/dev/tpm`, no `/sys/class/tpm`) | absent unless fitted; **phone has it, hub usually doesn't** | representative of hub; **not of the phone** `(live ls)` |

So: for the **owner-hub** verdicts, this sandbox gives real, load-bearing probe data (CAP_BPF, XDP
load, single-NUMA, io_uring, syscall cost). For the **courier-phone** verdicts (attestation, raw-L2
from a phone), the sandbox is **not representative** and those are `[NEEDS-REAL-TARGET-HARDWARE]` /
`[RESEARCHED]`.

---

## §0.5 THE DECISIVE MEASUREMENT — crypto dwarfs transport, on ANY hardware `[PROBED]`

Before item-by-item verdicts, one measurement governs items 1, 2, and half of 4. **Every network
acceleration technique in this cluster (RDMA, DPDK, AF_XDP, io_uring-on-the-wire) saves TRANSPORT
cost. The mesh's per-message cost is dominated by CRYPTO, which every node must run regardless of
fabric.** I measured both, on this host:

| Cost (this host, measured) | ns / operation | Probe |
|---|---|---|
| **Ed25519 verify** (OpenSSL 3.0.13) | **≈ 69,400 ns** (14,418 verify/s) | `openssl speed ed25519` `[PROBED]` |
| Ed25519 sign | ≈ 26,700 ns (37,467/s) | same `[PROBED]` |
| **Per-recv mesh crypto = Ed25519 + ML-DSA-65** (RequireBoth) | **≈ 55,000–140,000 ns** | Ed25519 measured + ML-DSA-65 verify ~40–70 µs `(iroh_transport.rs:372,437 file:line — every recv verifies BOTH legs)` |
| **UDP loopback, full TX+RX kernel stack, 64 B** | **6,882 ns** | `udp_bench.c` `[PROBED]` |
| UDP loopback, full stack, 1200 B (MTU) | 7,253 ns | `udp_bench.c` `[PROBED]` |
| Raw syscall floor (getpid, uncached) | **181 ns** | `iouring_bench.c` `[PROBED]` |
| io_uring NOP, batch=64 (amortized syscall) | **77 ns** | `iouring_bench.c` `[PROBED]` |

**The governing ratio:** per-recv crypto (**~55–140 µs**) is **~8–20× the entire packet stack
traversal** (~7 µs) and **~300–770× the raw syscall tax** (~0.18 µs) that io_uring/AF_XDP/DPDK
actually eliminate.

**Consequence, stated once and reused below:** kernel-bypass on the network path has a hard ceiling of
**removing ~7 µs per message** (the whole stack) — against a **~55–140 µs crypto floor that runs on
every node no matter what NIC is underneath.** Best realistic case (removing only the syscall portion,
which is what io_uring/AF_XDP mostly do) is **<0.4 %** of per-message cost; removing the entire stack
(DPDK/RDMA, impossible for phone peers anyway) is **<10 %**. **This holds on ideal bare-metal too** —
it is a *ratio between two costs that both live on the receiving node*, not an artifact of
virtualization. This is the target-correct reason several items stay REJECT even though the
"Firecracker" reason evaporated.

*(Caveat, Anu-honest: OpenSSL's Ed25519 is not the mesh's actual lib — `ring`/`ed25519-dalek` verify
~15–30 µs. Even at that optimistic floor, plus ML-DSA-65 ~40 µs, per-recv is ~55 µs = still ~8× the
packet stack and ~300× the syscall tax. The conclusion is robust across the whole plausible range.)*

---

## §1. RE-DERIVED VERDICTS (per the operator's six correction items)

### 1. RDMA / RoCE → **STAYS REJECT** — but for the target-correct reason, not "Firecracker" `[PROBED]` + `[RESEARCHED]`

- **Phones: no.** A sandboxed mobile app has no RNIC, no verbs, no kernel-bypass. Not arguable.
- **Owner hub: probed absent, and structurally wrong even if bought.** This host has **no
  `/dev/infiniband`, no `/sys/class/infiniband`** `(live)`. A realistic small-business courier hub is
  a mini-PC / NUC with a **consumer Intel/Realtek 1–2.5 GbE NIC — no RDMA verbs**.
- **What RDMA would actually require** `[RESEARCHED]`: a ConnectX/FastLinQ-class RNIC (used ConnectX-4
  25 GbE exists on the second-hand market, ~$40–100) **plus** — the real blocker — a **lossless
  fabric**: DCB + PFC-capable managed switches, which the vendor docs themselves call *"a complex
  process… scalability significantly constrained"* (NVIDIA/FS.com). That is **datacenter rack
  networking**, not a courier depot's WiFi.
- **The structural kill (Ananke, target-independent):** RDMA is **point-to-point within a rack between
  two RNICs**. The mesh's peers are **couriers on phones over LTE/5G/WAN** — there is **no RDMA path
  to a phone**, ever. And even hub↔hub, **RDMA's whole benefit is bypassing the remote CPU** — but the
  mesh *requires* the remote CPU to **verify both signature legs on every recv** (`RequireBoth`,
  `iroh_transport.rs:372`). You cannot RDMA-write into a peer's memory and skip its signature check
  without **discarding the entire authenticity model.** RDMA's core value proposition is
  *structurally incompatible* with a verify-every-message mesh.
- Plus §0.5: even if a hub↔hub RDMA link existed, it would shave transport off a **crypto-bound** cost.

**Verdict — REJECT-on-target.** *Reason changed:* v1 said "device absent in Firecracker." Corrected
reason: **(a)** the realistic hub has a consumer NIC + no lossless fabric (datacenter-only technique
for THIS business); **(b)** phone peers make it topologically impossible regardless; **(c)** its
CPU-bypass benefit contradicts verify-every-recv; **(d)** it would optimize transport that is <10 % of
per-message cost. **Un-reject trigger:** a hub↔hub-only sub-mesh on bought RNICs + a lossless switch
where a profile shows **transport, not crypto, dominates hub-to-hub latency** — a hardware
procurement + a measurement that §0.5 predicts will not appear.

### 2. DPDK on a dedicated hub / io_uring / AF_XDP → **"single-NIC" objection FLIPS; verdict stays DEFER/REJECT on measured crypto-dominance** `[PROBED]`

The operator is **correct** that the v1 physics reason is wrong here: an owner hub with **2+ NICs can
dedicate one to DPDK** without killing its only network path — the "binding the single shared NIC
removes all networking" objection **dissolves** for a real multi-NIC appliance. So I re-derived on
*payoff*, not possibility, and **built the benchmark** the operator asked for:

**io_uring vs raw-syscall, measured on this kernel** (`iouring_bench.c`, `[PROBED]`):

| Path | ns/op | syscalls/op |
|---|---|---|
| raw syscall (getpid) | 181 | 1 |
| io_uring NOP **batch=1** | **417** ← *slower than a raw syscall* | 1 |
| io_uring NOP batch=4 | 156 | 0.25 |
| io_uring NOP batch=16 | 94 | 0.0625 |
| io_uring NOP **batch=64** | **77** | 0.0156 |

Two real findings: **(a)** io_uring **only wins when it batches** — a single op (417 ns) is *worse*
than a plain syscall (181 ns) because of ring/mmap/atomic overhead. The mesh's outbound is *a few
long-lived QUIC sockets*, i.e. exactly the no-batching regime where io_uring **loses**. **(b)** Even
at batch=64, io_uring saves ~104 ns/op vs a raw syscall — and per §0.5 the whole packet stack is 7 µs
and crypto is 55–140 µs, so this ~0.1 µs saving is **noise against the real cost.**

- **Does io_uring / AF_XDP get "80 % of DPDK's benefit with less complexity"?** For the *syscall-tax*
  portion, **yes** — batch=64 io_uring already amortizes the syscall to 0.0156/op, and AF_XDP adds
  zero-copy on top, both without unbinding the NIC or spinning a poll-mode core. But **80 % of a
  benefit that is <10 % of per-message cost is still <10 %.** The complexity/benefit math is
  *decisively* against all three on the network path.
- **DPDK specifically:** even on a 2-NIC hub where it's *possible*, it needs the **peer to also run
  DPDK** to matter end-to-end — and the peers are **phones that cannot.** Plus a poll-mode core
  spinning at 100 % on a fanless mini-PC is a thermal/power anti-pattern for a home/depot appliance.

**Verdict:** **DPDK → REJECT-on-target** (possible on a 2-NIC hub, but crypto-dominance + phone-peers
+ power make the payoff negative). **AF_XDP → DEFER-WITH-TRIGGER on the hub** (now genuinely
*attachable* — see item 4 — but measured near-zero payoff on a crypto-bound QUIC mesh). **io_uring →
DEFER-WITH-TRIGGER, storage not network** (unchanged from v1's scope decision, now with a real ns
number: it *loses* in the few-long-lived-sockets regime; its only earn-its-place case is a
**syscall-heavy local file-I/O** path — the tensor-arena/block-store batch, if measured bottlenecked).

### 3. Custom L2 / raw Ethernet framing → **the "no shared L2 domain" reason PARTIALLY FLIPS; raw-L2-framing STAYS REJECT** `[RESEARCHED]` + `[PROBED]`

- **Is there a real shared-L2 scenario?** **Yes — the operator is right that one exists.** Couriers on
  the road are on LTE/5G → NAT'd, different ISPs, L3-routed, **no shared L2** (permanently WAN for the
  mobile case). **But** a **restaurant / warehouse / depot local WiFi AP** puts a courier's phone and
  the owner's hub on the **same subnet / same L2 broadcast domain** — a real, common food-delivery /
  logistics scenario (couriers picking up at a hub with owner-provided WiFi). So the blanket "no L2
  segment exists on either substrate" is **false for the co-located-WiFi scenario.**
- **But raw-L2 *framing* still can't be used there, for two target-correct reasons:**
  1. **The phone endpoint cannot emit raw Ethernet frames.** Android apps have no `CAP_NET_RAW` /
     `AF_PACKET` without root; **iOS sandboxes raw sockets entirely.** Even on shared WiFi, a courier
     PHONE app **physically cannot send a custom 32-byte L2 frame.** The weakest-endpoint rule (§0)
     kills it. *(This host HAS `cap_net_raw` `(live)` — but the hub is never the weak endpoint; the
     phone is.)*
  2. **Security regression is target-independent** (unchanged, correct in v1): putting
     `TileID/EpochID/HypothesisID/Seq` in a **cleartext L2 header** moves routing-critical fields
     **outside the signed envelope** (`framing.rs` length-prefixed carrier-neutral envelope;
     `iroh_transport.rs` verifies every recv). Raw-L2 framing throws away the authenticity model to
     replace a transport QUIC already provides.
- **The real correction for the co-located-WiFi case** (this is genuinely adoptable and better than
  raw L2): **local-subnet UDP peer discovery — mDNS / UDP broadcast — which phones CAN do.** When a
  courier's phone and the hub share a LAN, discover and talk **directly on the local subnet over
  QUIC/UDP** (skipping any WAN relay), instead of a raw L2 frame. Same latency win the L2 idea reached
  for, achievable on the actual weak endpoint, and it keeps the signed envelope intact.

**Verdict — raw-L2 framing REJECT-on-target** (phone OS blocks raw frames on the weak endpoint +
signed-envelope regression). *Adopt instead:* **LAN-local UDP/mDNS discovery + direct-subnet QUIC**
for the co-located-WiFi scenario the operator correctly identified — a real un-rejection of the
*intent* on a phone-capable mechanism.

### 4. RSS / hardware flow-steering; eBPF / XDP → **eBPF FLIPS to AVAILABLE-on-hub (proved by loading real programs); DEFER-WITH-TRIGGER, phone-incapable; RSS stays REJECT-on-virtio / DEFER-on-real-NIC** `[PROBED]`

**I built the probe the operator asked for and it settled the eBPF question directly.** `bpf_probe.c`
loads real BPF programs via the `bpf()` syscall (hand-assembled bytecode — no clang needed):

```
SOCKET_FILTER load: fd=3  errno=0  OK — verifier+JIT accepted
XDP           load: fd=4  errno=0  OK — verifier+JIT accepted
bpf_jit_enable = 1
```

Both a socket-filter **and an XDP program load, pass the in-kernel verifier, and JIT-compile** on this
rooted host. `unprivileged_bpf_disabled = 2` `(live)` — unprivileged BPF is off, but **root +
`CAP_BPF` bypass it** (proved by the successful load). So the v1 verdict "prod guest lacks CAP_BPF"
is **factually inverted for the owner-hub class: eBPF/XDP is genuinely available and works.**

**But it stays DEFER, not ADOPT, for three honest reasons:**
1. **No measured need.** eBPF/XDP's value is **line-rate in-kernel packet steering/filtering**. The
   hub is a **crypto-bound decision node**, not a packet router (§0.5). There is no measured
   packets-per-second problem for XDP to solve. The routing *logic* ("send this hypothesis/tile to the
   right worker") is a **userspace admission decision** — P25's `WorkClass`/PSI local-decision layer
   already owns it, no network round-trip (`prior-art`).
2. **Sovereign-toolchain tension (concrete, felt this session):** a real XDP program is normally
   compiled from C with clang/LLVM → **this host has no clang** `(live: only gcc)`; I had to
   hand-assemble bytecode to probe at all. Shipping eBPF means either a C/LLVM toolchain in the build
   (fights the offline-buildable / no-C-supply-chain constraint the mesh already chose —
   `iroh_transport.rs:6-15`) or a Rust-BPF (aya) dependency. Adoptable, but a real cost to weigh.
3. **Phones cannot.** Whatever XDP the hub runs, **no courier phone can run eBPF/XDP** — so it can
   only ever be a hub-local optimization, never a mesh-wide mechanism.
- **RSS / hardware flow-steering:** this host's virtio NIC has **1 combined queue,
  `ntuple-filters off [fixed]`, `receive-hashing off [fixed]`** `(live ethtool -l/-k)` — **no RSS
  here.** A real mini-PC's physical NIC (Intel i225/i226) has a **few** RX queues + basic RSS but
  **still lacks rich hardware ntuple steering** (that's server-NIC territory). **REJECT-on-virtio;
  DEFER-on-real-NIC-hub** with a trigger: a measured multi-core RX-interrupt imbalance on the hub
  under real load — which a single-flow-per-peer QUIC mesh is unlikely to produce.

**Verdict — eBPF/XDP: AVAILABLE-on-owner-hub (PROVED loadable), DEFER-WITH-TRIGGER** (no measured
steering need + toolchain-sovereignty cost + phone-incapable). *Verdict changed:* v1 "REJECT — no
CAP_BPF in prod" → **corrected: CAP_BPF present and eBPF/XDP demonstrably works; the reason to wait is
now "no measured need + sovereign-toolchain cost," not "impossible."** **RSS: REJECT-on-virtio /
DEFER-on-real-NIC.**

### 5. Hardware-attestation for Sybil-resistant capability issuance → **FLIPS from REJECT to ADOPT-AS-AUGMENTATION on the courier phone** `[RESEARCHED]` + `[NEEDS-REAL-TARGET-HARDWARE]`

**This is the single most important correction in the redo, and the operator is right.** v1 (register
#19 / ledger C4) rejected attestation as *"no TPM/enclave surface on Firecracker (physics)."* That
premise is **entirely wrong for the real target**: the courier's actual device is a **modern phone
with real, deployed, hardware-backed key attestation.**

**The real API surface** `[RESEARCHED]`:
- **Android — Key Attestation + StrongBox Keymaster** (API 28+): generate a key in the **StrongBox HSM
  (discrete secure element with its own CPU, secure storage, TRNG)** or TEE, with
  `setAttestationChallenge(...)`. The system returns an **X.509 certificate chain rooted in Google's
  hardware attestation root**, attesting: the key is hardware-backed, the security level
  (StrongBox/TEE/software), verified-boot state, and OS patch level. A verifier checks the chain +
  the attestation extension.
- **iOS — App Attest / DeviceCheck** (iOS 14+): `DCAppAttestService.generateKey()` mints a key in the
  **Secure Enclave**; `attestKey()` returns an **Apple-signed attestation** certifying the key came
  from a **genuine, unmodified app on a genuine Apple device.** Keys are **per-app-installation, don't
  survive reinstall, aren't backed up or synced.** Apple additionally exposes an **App Attest Risk
  Metric** = approximate number of keys minted on one device (built to catch the exact
  one-device-many-keys Sybil attempt). Apple states it *"blocks thousands of fraudulent attempts
  daily."*

**Does it give genuine Sybil-resistance?** **Yes, meaningfully — it FLIPS the verdict — but as a
COST/RATE layer UNDER the existing anchor-rooted issuance, not a replacement, and with named
caveats:**
- **The real win (why it flips):** it raises the cost of minting a fake mesh identity from **~free**
  (generate a keypair — the assumption behind the original Sybil rejection) to **~the cost of a real,
  attested physical device.** N Sybils now require N genuine devices (or N genuine attestations). That
  is exactly the operator's point: *"one hardware-bound key per physical device, expensive to mint
  fake ones at scale."* Correct, and it's real deployed hardware, not a research artifact.
- **Caveat 1 — it AUGMENTS, does not REPLACE, C3.** The mesh's accepted Sybil-resistance is
  **asymmetric anchor-rooted capability issuance** (`verify_chain`, `roster.rs:252-316`; ADOPT-PROVEN
  in Batch 7 / ledger C3): N Sybils ⇒ `UnknownIssuer` ⇒ **zero authority**, identity-free. Attestation
  is a *complementary* gate: bind the anchor's *granting decision* (the residual C3 flagged — the
  per-anchor issuance budget) to **"the requesting device presented a valid hardware attestation,"**
  so the anchor's cheap-to-grant decision now also costs a real device. It slots into the
  `RootDelegationPolicy` / issuance-budget predicate as an **additional pure precondition**, checked at
  delegation-sign time.
- **Caveat 2 — new external trust anchor (sovereignty tension, must be named).** Verifying an
  attestation means **trusting Google's / Apple's attestation roots** (and Apple's App Attest servers).
  For a mesh whose stated posture is *no external dependency, offline-buildable, anchor-only trust*,
  this is a **real tradeoff** — you gain Sybil-cost, you take on Google/Apple as trust roots for the
  *attestation check only* (never for authorization, which stays anchor-rooted). Decision-square: the
  attestation is **optional evidence that raises an anchor's confidence**, degradable-closed if the
  root is unreachable — it must never become a *hard* gate that bricks a courier when Google Play
  Integrity is down.
- **Caveat 3 — the hardware root is only as strong as OEM key hygiene** `[RESEARCHED]`: the
  Android **`keybox.xml` attestation-key leaks** were a real, generation-wide break (leaked/shared TEE
  attestation keys let attackers forge attestations). Google's fix is **RKP (Remote Key Provisioning)**,
  which makes the leak-and-share model *"technically impossible."* Practical implication: **prefer
  StrongBox (discrete SE) + RKP-provisioned attestation over plain TEE**, and treat pre-RKP TEE
  attestations as lower-assurance.
- **Caveat 4 — bounds keys-per-device, not devices-per-attacker.** A determined attacker with a device
  farm (or rented attested devices) still scales — at **real hardware cost.** That's a Sybil *tax*,
  not a Sybil *wall* — which is precisely why it's an augmentation of anchor-rooted authority, not a
  standalone.

**Verdict — ADOPT-AS-AUGMENTATION (FLIPPED from REJECT).** Hardware attestation is a real, deployed,
courier-phone-native Sybil-**cost** layer that binds capability *requests* to genuine devices, sitting
under the anchor-rooted issuance that provides the actual *authorization*. It does not replace C3; it
prices the anchor's granting decision in real hardware and closes the "keys are free to mint" residual.
**This sandbox cannot probe it** (no phone, no TPM here). **`[NEEDS-REAL-TARGET-HARDWARE]` — the exact
settling probe:** on Android, a minimal app that calls `KeyGenParameterSpec.Builder(...).setIsStrongBoxBacked(true).setAttestationChallenge(nonce)`,
retrieves the X.509 chain, and a hub-side verifier that (1) validates the chain to Google's hardware
attestation root, (2) asserts the attestation extension's `securityLevel == StrongBox`, (3) binds the
challenge to a fresh mesh nonce. On iOS, `DCAppAttestService generateKey`/`attestKey`, verify the
attestation object server-side against Apple's App Attest root, then bind the mesh capability request
to that key-id and consult the Risk Metric. Until that runs on real devices, the verdict is a
*research-grounded flip*, not a probe-proven one — stated honestly.

### 6. NUMA pinning → **STAYS REJECT/no-op for the REALISTIC target — and the operator's "required" premise anticipates hardware that isn't the deployment target** `[PROBED]` (honest disagreement, decision-square framed)

The operator asserts *"NUMA pinning is required."* Re-derived honestly, **required for WHAT node
class?**
- **Courier phone:** single SoC, **no NUMA.** N/A.
- **Owner hub, realistic hardware:** a small-business self-hosted courier hub is a **single-socket
  mini-PC / NUC / small tower** (Intel N100 / i5 / i7, AMD Ryzen, or at most a single-socket used Xeon
  E-series). **All single-socket → 1 NUMA node.** This sandbox confirms the shape live: **1 socket, 1
  NUMA node, `node distances: 0→0 = 10`** `(live lscpu / numactl --hardware)`. NUMA
  pinning/interleave/locality on a single node is a **literal no-op.**
- **When NUMA pinning WOULD matter:** only **dual-socket (or multi-die with NUMA-per-die) server
  hardware** — used dual-Xeon/EPYC boxes. Those are **power-hungry, loud, expensive**, and run
  *against* the sovereign posture of cheap/quiet/low-power local hubs the whole architecture points at.

**Honest verdict — I do not agree that NUMA pinning is required for the realistic target; I think the
premise anticipates dual-socket server hardware that is not the deployment reality.** Decision-square,
per the operator's own method:
- *If the hub stays single-socket (realistic):* NUMA pinning is a **no-op today and long-term** —
  adopting it as a *requirement* would be cargo-cult. **NUMA-aware allocation code is harmless to
  write (it degrades to a no-op on 1 node) and cheap to keep dormant** — that's the safe hedge.
- *If the operator genuinely intends a dual-socket hub:* then NUMA pinning becomes **real and
  applicable** — but that is a **hardware-procurement decision** with power/noise/cost consequences
  that should be made explicitly, not assumed. **This is the honest question back to the operator:**
  is the hub single-socket (then NUMA = no-op, adopt the rejection) or dual-socket (then it applies,
  and let's cost the hardware)? The CPU *pinning* that IS useful on any hub — `taskset`/cgroup
  `cpuset` + `nice` to bind crypto-heavy work to specific cores — is **already decided (P25
  CORE-BOUND)** and `CAP_SYS_NICE` is present `(live)`; that stands regardless of NUMA.

**Verdict — NUMA pinning REJECT-as-requirement / no-op on the realistic single-socket hub** (adopt the
rejection as a *decision*), **DEFER-if-dual-socket-hardware-is-actually-chosen** (a procurement gate,
flagged back to the operator with the power/cost tradeoff). CPU core-pinning (non-NUMA) is already
ADOPTED via P25.

---

## §2. WHAT FLIPPED vs WHAT STAYED — corrected-target summary

| # | Concept | v1 (Fly) verdict | v2 (real target) verdict | Flipped? | Target-correct reason |
|---|---|---|---|---|---|
| 1 | RDMA / RoCE | REJECT (no device in Firecracker) | **REJECT** | reason-changed | consumer hub NIC + no lossless fabric (datacenter-only); phone peers ⇒ topologically impossible; CPU-bypass contradicts verify-every-recv; <10 % of crypto-bound cost `[PROBED+RESEARCHED]` |
| 2 | DPDK / io_uring / AF_XDP | REJECT (single NIC; unprivileged) | **DPDK REJECT · AF_XDP DEFER · io_uring DEFER (storage)** | **partial flip** | "single-NIC" objection dissolves on a 2-NIC hub; stays down because crypto is ~8–20× the whole stack; io_uring *loses* at few-long-lived-sockets (measured 417 ns batch=1) `[PROBED]` |
| 3 | Custom L2 framing | REJECT (no L2 segment; security) | **REJECT (raw-L2) · ADOPT LAN-UDP/mDNS discovery** | **partial flip** | shared-WiFi L2 scenario is real, but phones can't emit raw frames (weak endpoint) + cleartext-header regression; intent adopted via phone-capable LAN UDP discovery `[RESEARCHED+PROBED]` |
| 4 | RSS / eBPF / XDP | REJECT (no CAP_BPF in prod) | **eBPF AVAILABLE-on-hub, DEFER · RSS REJECT-virtio/DEFER-real-NIC** | **FLIPPED (eBPF)** | XDP+socket-filter programs **load & JIT** here (CAP_BPF works); deferred on no-measured-need + sovereign-toolchain cost + phones-can't `[PROBED]` |
| 5 | **Hardware attestation** | **REJECT** (no TPM/enclave in Firecracker) | **ADOPT-AS-AUGMENTATION** | **FLIPPED — the big one** | courier phones have real StrongBox/Secure-Enclave attestation; prices Sybil-minting at real-device cost, under anchor-rooted issuance; caveats: new Google/Apple trust root, keybox-leak history (prefer StrongBox+RKP), degrade-closed `[RESEARCHED / NEEDS-REAL-HARDWARE]` |
| 6 | NUMA pinning | REJECT (1 socket/1 node) | **REJECT-as-requirement / no-op** (question back to operator) | reason-sharpened | realistic hub = single-socket = 1 NUMA node (confirmed live); "required" anticipates dual-socket server HW that isn't the target; DEFER-if-dual-socket-procured `[PROBED]` |

**Net:** **2 clean flips** (item 4 eBPF: REJECT→AVAILABLE/DEFER; item 5 attestation: REJECT→ADOPT —
the most important correction). **2 partial flips** (item 2 DPDK single-NIC objection dissolves but
payoff stays negative; item 3 shared-L2 scenario is real but raw-L2 stays rejected while LAN-UDP
discovery is adopted). **2 stay REJECT on target-correct reasons** (item 1 RDMA — topology + crypto
dominance; item 6 NUMA — single-socket reality). The v1 doc's error was **uniform**: it attributed to
*physics* (device absent in Firecracker) what was really *the wrong substrate model*. On the real
target, the honest gate is no longer "does the device exist" but **"does the weakest endpoint (a
sandboxed phone) support it, and does it optimize the cost that actually dominates (crypto, not
transport)."**

---

## §3. Probes I built and ran (reproducible)

All in the session scratchpad; each is a few lines of C, compiled with the system `gcc` (no clang):

| Probe | File | What it measured / proved |
|---|---|---|
| BPF/XDP loadability | `bpf_probe.c` | socket-filter **and** XDP programs load via `bpf()` (verifier+JIT accept); `bpf_jit_enable=1` — CAP_BPF works on owner-hub-class Linux |
| syscall tax + io_uring | `iouring_bench.c` | getpid 181 ns; io_uring NOP batch=1 **417 ns** (loses), batch=64 **77 ns**; `io_uring_setup features=0x3fff` |
| packet-stack cost | `udp_bench.c` | UDP loopback full TX+RX stack: 64 B = 6,882 ns; 1200 B = 7,253 ns |
| crypto cost | `openssl speed ed25519` | Ed25519 verify ≈ 69 µs, sign ≈ 27 µs — the per-recv cost that dominates |
| hardware inventory | `lscpu`/`capsh`/`ethtool`/`ls /dev/*` | 1 socket/1 NUMA; full caps incl. cap_bpf; virtio NIC 1 queue, ntuple off; no infiniband; no TPM |

*(These files are dev-tooling probes, not product code — they write nothing to the mesh.)*

---

## §4. Anu / Ananke check

**Anu (derivable, not asserted):** every flip and every stay is grounded in **a probe I ran this
session** — I *loaded real BPF/XDP programs*, *benchmarked io_uring against a raw syscall*, *measured
the UDP stack and Ed25519 verify*, and *inventoried the hardware live* — not in analogy to another
system. The two verdicts I could **not** probe (attestation, item 5; and any real-phone raw-L2, item
3) are tagged `[NEEDS-REAL-TARGET-HARDWARE]` / `[RESEARCHED]` with the **exact settling probe named**,
never dressed up as proven. Weakest Anu link, named: the crypto-dominance ratio uses OpenSSL's
Ed25519 (~69 µs); the mesh's real lib is faster (~15–30 µs) while ML-DSA-65 adds ~40 µs — I gave the
whole range and showed the conclusion (crypto ≫ transport) holds across all of it.

**Ananke (structural, not hoped):** the deepest corrections are **structural, target-independent**
facts, not measurements that could drift: **(1)** the mesh is heterogeneous — the *weakest endpoint is
always a sandboxed phone*, which structurally gates RDMA/DPDK/raw-L2 no matter how capable the hub is;
**(2)** *every recv verifies both signature legs* (`iroh_transport.rs:372`), which structurally makes
RDMA's CPU-bypass value incompatible with the mesh and makes transport a fixed <10 % of per-message
cost; **(3)** attestation is structurally a *cost* layer (keys-per-device), not a *wall*, which is why
it augments rather than replaces anchor-rooted authorization (`roster.rs:252-316`). These three
structural facts, not the ns numbers, are what actually decide the cluster on the corrected target.

---
---

# APPENDIX — SUPERSEDED v1 (evaluated against the WRONG target: Fly Firecracker)

> Preserved verbatim for honesty / audit trail. **Every verdict below was derived against Fly.io
> Firecracker microVMs, which the operator has confirmed are no longer the deployment target.** Read
> the v2 above for the corrected verdicts. Kept because: (a) the *dev-host probe data* in §0.1 below
> is still accurate live data; (b) the *prior-art reconciliation* (§2 below) with `discovery.rs` /
> `framing.rs` / `iroh_transport.rs` and P24/P25/P26 is unchanged and still load-bearing; (c) the
> reasoning error (wrong substrate model) is instructive and should not be silently erased.

---

# BATCH 5 — Network / Hardware Cluster: Findings & Verdicts (2026-07-17) [SUPERSEDED]

> Research + audit only. Writes no product code. Evaluates the network/hardware concepts from the
> Bebop2 mesh brainstorm (`01-RAW-DIALOGUE-PART-A.md`) against **real local code** and the **real
> deployment substrate**, per the operator's binding instruction (`00-SOURCE-PROMPT.md:10-23`) and
> `AGENTS.md` DECART discipline (every load-bearing claim carries a `file:line`, a live-command
> ground, or an explicit epistemics tag).
>
> **Cluster scope:** gossip protocol design · custom L2/Ethernet 32-byte framing · RDMA/RoCE ·
> AF_XDP vs full DPDK · flow-steering / RSS / eBPF tensor-aware routing · CPU pinning / NUMA /
> isolcpus · io_uring.
>
> **Epistemics tags used:** `(live)` = verified by a command I ran this session · `(file:line)` =
> read from actual source · `(prior-art)` = decided in a sibling blueprint already merged into the
> corpus · `(training-knowledge)` = asserted from general knowledge, flagged for re-verification ·
> `(inference)` = my derivation from the above.
>
> **Rejection rule (operator override, `00-SOURCE-PROMPT.md:15-16`, 23):** complexity / rewrite-cost
> is **NOT** a valid rejection reason for this arc. "The hardware does not exist" **IS** valid
> (physics/correctness). Every REJECT below is a physics/absence rejection, never a complexity one.

---

## 0. The decisive fact first: two substrates, and what each can physically do

Everything in this cluster resolves against **where the code actually runs**. There are two distinct
substrates, and the dialogue's networking proposals implicitly assume a third (bare metal) that does
not exist here.

### 0.1 Substrate A — the dev / harness host (where agents + kernel tests run)

Live-probed this session:

| Property | Value | Source |
|---|---|---|
| Kernel | `6.8.0-134-generic` | `uname -r` **(live)** |
| Virtualization | QEMU/KVM guest, **`virtio_net`** NIC | `ethtool -i eth0` → `driver: virtio_net` **(live)** |
| Capabilities | **full set**: `cap_net_raw`, `cap_net_admin`, `cap_bpf`, `cap_perfmon`, `cap_sys_nice`, `cap_ipc_lock`, `cap_sys_admin`, `cap_checkpoint_restore`, uid=0 | `capsh --print` **(live)** |
| HugePages | **`HugePages_Total: 0`**, THP `AnonHugePages: 0 kB` (2 MiB pagesize available, none reserved) | `/proc/meminfo` **(live)** |
| isolcpus | **absent** — `/proc/cmdline` has no `isolcpus=` | `cat /proc/cmdline` **(live)** |
| io_uring | syscall present in kernel | `grep io_uring_setup /proc/kallsyms` **(live)** |
| RDMA | **absent** — no `/dev/infiniband`, no `/sys/class/infiniband` | `ls` **(live)** |
| CPU topology | 8 vCPU = 4 physical cores × 2 SMT, 1 socket, **1 NUMA node** | `lscpu` (P25 §1.1) **(prior-art)** |

So the dev host is a **privileged QEMU guest**: it *could* technically attach XDP/eBPF/raw sockets
(root + `cap_net_raw`/`cap_bpf` present), but it has **no RDMA hardware**, **one shared virtio NIC**,
**no reserved hugepages**, **no isolated cores**, and — critically — **it is a single node with no
peer to talk to over any of these fabrics.**

### 0.2 Substrate B — the deployed production artifact

The deployed dowiz artifact is **not a server mesh at all**. It is a static-SPA server:

- `Dockerfile:52-68` **(file:line)** — final stage is `FROM scratch`; it copies exactly one thing: a
  single static Rust binary (`native-spa-server`) + the SPA `dist` + CA certs. There is **no OS
  beneath it**, no shell, no privileged networking, no packet tooling.
- `Dockerfile:3-5` **(file:line)** — "the legacy centralized server (apps/api + apps/worker, Fly,
  Supabase) was **DROPPED**." Confirmed by `apps/*` deleted at HEAD (P24 §7 adoption note,
  `roadmap §1.2`) **(prior-art)**.
- Prod host = `dowiz.fly.dev` (`.claude/CLAUDE.md:39` **(file:line)**). Fly.io runs guest workloads
  as **Firecracker / Cloud-Hypervisor microVMs**: minimal device model, **virtio-net only, no
  SR-IOV, no NIC passthrough, no RDMA, unprivileged guest** **(training-knowledge — Fly's public
  architecture; the implication holds for any unprivileged cloud microVM regardless of exact
  hypervisor)**.

**Consequence, stated once:** on Substrate B, *every* kernel-bypass / raw-NIC / RDMA proposal in this
cluster is **physically undeployable** — not "hard," not "complex," but *the device does not exist in
the guest*. This is the valid physics rejection the operator's own rule allows.

### 0.3 The mesh's transport already exists — and already made these choices

The Bebop2 mesh carrier is **not** a greenfield. It lives in `bebop-repo/bebop2/proto-wire/` and is
built and tested:

- `iroh_transport.rs:1-25` **(file:line)** — despite the filename, the real carrier is **QUIC over
  UDP via `quinn` + `rustls` + `ring`** (a real node-to-node carrier, not a stub). iroh itself was
  **rejected** for an `ed25519-dalek` version conflict and the offline-build / no-C-supply-chain
  requirement (`iroh_transport.rs:6-15`).
- `iroh_transport.rs:22-25` **(file:line)** — "**NAT traversal is a deployment concern**… Trigger:
  add an iroh/derp relay or a STUN-less hole-punch layer if a real deployment needs it." NAT/discovery
  was *deliberately* deferred, not forgotten.
- `discovery.rs:1-7` **(file:line)** — a real **anti-entropy full-roster gossip** protocol over that
  QUIC transport, **zero new dependencies**. "'Just use libp2p' was **rejected**: it cannot build
  offline (native deps) and fights the anchored allow-list trust model."
- `framing.rs:1-11` **(file:line)** — a byte-deterministic **length-prefixed envelope** carrier format
  (`[u32 LE len][envelope bytes]`) explicitly designed to be *carrier-neutral* across QUIC and WSS.

This is the prior art the dialogue's cluster must **extend, not replace**. Several proposals (raw L2,
libp2p-shaped DHT) are things this codebase already evaluated and rejected on the same grounds I reach
below.

---

## 1. Verdicts per concept

### 1.1 Gossip protocol design → **EXTEND-EXISTING** (buildable now, pure logic)

**What the dialogue proposes:** "DecisionUnit gossip / JIT-compilation swarm intelligence," epoch
propagation, flocking broadcast-to-neighbors, Merkle-bisection for state divergence.

**What already exists** (`discovery.rs` **(file:line)**):
- `GossipAgent` with `tick()` (dial known peers, push roster, merge response, `discovery.rs:222-255`)
  and `listen_loop`/`handle_conn` (serve inbound, merge, reply, `discovery.rs:295-374`).
- `PeerDirectory` — content-addressed `BTreeMap<PeerId,_>`, deterministic order, `merge()` anti-entropy
  (`discovery.rs:70-79`), `evict_revoked()` (`discovery.rs:82-93`).
- `snapshot_root()` — an **FNV-1a fingerprint over the sorted peer set** (`discovery.rs:97-104`): two
  directories with the same content yield the same root regardless of insertion order. **This is
  already a Merkle-root-lite divergence primitive** — the exact "detect that two nodes diverged
  cheaply" function the dialogue's Merkle-bisection idea starts from.

**Verdict — EXTEND-EXISTING.** The dialogue's gossip ideas are *additions to a working protocol*, not a
new one:
- **Epoch / HLC field** → add an `epoch`/HLC scalar to the gossip payload (the roster wire,
  `discovery.rs:127-141`) so peers converge on epoch without a clock master. Pure deterministic logic,
  **buildable now**, zero new deps. (Note: `iroh_transport.rs:391` **(file:line)** currently reads
  `SystemTime::now()` for capability expiry on the recv path — the HLC/determinism thread from the
  dialogue connects precisely here; an HLC would be the deterministic replacement, and this is worth a
  cross-reference to the epoch/consensus batch.)
- **Delta-gossip** → today `tick()` ships the *whole* roster every round (`to_wire()`,
  `discovery.rs:233`). Fine for an allow-list of tens; if the roster grows, gossip only the diff since
  the peer's last-seen `snapshot_root`. Falsifiable trigger: roster size where full-roster bytes/round
  become measurable — not now.
- **Merkle bisection** → `snapshot_root` is the leaf; a tree over sub-ranges enables O(log n) divergence
  localization. **DEFER-WITH-TRIGGER**: build only when a real multi-node divergence-debugging need is
  measured; the flat fingerprint is sufficient at current scale.

**Epistemics:** the "already exists" claims are `(file:line)`-grounded; the extensions are `(inference)`
from the existing shapes.

### 1.2 Custom 32-byte L2 / Ethernet framing (Dst/Src MAC + EtherType + TileID/EpochID/HypothesisID/Flags/Seq) → **REJECT-on-physics (carrier) + REDUNDANT (design)**

**Two independent, each-sufficient reasons:**

1. **Physics (carrier):** raw L2 framing means `AF_PACKET`/raw Ethernet sockets, which require (a)
   `CAP_NET_RAW` **and** (b) an actual **L2 broadcast domain** shared by the peers. Prod Substrate B is
   Fly Firecracker microVMs (§0.2): unprivileged, virtio-net, and Fly machines are **routed at L3 /
   WireGuard-meshed — there is no shared Ethernet segment between two machines** **(training-knowledge)**.
   Even on the dev host, `CAP_NET_RAW` is present **(live)** but it is **one VM with no L2 peer**. A raw
   Ethernet frame has nowhere to go on either substrate.

2. **Design-redundant + security-regressive:** `framing.rs:1-22` **(file:line)** already provides a
   byte-deterministic, length-prefixed, carrier-neutral envelope with a fail-closed 1 MiB bound. The
   header fields the dialogue wants (`TileID/EpochID/HypothesisID/Sequence`) are **application metadata**
   — they belong **inside the signed `SignedFrame` payload** (`iroh_transport.rs:345-357` sends, `:360-402`
   verifies every recv through the `RequireBoth` hybrid gate). Putting them in a **cleartext L2 header**
   would place routing-critical fields *outside* the signature envelope, which `framing.rs:54-58` already
   flags as the downgrade hazard to avoid ("carry version inside the signed SignedFrame domain so a MITM
   cannot flip it"). Raw L2 framing **throws away the mesh's entire authenticity model** to save a
   transport layer that QUIC already provides.

**Verdict — REJECT the raw-L2 carrier** (physics, both substrates). The header *fields* are real and
useful → **fold them into the signed envelope / gossip payload schema (EXTEND, §1.1)**, where they are
authenticated. Falsifiable un-reject trigger: a bare-metal single-L2-segment deployment where nodes are
Ethernet-adjacent *and* line-rate framing (not crypto) is the measured bottleneck — not this infra.

### 1.3 RDMA / RoCE → **REJECT-on-physics (hardware absent, both substrates)**

- Dev host: **no RDMA hardware** — `/dev/infiniband` and `/sys/class/infiniband` both absent **(live)**.
- RDMA needs an RNIC (InfiniBand HCA, or a RoCE-capable Ethernet NIC with hardware verbs offload) exposed
  to the guest; RoCEv2 additionally needs a **lossless, PFC/DCB-configured Ethernet fabric**
  **(training-knowledge)**. Hetzner standard cloud VMs and Fly Firecracker expose **virtio-net only** —
  no verbs, no SR-IOV, no RNIC **(training-knowledge + `ethtool` live shows virtio_net)**.
- This is the cleanest physics rejection in the cluster: **the device does not exist.**

**Verdict — REJECT-on-physics.** Falsifiable un-reject trigger: the mesh is deployed on **bare metal
with a Mellanox/InfiniBand or RoCE-capable NIC** and a lossless fabric — a hardware procurement event,
not a code decision.

### 1.4 AF_XDP (zero-copy, kernel-bypass-lite) → **REJECT-on-physics (prod) / DEFER-WITH-TRIGGER (dev, near-zero payoff)**

- **Prod (Substrate B):** Fly Firecracker microVM is unprivileged (no `CAP_NET_RAW`/`CAP_BPF` to the
  app) and serves a `scratch` static binary — AF_XDP cannot attach. **Undeployable.**
- **Dev (Substrate A):** the `virtio_net` driver **does** have an XDP data path, and `CAP_NET_RAW` +
  `CAP_BPF` are present **(live)**, so an `AF_XDP` socket *could* attach in a lab. But the payoff is
  near-zero here, for three independent reasons:
  1. AF_XDP's win is **bypassing the kernel network stack for line-rate packet I/O**. The mesh carrier is
     **QUIC**, which *needs* the UDP/IP stack (`quinn` binds a normal UDP socket, `iroh_transport.rs:256`,
     `:308` **(file:line)**). AF_XDP and QUIC are not composable without re-implementing UDP/QUIC on top of
     raw frames — a from-scratch stack.
  2. The measured mesh bottleneck is **signature verification** (Ed25519 + ML-DSA-65 per recv,
     `iroh_transport.rs:380` **(file:line)**), which is µs–ms of CPU, **orders of magnitude above** any
     packet-copy saving.
  3. virtio-net is *already a software path*; "kernel bypass" over a paravirtual NIC saves little vs. bare
     metal **(training-knowledge)**.

**Verdict — REJECT-on-physics for prod; DEFER-WITH-FALSIFIABLE-TRIGGER for dev.** Trigger: a **bare-metal
multi-node** deployment where a profile shows the **kernel UDP copy path (not crypto) dominates** mesh
latency. Until that exact measurement exists, AF_XDP is capacity-not-need.

### 1.5 Full DPDK (total kernel bypass) → **REJECT-on-physics (both substrates)**

- DPDK requires binding a NIC to **VFIO/uio** (removing it from the kernel entirely), reserved
  **hugepages**, a **poll-mode driver spinning a full core**, and dedicated CPUs **(training-knowledge)**.
- Both substrates have **one shared NIC** (`eth0`, virtio_net **(live)**). Binding it to DPDK **kills all
  normal networking** — including SSH to the dev box and the agents' own outbound LLM API calls (the D-class
  work in P25 §3.4 **(prior-art)**). On Fly Firecracker there is **no NIC to pass through** at all.
- HugePages are currently **0/reserved (live)**; DPDK's hugepage + core-dedication demands compound the
  impossibility on a shared 4-core VM.

**Verdict — REJECT-on-physics.** Not complexity — DPDK on a single-NIC shared cloud VM is *self-defeating
by construction* (it removes the machine's only network path). No falsifiable trigger short of a
dedicated bare-metal packet-processing appliance the dowiz mesh is not.

### 1.6 Flow steering / RSS / eBPF tensor-aware routing → **REJECT-on-physics (RSS) / ALREADY-RULED (eBPF)**

- **RSS / hardware flow steering:** RSS distributes flows across cores via a multi-queue NIC hashing
  packets, and *steering* specific flows to specific cores needs hardware **ntuple filters**. virtio-net
  offers limited multi-queue but not real hardware ntuple steering; Firecracker exposes even less
  **(training-knowledge)**. **REJECT-on-physics (prod).**
- **eBPF-based "tensor-aware routing":** two blocks, one decisive:
  1. **ALREADY-RULED by P24 (prior-art).** `BLUEPRINT-NATIVE-TELEMETRY-…:169-194` **(file:line, prior-art)**
     deliberately **does not load real eBPF**. It *ports the eBPF/perf ideas* — in-kernel aggregation →
     userspace relaxed atomics, sampling, ring buffers — into **pure-Rust std code** ("Ported as: per-site
     always-on aggregates … the ring is §3.2's `ring.rs`"). The repo's stance is: **adopt the technique,
     not the eBPF loader**, because a real eBPF program fights the offline-buildable / no-C-toolchain
     sovereign constraint (`iroh_transport.rs:6-15` same constraint **(file:line)**).
  2. **Physics (prod):** loading XDP/tc eBPF needs `CAP_BPF`/`CAP_NET_ADMIN` — present on dev **(live)**,
     **absent** in the prod Firecracker guest (§0.2).
- The **routing *logic*** the dialogue wants ("route this hypothesis/tile to the right worker") is real and
  already has a home: it is a **userspace admission/scheduling decision**, and P25 already built the
  local-native decision primitive for exactly this — `WorkClass` admission over PSI gauges, computed
  locally, **never a network round-trip** (`BLUEPRINT-WAVE-SCHEDULING-…:212-222` LOCAL-DECISION rule
  **(file:line, prior-art)**).

**Verdict — REJECT eBPF-as-mechanism (physics + P24's ported-ideas stance); the tensor-aware routing
*logic* → EXTEND the existing userspace gossip (§1.1) + P25 admission layer.**

### 1.7 CPU pinning / NUMA / isolcpus → **ALREADY-DECIDED (pinning) / REJECT (NUMA) / DEFER-host-level (isolcpus)**

- **CPU pinning → ALREADY-EQUIVALENT / EXTEND P25.** P25 already owns this as the binding **CORE-BOUND
  rule**: CPU-bound work is pinned `taskset -c 0,2,4,6` (one thread per SMT sibling pair) = 4 strict-core
  slots, `nice 10` (`BLUEPRINT-WAVE-SCHEDULING-…:224-232, 234-250` **(file:line, prior-art)**).
  `CAP_SYS_NICE` + `CAP_SYS_ADMIN` are present **(live)** so `taskset` / cgroup `cpuset` work inside the
  container. **Nothing to re-derive — adopt P25.**
- **NUMA → REJECT (no-op on this host).** P25 §2.4 **(prior-art)** verified **1 socket, 1 NUMA node**
  (`lscpu`) — NUMA pinning/interleave/locality **have no effect**. A rejection *as a decision*, not an
  oversight.
- **isolcpus → DEFER-host-level-WITH-TRIGGER.** `/proc/cmdline` has **no `isolcpus=`** **(live)**;
  enabling it is a **kernel boot parameter requiring a reboot of the Hetzner host** (root is available but
  this is host-level and disruptive) and **does not exist on Fly at all**. On shared cloud VMs, cgroup
  `cpuset` + `nice` (P25's mechanism) already delivers most of the isolation **reversibly**. Trigger:
  only a dedicated bare-metal mesh node justifies true core isolation.

### 1.8 io_uring → **ALREADY-RULED out of scope / DEFER-WITH-TRIGGER**

- io_uring **is** available on this kernel (`io_uring_setup` in kallsyms, 6.8 **(live)**) — so this is a
  *scope* decision, not a physics one.
- **P25 §2.5 already scoped it OUT** (`BLUEPRINT-WAVE-SCHEDULING-…:200-206` **(file:line, prior-art)**):
  outbound HTTPS to the LLM API is a few long-lived sockets (epoll-class blocking I/O is already fine) and
  "dowiz's `ureq` adapters are blocking-by-design." **P26 confirms** it: "io_uring, NUMA work | Out of
  scope | P25 §2.4–2.5 already ruled" (`BLUEPRINT-MEMORY-OPTIMIZATION-…:451` **(file:line, prior-art)**).
- **P24 uses io_uring only conceptually** — as the SPSC ring **memory-ordering reference**
  (`io_uring_smp_load_acquire`/`store_release`), not the real syscall
  (`BLUEPRINT-NATIVE-TELEMETRY-…:210-214` **(file:line, prior-art)**). The *idea* (shared-ring SPSC
  discipline) is adopted in pure Rust; the *syscall* is not.
- **Verdict — ALREADY-RULED / DEFER-WITH-TRIGGER.** For the **network** layer: not applicable (QUIC/UDP,
  blocking `ureq`). The *only* place io_uring could ever earn its place is a **syscall-heavy local file
  I/O** path — e.g. the tensor-arena / block-store the sibling batches are designing — **if** it becomes a
  measured bottleneck. That is a storage decision, not a network/hardware one, and belongs to that batch.

---

## 2. Prior-art reconciliation table (extend, never contradict silently)

| This cluster's concept | Prior decision found | This batch's relation |
|---|---|---|
| CPU pinning, core-bound | P25 CORE-BOUND rule, `taskset -c 0,2,4,6`, `nice 10` (`…WAVE-SCHEDULING…:224-250`) | ADOPT unchanged |
| NUMA | P25 §2.4 verified irrelevant (1 socket/1 node) | ADOPT the rejection |
| io_uring | P25 §2.5 out of scope; P26:451 confirms | ADOPT the scoping |
| eBPF / perf ideas | P24 ports the *ideas* to userspace atomics, does **not** load eBPF (`…TELEMETRY…:169-194`) | ADOPT the stance; routing logic → userspace |
| Gossip / roster / divergence | `discovery.rs` anti-entropy gossip + `snapshot_root` fingerprint | EXTEND (epoch/HLC field, delta-gossip) |
| Transport carrier | `iroh_transport.rs` QUIC/`quinn`; libp2p & iroh-DHT already rejected | EXTEND; do **not** replace with raw L2 |
| Envelope framing | `framing.rs` length-prefixed carrier-neutral, 1 MiB fail-closed | EXTEND payload schema; do **not** move fields to cleartext L2 |
| Admission = local decision | P25 LOCAL-DECISION rule (never a network round-trip) | ADOPT for "tensor-aware routing" logic |

No prior decision is contradicted by this batch. Two dialogue proposals (raw-L2 carrier, libp2p-shaped
DHT gossip) are things the codebase **already rejected on the same grounds** I reach independently here.

---

## 3. Prioritized build-order — what is REAL on this infra

### Tier 1 — REAL, buildable now (pure logic, zero new deps, runs on current substrate)

1. **Extend `discovery.rs` gossip with an epoch/HLC field** (§1.1). Smallest kernel-level abstraction,
   highest leverage, directly serves the dialogue's "epoch propagation, no clock master." Deterministic;
   also resolves the `SystemTime::now()` non-determinism at `iroh_transport.rs:391`. *Cross-ref the
   epoch/consensus batch — HLC is shared surface.*
2. **Fold `TileID/EpochID/HypothesisID/Sequence` into the signed envelope / gossip payload schema**
   (§1.2), where they are authenticated by the existing hybrid gate — **not** a raw L2 header.
3. **Adopt P25 CPU-pinning (CORE-BOUND) for any local mesh compute** (§1.7). Already decided; just apply
   `cgroup cpuset` + `nice` — no new design.
4. **Keep `snapshot_root` as the divergence primitive; add tree-bisection only on measured need** (§1.1,
   Merkle-bisection DEFER).

### Tier 2 — DEFER-WITH-FALSIFIABLE-TRIGGER (buildable only on bare-metal / measured-need-gated)

5. **AF_XDP** — only on bare-metal multi-node **with a measured UDP-copy-dominant** profile (§1.4).
6. **isolcpus** — only on a dedicated bare-metal mesh node (§1.7).
7. **io_uring** — only if a **local file-I/O** path (tensor arena / block store) becomes a measured
   bottleneck (§1.8) — a storage batch's call, not this one.
8. **Delta-gossip / Merkle-bisection** — only at a roster scale where full-roster bytes/round are
   measurable (§1.1).

### Tier 3 — REJECT-on-physics (hardware / infra absent; re-open only on a hardware-procurement event)

9. **RDMA / RoCE** — no RNIC on dev or prod; needs bare-metal RDMA hardware + lossless fabric (§1.3).
10. **Full DPDK** — single shared NIC; binding it removes the machine's only network path; Firecracker has
    no passthrough (§1.5).
11. **Raw L2 / custom Ethernet framing carrier** — no shared L2 segment on either substrate; Firecracker;
    and it discards the signed-envelope authenticity model (§1.2).
12. **RSS / hardware flow steering** — virtio / Firecracker expose no hardware ntuple steering (§1.6).

---

## 4. Anu / Ananke check

**Anu (derivable, not asserted):** every REJECT is grounded in a **live probe** (`ls /dev/infiniband`
empty; `ethtool` = virtio_net; `capsh`; `/proc/cmdline`; `/proc/meminfo` hugepages 0) or a **`file:line`**
(the `scratch` Dockerfile, the QUIC transport, the length-prefixed framing) — not in "this is too
complex." The one substrate claim I could not directly verify — that Fly prod is specifically Firecracker
— is tagged `(training-knowledge)`, and the rejection **does not depend on the exact hypervisor**: it
holds for *any* unprivileged cloud microVM with a virtio NIC, which the deployed `scratch` artifact
provably is. Weakest Anu link, named: the AF_XDP-on-virtio-net "near-zero payoff" rests on the *asserted*
crypto-dominates-copy profile (`iroh_transport.rs:380` shows the per-recv hybrid verify exists, but I did
not benchmark it this session) — hence AF_XDP is **DEFER-WITH-TRIGGER (measure first)**, not a hard
reject.

**Ananke (structural, not hoped):** the mesh's authenticity is structural — every recv re-verifies both
signature legs (`iroh_transport.rs:360-402`), which is *why* moving routing fields to a cleartext L2
header (§1.2) is a security regression, not a neutral swap. The gossip determinism is structural — the
`BTreeMap`/`snapshot_root` design (`discovery.rs:41-104`) makes divergence detection order-independent by
construction. The extend-don't-replace posture is enforced by the fact that the transport, framing, and
gossip **already build and pass tests** — a from-scratch L2/RDMA/DPDK carrier would be *unverifiable* on
this infra (no hardware to test against), i.e. it could never satisfy the repo's own falsifiable-proof
rule, which is itself the physics rejection restated.

---

## 5. Open cross-batch handoffs (flagged, not resolved here)

- **HLC / epoch determinism** (§1.1) overlaps the epoch/consensus batch — the `SystemTime::now()` at
  `iroh_transport.rs:391` is the concrete determinism defect an HLC would fix; coordinate so the field is
  designed once.
- **io_uring for local storage** (§1.8) is a live question **only** for the tensor-arena / block-store
  batch, not for networking — handed off there with the "measure first" trigger attached.
- **NAT traversal / relay** (`iroh_transport.rs:22-25`) is the real deployment gap the dialogue's L2/RDMA
  proposals were (wrongly) reaching for — if the mesh ever needs to cross NATs between cloud microVMs, the
  answer is a **QUIC-layer relay/hole-punch (iroh/derp/STUN)**, not raw L2 — flagged for the deployment/
  topology batch.
