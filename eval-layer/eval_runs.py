"""
eval_runs.py — Load agent-run records → evaluate via DeepEval → write deepeval-result.json.

Usage:
    export ***REDACTED***=sk-or-...
    export DEEPEVAL_JUDGE_MODEL=openai/gpt-4o
    python eval_runs.py runs.json

Input runs.json shape — see runs.example.json.
Output: deepeval-result.json (per run, keyed by run_id).
"""

import json
import os
import sys

from deepeval import evaluate
from deepeval.test_case import LLMTestCase, ToolCall

from metrics import default_metrics
from openrouter_judge import OpenRouterJudge


def load_runs(path: str) -> list[dict]:
    with open(path, "r") as f:
        data = json.load(f)
    if isinstance(data, list):
        return data
    if isinstance(data, dict) and "runs" in data:
        return data["runs"]
    raise ValueError("runs.json must be a list or { runs: [...] }")


def to_tool_calls(raw: list[dict] | None) -> list[ToolCall]:
    """Convert raw tool-call records to DeepEval ToolCall objects."""
    if not raw:
        return []
    calls = []
    for tc in raw:
        name = tc.get("name") or tc.get("tool") or "unknown"
        inp = tc.get("input") or tc.get("arguments") or {}
        out = tc.get("output") or tc.get("result") or ""
        calls.append(ToolCall(name=name, input=str(inp), output=str(out)))
    return calls


def build_cases(
    runs: list[dict],
    required_fields: list[str],
) -> list[LLMTestCase]:
    """Build LLMTestCases from run records, skipping those missing required fields."""
    cases = []
    for run in runs:
        missing = [f for f in required_fields if f not in run]
        if missing:
            print(f"  [warn] run_id={run.get('run_id','?')} missing fields: {missing} — skipping")
            continue
        cases.append(
            LLMTestCase(
                input=run.get("input", ""),
                actual_output=run.get("actual_output", ""),
                expected_output=run.get("expected_output"),
                tools_called=to_tool_calls(run.get("tools_called")),
                # ToolCorrectness uses expected_tools, not tools_called
                expected_tools=to_tool_calls(run.get("expected_tools")),
                context=run.get("context") if isinstance(run.get("context"), list) else None,
                retrieval_context=run.get("retrieval_context") if isinstance(run.get("retrieval_context"), list) else None,
            )
        )
    return cases


def main():
    dry_run = "--dry-run" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("--")]

    if len(args) < 1:
        print("Usage: python eval_runs.py [--dry-run] runs.json")
        sys.exit(1)

    runs_path = args[0]
    runs = load_runs(runs_path)
    print(f"[eval-layer] loaded {len(runs)} run(s) from {runs_path}")

    if dry_run:
        print("[eval-layer] DRY RUN — skipping LLM judge, producing mock scores")
        output = []
        for run in runs:
            run_id = run.get("run_id", "?")
            output.append({
                "run_id": run_id,
                "metrics": [
                    {"name": "TaskCompletion", "score": 0.85, "passed": True, "reason": "(dry-run mock)"},
                    {"name": "Trajectory logic", "score": 0.72, "passed": True, "reason": "(dry-run mock)"},
                    {"name": "Output quality", "score": 0.90, "passed": True, "reason": "(dry-run mock)"},
                ],
            })
        with open("deepeval-result.json", "w") as f:
            json.dump(output, f, indent=2)
        print(f"[eval-layer] DRY RUN — results written to deepeval-result.json")
        print(json.dumps(output, indent=2))
        sys.exit(0)

    # Build judge
    judge = OpenRouterJudge()
    model = judge.model
    print(f"[eval-layer] judge model: {model}")

    # Build all metrics
    metric_defs = default_metrics(model)
    metrics = [md[0] for md in metric_defs]

    # Collect all required fields across metrics
    all_required = set()
    for _, fields in metric_defs:
        all_required.update(fields)

    cases = build_cases(runs, list(all_required))
    print(f"[eval-layer] built {len(cases)} test case(s) for evaluation")

    if not cases:
        print("[eval-layer] no test cases to evaluate — skipping")
        print(json.dumps({"eval_layer": [], "skipped": True}, indent=2))
        with open("deepeval-result.json", "w") as f:
            json.dump({"eval_layer": [], "skipped": True}, f, indent=2)
        sys.exit(0)

    # Run evaluation
    print(f"[eval-layer] evaluating {len(metrics)} metric(s) across {len(cases)} case(s) ...")
    result = evaluate(test_cases=cases, metrics=metrics)

    # Build per-run output — defensive iteration over result shape
    output = []
    if hasattr(result, "test_results"):
        for tr in result.test_results:
            run_idx = getattr(tr, "index", 0)
            run_id = runs[run_idx].get("run_id", f"run-{run_idx}") if run_idx < len(runs) else f"run-{run_idx}"
            metrics_out = []
            if hasattr(tr, "metrics_data"):
                for md in tr.metrics_data:
                    metrics_out.append({
                        "name": getattr(md, "name", "?"),
                        "score": getattr(md, "score", 0.0),
                        "passed": getattr(md, "is_successful", False),
                        "reason": getattr(md, "reason", ""),
                    })
            elif hasattr(tr, "metrics"):
                for md in tr.metrics:
                    metrics_out.append({
                        "name": getattr(md, "name", "?"),
                        "score": getattr(md, "score", 0.0),
                        "passed": getattr(md, "is_successful", False),
                        "reason": getattr(md, "reason", ""),
                    })
            else:
                # Last resort: iterate scored by metric name
                for metric in metrics:
                    try:
                        sc = metric.score
                        metrics_out.append({
                            "name": metric.__class__.__name__,
                            "score": sc,
                            "passed": sc >= (metric.threshold if hasattr(metric, 'threshold') else 0.5),
                            "reason": getattr(metric, 'reason', ''),
                        })
                    except Exception:
                        pass

            output.append({"run_id": run_id, "metrics": metrics_out})
    elif hasattr(result, "metrics_data"):
        # Flat metrics_data when len(cases)==1
        run_id = runs[0].get("run_id", "run-0") if runs else "run-0"
        metrics_out = []
        for md in result.metrics_data:
            metrics_out.append({
                "name": getattr(md, "name", "?"),
                "score": getattr(md, "score", 0.0),
                "passed": getattr(md, "is_successful", False),
                "reason": getattr(md, "reason", ""),
            })
        output.append({"run_id": run_id, "metrics": metrics_out})
    else:
        # Last-resort: try attribute-style access
        print(f"  [warn] unknown evaluate() result shape: {type(result).__name__}")
        print(f"  [warn] dir: {[x for x in dir(result) if not x.startswith('_')]}")
        output.append({"run_id": "unknown", "metrics": []})

    print(f"[eval-layer] evaluation complete — {sum(len(o['metrics']) for o in output)} metric values")

    with open("deepeval-result.json", "w") as f:
        json.dump(output, f, indent=2)

    print(f"[eval-layer] results written to deepeval-result.json")
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
