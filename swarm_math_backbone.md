# Math Backbone — Real-Time Swarm Control Plane
*Exact equations + cited sources. Web verification was OFFLINE; URLs flagged [unverified].*

## 1. EMA Recalibration
Recursive estimate `E_t` updated from actual `A_t`:
```
E_{t+1} = α·A_t + (1−α)·E_t,      α ∈ (0,1]
```
**SMA-equivalent window** (center-of-mass match to an SMA of `k` points):
```
α = 2/(k+1)            (equivalently k = (2−α)/α)
```
**Adaptive recalibration by sample size** — unbiased incremental mean over `N_t` seen samples:
```
α_t = 1/(1+N_t),   E_{t+1} = E_t + (A_t − E_t)/(N_t+1)
```
Asymptotic variance of the EMA:  **Var(E_∞) = σ²·α/(2−α)**.

## 2. Tail Latency — M/M/c + Erlang C + Gunther USL
M/M/c: Poisson arrivals `λ`, `c` exponential servers rate `μ`, **ρ = λ/(cμ) < 1**.
Offered load `a = cρ`. Erlang-C probability all servers busy (i.e. must wait):
```
C(c,a) =  [ a^c/(c!(1−ρ)) ]  /  [ Σ_{n=0}^{c−1} a^n/n!  +  a^c/(c!(1−ρ)) ]
```
Mean queue wait (Erlang C / Pollaczek–Khintchine):
```
W_q = C(c,a)·ρ/(cμ(1−ρ))  =  C(c,a)·ρ/(cμ − λ)
```
Tail wait distribution (unconditional):
```
P(W_q > t) = C(c,a)·e^{−cμ(1−ρ)t}  =  C(c,a)·e^{−(cμ−λ)t}
```
As `ρ→1`, decay rate `cμ(1−ρ)→0` ⇒ tail flattens ⇒ latency blowup.

**Gunther Universal Scalability Law** — speedup vs `N` (=c workers):
```
S(N) = N / ( 1 + σ(N−1) + κ·N(N−1) )
```
`σ∈[0,1]` serial/contention fraction (Amdahl); `κ≥0` coherency/crosstalk fraction.
`σ`-term ⇒ sublinear; `κ`-term ⇒ retrograde (throughput peaks then falls). Link to tail:
fanning out to more workers raises `N`; once `κ` drives `S(N)` down, effective
capacity collapses, `ρ` climbs for fixed `λ`, and Erlang-C tail explodes.

## 3. SPC / EWMA Drift Detection
EWMA statistic (in-control target `μ0`):  `z_t = λ·x_t + (1−λ)·z_{t−1}`, `z_0 = μ0`.
Steady-state std `σ_z = σ·√(λ/(2−λ))`. **Control limits:**
```
UCL / LCL = μ0 ± L·σ·√(λ/(2−λ))        (L = 3 typical)
```
Drift = limit exit, or **Western Electric (WECO) rules**:
- WE1: 1 point beyond 3σ (outside limits)
- WE2: 2 of 3 consecutive > 2σ, same side
- WE3: 4 of 5 consecutive > 1σ, same side
- WE4: 8 consecutive same side of center (run)
- WE5: 6 trending (monotonic) points
- WE6: 15 consecutive within 1σ (over-control / reduced variation)

### Sources (canonical; live-verify flagged)
- EMA / SMA-equiv / recursive mean: R.G. Brown, *Smoothing...Forecasting* (1963); NIST/SEMATECH e-Handbook §6.4.3. [unverified]
- M/M/c, Erlang C: A.K. Erlang (1917), *POEEJ* 10:189–197; L. Kleinrock, *Queueing Systems* Vol.1 (1975). [unverified]
- USL: N.J. Gunther, *Guerrilla Capacity Planning* (Springer, 2007); perfdynamics.com. [unverified]
- EWMA chart: S.W. Roberts (1959), *Technometrics* 1(2):239–250; J.S. Hunter (1986), *JQT* 18(4):203–210; G.E.P. Box (1951). (EWMA chart proper = Roberts/Hunter; Box ≈ CUSUM/time-series.) [unverified]
- WECO rules: Western Electric (1956), *Statistical QC Handbook*; D.C. Montgomery, *Introduction to SPC* (§6). [unverified]

*NOTE: All equation forms above are the standard, well-established definitions. URL/DOI
live-verification could not be performed (web tools offline); verify links before publication.*
