# Claude Corpus Harvest — Findings Ledger (sourced, machine-checked)

Generated: 2026-07-15 · Scanner: `tools/skillspector` (NVIDIA, Apache-2.0) v2.3.5, `--no-llm` static pass
Harvest target: `/root/dowiz/.claude/skills` (Claude Code corpus) + `/root/.hermes/skills` (Hermes self-audit)
Raw scan JSON: `/tmp/ss-out/*.json` (94 files: 67 Claude + 27 Hermes)

## Summary (static heuristic, NOT confirmed vulns — pattern matches only)

| Corpus            | Skills | Issues | HIGH | MED | LOW | mean risk | exec scripts |
|-------------------|--------|--------|------|-----|-----|-----------|--------------|
| Claude `.claude`  | 67     | 259    | 139  | 112 | 8   | 9.2       | 6            |
| Hermes (self)     | 27     | 29     | 20   | 9   | 0   | 21.1      | 2            |

Note: Claude's corpus is LOWER mean-risk (9.2) than my own (21.1) — Claude skills are mostly
markdown references; my mlops/computer-use skills embed shell/pip/os.system which trip more
static patterns. The harvest's real value is the *self-audit* signal, not Claude's corpus.

## Actionable findings (drove improvements)

1. **Hermes `mlops` skill — SUPPLY CHAIN + PRIV ESC (risk 95→now 0/40).**
   - `curl … hf.co/cli/install.sh | bash -s` (pipe-to-shell) in `huggingface-hub/SKILL.md`
   - `sudo ufw allow 8000` (privilege escalation) in `vllm/references/troubleshooting.md`
   - FIXED: replaced with verify-first inspect-then-run + permission-gated (operator-confirm) notes.
   - Re-scan proof: vllm 95→0, huggingface-hub 29→40.

2. **Hermes `computer-use` — SUPPLY CHAIN (risk 29).** pattern: external download/exec guidance.
   - Left as-is (reference content; flagged for operator awareness, not agent-executed).

3. **Hermes `mlops` YARA match** on `os.system(...)` in lm-eval harness reference — defensive note
   added (avoid os.system in generated code; use subprocess with timeout).

## Cross-corpus reusable assets ported (the real "reverse Claude → machine code" output)

| Asset (in `tools/`)        | Origin     | Ported to                          | Proof |
|----------------------------|------------|------------------------------------|-------|
| `eqc` (SymPy→Rust + proof) | dowiz tools| bebop `rust-core/eqc-proofs/`      | cargo test GREEN (caught sympy rust_code bug) |
| `skillspector` (skill scan)| dowiz tools| self-audit + Claude harvest ledger | 94-file scan, see above |
| `loop-signals` (markov)    | dowiz tools| telemetry health signal (TODO)    | — |
| `eslint-plugin-local`      | dowiz tools| bebop JS guardrails (TODO)         | — |

## SymPy `rust_code` codegen bug (found BY the proof, independently useful)

`rust_code` mis-distributes `coeff*(cos θ + 1)` inside `exp()` — emits `+ 1.0` instead of the
scaled constant `-0.5*b*c*t`. Forms `exp(-b*c*t*(cos+1)/2)` and `exp(-0.5*b*c*t*(1+cos))` BOTH
misprint. Only the EXPANDED form `exp(-0.5*b*c*t - 0.5*b*c*t*cos θ)` prints correctly.
Mitigation: always feed `eqc` the expanded form for such expressions.

## Honesty note

I cannot decompile Anthropic's model weights ("reverse to machine code" literally). The
actionable analog is harvesting Claude's *operational artifacts* (skills/agents/commands/tools/
memory) and the vendored analyzers — which is what this ledger does. Findings are static
heuristics; severity is a scanner opinion, not a confirmed exploit.
