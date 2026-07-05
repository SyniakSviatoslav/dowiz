#!/usr/bin/env python3
"""G2 pre-flight — fail-closed safety gate for the DeepEval menu-parser eval (tooling-integration-eval).

Pure stdlib. Runs BEFORE any fixture is loaded or `deepeval` is imported, and exits non-zero if ANY
precondition fails — so a misconfigured run never reaches the network with data. CI is the enforceable
authority (the egress allowlist is a network-layer construct the raw `deepeval` CLI cannot escape);
this pre-flight is the in-process fail-closed complement.

Checks (Breaker M1 / H3 / RA-6, ADR-tooling-integration-eval G2):
  1. Telemetry OFF        — DEEPEVAL_TELEMETRY_OPT_OUT == "1" (+ error-reporting opt-out).
  2. Confident-AI cloud OFF — no CONFIDENT_AI_API_KEY / DEEPEVAL_API_KEY (no cloud login/sync).
  3. #2497 OTel hijack neutralized — OTEL_SDK_DISABLED == "true" and no off-allowlist OTLP endpoint.
  4. Egress allowlist present — EVAL_EGRESS_ALLOWLIST set and contains ONLY allowlisted hosts
     (the Anthropic judge endpoint); a denylist is fail-open and rejected.
  5. Synthetic-only fixtures — the fixtures dir carries NO structured PII (defense-in-depth; the
     authoring ritual is the load-bearing floor).
"""
import os
import re
import sys
from pathlib import Path

FIXTURES = Path(__file__).parent / "fixtures"
# The ONLY destination an eval run may reach (the Claude judge). Anything else → fail.
ALLOWED_HOSTS = {"api.anthropic.com"}

errors: list[str] = []


def need(cond: bool, msg: str) -> None:
    if not cond:
        errors.append(msg)


# 1. telemetry off
need(os.environ.get("DEEPEVAL_TELEMETRY_OPT_OUT") == "1",
     "DEEPEVAL_TELEMETRY_OPT_OUT must be '1' (telemetry is ON by default).")
need(os.environ.get("ERROR_REPORTING") in (None, "0", "false"),
     "ERROR_REPORTING must be off.")

# 2. Confident-AI cloud off (no login/sync)
for k in ("CONFIDENT_AI_API_KEY", "DEEPEVAL_API_KEY", "CONFIDENT_API_KEY"):
    need(not os.environ.get(k), f"{k} must be UNSET (no Confident-AI cloud login/sync).")

# 3. #2497 OTel global-TracerProvider hijack neutralized
need(os.environ.get("OTEL_SDK_DISABLED") == "true",
     "OTEL_SDK_DISABLED must be 'true' (neutralizes the #2497 TracerProvider hijack/exfiltration).")
otlp = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "")
if otlp:
    host = re.sub(r"^https?://", "", otlp).split("/")[0].split(":")[0]
    need(host in ALLOWED_HOSTS, f"OTEL_EXPORTER_OTLP_ENDPOINT points off-allowlist ({host}).")

# 4. egress allowlist present + allowlist-only (not a denylist)
allow = [h.strip() for h in os.environ.get("EVAL_EGRESS_ALLOWLIST", "").split(",") if h.strip()]
need(bool(allow), "EVAL_EGRESS_ALLOWLIST must be set (allowlist of reachable hosts; a denylist is fail-open).")
for h in allow:
    need(h in ALLOWED_HOSTS, f"EVAL_EGRESS_ALLOWLIST contains a non-allowlisted host: {h}")

# 5. synthetic-only fixtures (structured-PII scan; bare names held by the authoring ritual)
PII = [
    ("email", re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+")),
    ("url-query", re.compile(r"https?://[^\s]+\?[^\s]+")),
    ("iban", re.compile(r"[A-Z]{2}\d{2}[A-Z0-9]{10,30}")),
    ("card", re.compile(r"(?:\d[ -]*?){13,19}")),
    ("phone", re.compile(r"(?:\+|00)?(?:[0-9]{1,3})?[-\s()]*[0-9][-\s()0-9]{6,}[0-9]")),
]
if FIXTURES.exists():
    for f in FIXTURES.rglob("*.json"):
        txt = f.read_text(encoding="utf-8")
        for kind, rx in PII:
            m = rx.search(txt)
            if m:
                errors.append(f"FIXTURE-PII ({kind}): {f.name} contains '{m.group(0)[:30]}' — fixtures must be synthetic/PII-free.")

if errors:
    print(f"✗ G2 pre-flight FAILED ({len(errors)}) — refusing to run the eval (fail-closed):", file=sys.stderr)
    for e in errors:
        print("  - " + e, file=sys.stderr)
    sys.exit(2)
print("✓ G2 pre-flight: telemetry off, no Confident-AI cloud, #2497 neutralized, egress allowlist-only, fixtures PII-clean.")
