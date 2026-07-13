# ADR — agent-governance requires NO sandbox (durable non-goal)

- Status: ACCEPTED (design record — closes the "wrap agent-governance in a microVM" proposal)
- Date: 2026-07-13
- Supersedes/relates: DK-07 (docker-swap blueprint),
  [MICROVM-ISOLATION-RESEARCH-2026-07-13.md](./MICROVM-ISOLATION-RESEARCH-2026-07-13.md) §2, §6,
  [BLUEPRINTS-DOCKER-SWAP.md](../docker-swap/BLUEPRINTS-DOCKER-SWAP.md) DK-07.
- Part of: MV-05 (explicit non-goals / durable rejection record).

## Context

The initial microVM-isolation framing floated `agent-governance` as a microVM
sandbox candidate — the hypothesis being that it "runs untrusted agent-generated
code." That hypothesis was checked against the actual code
(`agent-governance/index.ts`) rather than assumed, and it is **false**.

`agent-governance/index.ts` is a **zero-dependency, pure-function TEXT/VOICE
policy filter**. Its axes (gender, profanity, archetype, GodRelation), its
HARD-BAN list (voodoo), and its drift/error-pattern scanners operate on *strings
and config* — they classify and constrain agent **output**. The module executes
nothing untrusted: there is no `exec` / `spawn` / `eval` of foreign code, no
dynamic plugin loading, no child process. It runs in the same trust domain and
the same process as the rest of the trusted application.

Therefore there is **nothing to sandbox**. Wrapping a pure, trusted, in-process
text filter in a microVM (Firecracker/Kata, which hard-require KVM) would isolate
trusted code from itself — the exact "isolation theater" the research doc warns
against — while adding a KVM host requirement, a ~125–200ms boot cost, and a
guest-kernel/networking ops surface for zero security benefit.

## Decision

- `agent-governance` requires **no sandbox** — not a microVM, not a WASM
  component boundary, not a container. It remains a plain, in-process pure
  function module.
- Its isolation posture is the same as the rest of the trusted application:
  it executes no untrusted code, so the threat model that motivates microVM
  (hardware-enforced isolation of genuinely untrusted non-WASM code) does not
  apply to it.
- The **real** microVM use case in this architecture is the future
  **Dev-agent-tier** capability-port adapters and untrusted third-party MCP
  servers that ship as native processes — code the dowiz team did not write and
  cannot fully vet (see microvm-isolation research §2/§4, MV-03). `agent-governance`
  is explicitly **not** in that set.

## Alternatives considered

- **A — wrap agent-governance in a Firecracker/Kata microVM:** REJECTED. It
  executes no untrusted code; the VM boundary isolates trusted code from itself
  (isolation theater), adds a hard KVM dependency, and costs boot latency for no
  security gain.
- **B — make it a WASM Component for "isolation":** REJECTED for the same reason
  — there is no untrusted payload to attenuate. WASM Components are reserved for
  the Dev-agent-tier untrusted-extension path (MV-02), which this module is not.
- **C — leave as plain in-process pure functions (chosen):** correct fit. The
  module's risk surface is logic/correctness (e.g. a mis-tuned HARD-BAN list),
  addressed by unit tests, not runtime isolation.

## Consequences

- **+** No KVM/host-virtualization requirement, no VM boot latency, no extra
  ops surface for a component that needs none of it.
- **+** This ADR is the durable rejection record: the next time someone proposes
  microVM-wrapping `agent-governance`, this decision closes the proposal as
  already-decided (MV-05 falsifiability).
- **−** None of substance. The only "cost" is that `agent-governance` shares the
  trust domain of its host process — which is acceptable because it executes no
  untrusted code by construction.

## Open items / verification

- **Proof (Mandatory Proof Rule, RED):** grep-based — `agent-governance/index.ts`
  contains **no** call to `exec` / `spawn` / `child_process` / `eval` /
  `Function(` / dynamic `require` of untrusted input. Any such introduction would
  invalidate this ADR and re-open the sandbox question.
- **HUMAN — none.** This is a documentation-only durable rejection; zero
  engineering risk.
