#!/usr/bin/env python3
"""G2 — DeepEval scoring of the menu-parser outputs (tooling-integration-eval, CI-only).

Imported AFTER preflight.py passes (run.sh enforces order). Judge = Anthropic Claude (the only
allowlisted egress). Scores each generated output with:
  - FaithfulnessMetric  : are the extracted items grounded in the OCR source (no hallucinated dishes)?
  - GEval "price-grounding": does every product price appear in the OCR text (no invented prices)?

Infra-vs-quality discriminator (Breaker L2): a transport/provider error (no metric computed) →
status NEEDS-RERUN, exit 0 (advisory, sticky for human re-run); a COMPUTED metric below threshold →
exit 1 (blocking). Telemetry/cloud/OTel are already neutralized by preflight + env.
"""
import json
import os
import sys
from pathlib import Path

HERE = Path(__file__).parent
OUT = HERE / "outputs"
THRESHOLD = float(os.environ.get("EVAL_THRESHOLD", "0.7"))
MODEL = os.environ.get("DEEPEVAL_JUDGE_MODEL", "claude-sonnet-4-6")

try:
    from deepeval import evaluate
    from deepeval.metrics import FaithfulnessMetric, GEval
    from deepeval.test_case import LLMTestCase, LLMTestCaseParams
except Exception as e:  # import/provider failure = infra, not quality
    print(f"NEEDS-RERUN: deepeval import failed (infra): {e}", file=sys.stderr)
    sys.exit(0)

cases = []
for f in sorted(OUT.glob("*.json")):
    d = json.loads(f.read_text())
    products = d["output"].get("products", [])
    actual = "; ".join(f'{p.get("name")} = {p.get("price")}' for p in products)
    cases.append(LLMTestCase(
        input=d["ocr"],
        actual_output=actual or "(no products extracted)",
        retrieval_context=[d["ocr"]],
    ))

if not cases:
    print("NEEDS-RERUN: no generated outputs found (run generate-outputs first).", file=sys.stderr)
    sys.exit(0)

price_grounding = GEval(
    name="price-grounding",
    criteria="Every product price in the output must appear verbatim in the OCR source text. "
             "Penalize any price not present in the source (a hallucinated price).",
    evaluation_params=[LLMTestCaseParams.INPUT, LLMTestCaseParams.ACTUAL_OUTPUT],
    model=MODEL,
    threshold=THRESHOLD,
)
faithfulness = FaithfulnessMetric(threshold=THRESHOLD, model=MODEL)

try:
    result = evaluate(test_cases=cases, metrics=[faithfulness, price_grounding])
except Exception as e:  # judge 5xx / timeout / provider down = infra
    print(f"NEEDS-RERUN: judge call failed (infra): {e}", file=sys.stderr)
    sys.exit(0)

failed = []
for tr in getattr(result, "test_results", []):
    for m in getattr(tr, "metrics_data", []) or []:
        if m.success is False:  # computed AND below threshold = quality failure
            failed.append(f"{tr.name}: {m.name}={m.score:.2f} < {THRESHOLD}")

if failed:
    print(f"✗ G2 eval QUALITY FAILURE ({len(failed)}):", file=sys.stderr)
    for x in failed:
        print("  - " + x, file=sys.stderr)
    sys.exit(1)
print(f"✓ G2 eval: {len(cases)} fixtures pass faithfulness + price-grounding (≥{THRESHOLD}).")
