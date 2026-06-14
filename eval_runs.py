# eval-layer/eval_runs.py
# Loads recorded agent runs, scores them with the advisory metrics, writes per-run results keyed by
# run_id. ADVISORY ONLY: uses evaluate() (does not raise/fail the build). The metric core (Phase B)
# is the gate — these scores feed analytics, never block.
#
# Usage:  python eval_runs.py runs.json
#   runs.json: list of recorded runs (see runs.example.json). Dump it from your runtime per run,
#   or pull from Langfuse traces. Keep `run_id` so scores join with the Phase B core score.

import json
import sys
from deepeval import evaluate
from deepeval.test_case import LLMTestCase, ToolCall
from metrics import ALL_METRICS


def to_tool_calls(items):
    return [ToolCall(name=t["name"], input_parameters=t.get("args", {})) for t in (items or [])]


def load_runs(path):
    runs = json.load(open(path))
    cases = []
    for r in runs:
        cases.append(
            LLMTestCase(
                input=r["input"],
                actual_output=r["actual_output"],
                expected_output=r.get("expected_output"),
                tools_called=to_tool_calls(r.get("tools_called")),
                expected_tools=to_tool_calls(r.get("expected_tools")),
                additional_metadata={"run_id": r["run_id"]},
            )
        )
    return runs, cases


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "runs.json"
    runs, cases = load_runs(path)

    results = evaluate(test_cases=cases, metrics=ALL_METRICS)  # advisory: returns, does not raise

    # ⚠ evaluate() return shape varies by DeepEval version — confirm `.test_results` / `.metrics_data`.
    test_results = getattr(results, "test_results", results)
    out = []
    for run, res in zip(runs, test_results):
        out.append(
            {
                "run_id": run["run_id"],
                "metrics": [
                    {
                        "name": m.name,
                        "score": getattr(m, "score", None),
                        "passed": getattr(m, "success", getattr(m, "passed", None)),
                        "reason": getattr(m, "reason", None),
                    }
                    for m in res.metrics_data
                ],
            }
        )

    json.dump(out, open("deepeval-result.json", "w"), indent=2)
    print(json.dumps(out, indent=2))

    # OPTIONAL (Phase D native dashboards): push each metric as a Langfuse score keyed by run_id.
    # Confirm the Langfuse Python SDK call + how you key (e.g. trace/session by run_id) for your version:
    #
    # from langfuse import Langfuse
    # lf = Langfuse()
    # for row in out:
    #     for m in row["metrics"]:
    #         lf.create_score(name=f"eval.{m['name']}", value=m["score"] or 0,
    #                         trace_id=resolve_trace_id(row["run_id"]))
    # lf.flush()


if __name__ == "__main__":
    main()
