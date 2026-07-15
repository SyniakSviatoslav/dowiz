# Control-Theory Layer of the Swarm

## 1. Closed-loop PID for budget auto-correction

Standard PID control law (Aström & Hägglund 1995), continuous form:

  u(t) = Kp·e(t) + Ki·∫₀ᵗ e(τ)dτ + Kd·(de/dt)

Discrete, sampled at step k for a software control plane:

  u[k] = Kp·e[k] + Ki·Σ_{i=0}^{k} e[i]·Δt + Kd·(e[k]−e[k−1])/Δt

Define the budget error as planned minus consumed resource:

  e[k] = B_est[k] − B_actual[k]      (B = remaining time-units or token-counts)

Controller output u maps to a swarm control action:
- u > 0 (ahead of budget): keep or raise parallelism c.
- u < 0 (over budget / behind): corrective action.

Swarm levers:
- Parallelism: c[k] = clamp(c[k−1] + round(u[k]), c_min, c_max).
- Simplification: sustained negative u (integral term) ⇒ drop low-priority sub-tasks / shrink scope.
- Re-plan: large |Kd·(de/dt)| (accelerating overrun) ⇒ re-plan / re-allocate lanes.

Anti-windup: saturate the integral term so a long violation does not overshoot when budget later recovers.

Source: Aström, K.J. & Hägglund, T., *PID Controllers: Theory, Design, and Tuning*, 2nd ed., ISA, 1995. (URL unverified — canonical text reference; ISBN 1-55617-516-7.)

## 2. Synchronous vs asynchronous feedback latency

- Synchronous (architect waits for verifier): loop latency L_sync = Σ service times of the blocking stage; throughput per cycle is gated by the slowest verification step.
- Asynchronous interrupt (architect keeps planning while executor runs): task is issued and planning continues; a violation arrives as an interrupt. Latency to correction L_async ≈ detection + interrupt-service, not full round-trip.

Tail-latency fact: in a parallel wave, "the slowest task bounds the wave" — a wave completes only when its last (straggler) task finishes. Synchronous feedback inherits this tail: T_wave = T_arrive + max_i(T_i) each cycle. Asynchronous feedback decouples planning from the slowest executor, so the wave's critical path is not blocked by verification; this is the basis of tail-at-scale hedging/cancellation.

Source: Dean & Barroso, "The Tail at Scale," CACM 56(2), 2013. https://research.google/pubs/the-tail-at-scale/

## 3. Dynamic lane sizing via Little's Law

Little's Law (Little 1961):

  L = λ · W

L = mean in-system items = concurrency c; λ = arrival/throughput rate; W = mean service time.

Solve for concurrency:

  c* = λ · W

Lane sizing from measured throughput: with arrival rate λ (tasks/s) and observed mean service time W (s/task), steady-state lanes c* = λ·W. For target utilization ρ<1 add headroom: c = λ·W/ρ. Then cap c with Gunther's Universal Scalability Law, since contention/coherency degrade throughput above an optimum:

  C(N) = N / (1 + σ(N−1) + κ(N−1)(N−2))

σ = contention coefficient, κ = coherency/coordination penalty. If κ>0 the optimum is N* = √(1/κ); beyond it throughput falls, so clamp c ≤ N*.

Sources:
- Little, J.D.C., "A Proof for the Queuing Formula L = λW," Oper. Res. 9(3), 1961. https://doi.org/10.1287/opre.9.3.383 ; https://en.wikipedia.org/wiki/Little%27s_law
- Gunther, N.J., *Guerrilla Capacity Planning*, Springer, 2007 (USL). https://doi.org/10.1007/978-3-540-31010-5 ; USL syntax: https://perfdynamics.blogspot.com/2008/03/syntax-for-universal-scalability-law.html ; https://www.perfdynamics.com/books.html
