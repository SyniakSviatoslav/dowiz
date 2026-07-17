# Security Policy

{{BRAND}} (dowiz / DeliveryOS) is sovereign delivery infrastructure built on a
deterministic Rust/WASM kernel and a post-quantum mesh protocol. Its safety model
is **fail-closed by design**, not by policy:

- **Kernel red-line gate** denies unsafe capability (money, auth, RLS/migrations)
  by *default*. A capability request that is not explicitly authorized on the
  allow-list is rejected — denial is the safe path, not an exception.
- **Pure core** (`kernel`, `engine`, mesh protocol, crypto) does no network / clock
  / RNG inside the decision path, so behavior is reproducible and auditable.
- **No cloud keys in files** — the hub reads configuration from the environment only.

## Reporting a vulnerability

Please report security issues **privately**:

- **Primary channel:** GitHub Private Vulnerability Reporting
  (Settings → Security → Advisories → *Report a vulnerability*), enabled at flip.
- **Fallback:** email the maintainer via the GitHub profile.

Do **not** open a public issue for a live exploit. Red-line / kernel-gate bypass
findings get priority and a fast, public fix.

## Safety model

{{BRAND}} is fail-closed by design: the kernel red-line gate (money / auth / RLS /
migrations) denies unsafe capability by default, not by policy. Red-line or
kernel-gate bypass findings get priority and a fast public fix.

## Scope

Out of scope for this repo's threat model:

- the LLM backend a hub wires in (its governance is the hub's), and
- the host-OS permissions granted to the process.

The kernel gates *what the agent is allowed to attempt*, not what the OS allows
the process overall — run it with least privilege.

## Supported versions

| Branch / tag        | Supported? |
|---------------------|------------|
| `main`              | Yes        |
| pre-flip tags       | No         |
