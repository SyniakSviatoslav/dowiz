# Bebop Energy Autonomy & EW-Resilience Architecture

> Status: vision captured 2026-08-18 (autopilot). Concrete modules tracked below.
> Applies to: Bebop bare-metal AArch64 + dowiz delivery OS.

## 1. Energy Autonomy (milliamp squeezing)

1. **big.LITTLE / DynamIQ core pinning.** Green-thread scheduler sets a hard CPU
   affinity mask. Background tasks (mesh, VSA packet wait) run exclusively on
   LITTLE cores (A53/A55/A510, 4-6x less energy); perf cores sleep deep until
   heavy peaks.
2. **WFI/WFE instead of polling.** `wfi` (Wait For Interrupt) / `wfe` (Wait For
   Event) stop the core clock immediately, entering hardware low-power state until
   an interrupt/timer arrives. No busy-wait pipeline activity.
3. **PSCI (Power State Coordination Interface).** Direct `svc #0` to firmware
   (EL3/EL2) to power off whole clusters/cores at low load, and drive DVFS from
   the logistics engine.
4. **Memory-bus optimization.** Whole working set fits L2/L3 (<200-300 KB, static
   arenas). External DDR enters Self-Refresh / Deep Power Down ~95% of the time.
5. **Radio duty cycling.** VSA protocol uses hard time-synchronized duty cycling:
   nodes sleep on timer, wake microseconds for dense binary packet exchange, then
   depower the radio at the hardware power-pin level.
6. **Branchless execution.** CSEL (conditional select) turns critical branches into
   linear pipeline flow, avoiding mispredict flushes. (Already in bebopc.)

## 2. EW-Resistance (frontline data integrity)

1. **Dead reckoning.** Local node interpolates state (trajectory, delivery status)
   from last valid data; corrects smoothly when a new packet arrives.
2. **VSA-native FEC.** Fountain codes (rateless) on binary VSA packets: any K of N
   coded chunks reconstruct the block; no ACK/NACK handshakes; survives 40% loss.
3. **Resilient mesh re-sync.** VSA bundles merge autonomously, deterministic,
   conflict-free (CRDT + vector clocks).
4. **Store-and-forward with priority.** Critical packets (alerts, routes,
   coordinates) preempt; routine data parks in static arenas for async delivery.

## 3. Space-Grade / DTN

1. **CGR (Contact Graph Routing).** Route on a time-space graph (schedules of
   drones/vehicles/terminals) — zero discovery traffic, no ELINT signature.
2. **Software TMR / SEU protection.** Critical arena state duplicated 3x with
   background bit-voting; flips corrected by majority without kernel crash.
3. **Credit-based flow control.** Transmitter sends nothing until a hardware
   "credit" for free arena space arrives (SpaceWire-style).
4. **Delta encoding.** Only state deltas fly; all nodes hold a deterministic world
   model.
5. **Split-brain convergence.** CRDT + vector clocks merge autonomously after
   long blackouts.

## 4. Experimental Paradigms

1. **VSA semantic modulation.** Entities encode directly into D=8192/16384-bit
   hypervectors; noise is orthogonal to signal, dot-product extracts from 70%
   jamming; superposition carries multiple streams per pulse.
2. **Persistent homology.** Mesh as topological manifold; holes (jammed nodes)
   computed from curvature, routes change before user notices, zero control traffic.
3. **Homomorphic network coding.** Relays emit random GF(2^8) linear combos of
   buffered blocks — uniform white noise to SIGINT; receiver reconstructs via SMT.
4. **STDP neuromorphic routing.** Synaptic weights on mesh links; successful
   delivery strengthens, loss decays; self-organizing nervous system.

## 5. Thermal-Aware Design

1. **Thermal-aware scheduler.** Treat die floorplan as a resource; migrate work
   to cold segments proactively; hold 35-40C (min leakage).
2. **Predictive pacing.** Heat cost per code block; stretch execution in time to
   avoid thermal spikes / DVFS thrash.
3. **Instruction-level thermal.** Interleave hot (NEON) with cold (wait/memory)
   instructions; avoid hot spots.
4. **IR signature.** Stable low temperature = stealth vs thermal optics.

## 6. Energy Metering & NTC

1. **Real-time joule accounting.** PMU counters + power model → deterministic
   joules per task; energy budget cuts secondary tasks first.
2. **EM-aware runtime.** PMIC/current sensing detects EM-induced voltage drift;
   blocks sensitive ops, depowers transceivers.
3. **Acoustic/vibration trigger.** IMU/mic spike → freeze arenas, zero keys/routes.
4. **MEP / Near-Threshold Computing.** Hold background cores at minimum-energy
   point (~0.5-0.6V); avoid DVFS switching; U-shaped energy curve.

## 7. Formal Guarantees

- **WCET** deterministic via lock-free/wait-free critical loops.
- **Model checking** on no_std kernel (Kani-style invariants).
- **SMT-DPLL** proves schedule fits deadline at fixed MEP frequency.

## 8. Heterogeneous (GPU/NPU)

- VSA/FEC/topology are matrix/vector ops → offload to Mali/Adreno/NPU at low freq
  for best flops-per-watt. Bare-metal GPU via command buffers (Panfrost/Freedreno
  model).

## Concrete module mapping (bebop)

| Idea | Module | Status |
|---|---|---|
| WFI/WFE sleep | power.c | planned |
| PMU energy metering | power.c | planned |
| Core affinity | gt.c (green threads) | planned |
| Fountain FEC | fec.c | planned |
| CSEL branchless | native.c | DONE |
| SMT-DPLL | smt.c | DONE |
| VSA packets | vsa.c | DONE |
