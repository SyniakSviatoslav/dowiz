# eval-layer/metrics.py
# The fuzzy metrics the deterministic core (Phase B) can't measure. All advisory.
#
# ⚠ DeepEval metric/param names shift across versions — confirm imports against your installed version.

from deepeval.metrics import GEval, ToolCorrectnessMetric, TaskCompletionMetric
from deepeval.test_case import LLMTestCaseParams
from openrouter_judge import OpenRouterJudge

judge = OpenRouterJudge()  # model-agnostic, via OpenRouter

# Tool calls vs expected — deterministic-ish, needs no judge model (cheap, low-noise).
tool_correctness = ToolCorrectnessMetric()

# Did the agent actually accomplish the task (outcome over the whole trajectory)?
task_completion = TaskCompletionMetric(threshold=0.7, model=judge)

# Was the PATH logical/efficient — not just the outcome?
trajectory_logic = GEval(
    name="Trajectory logic",
    criteria=(
        "The steps the agent took are logical, non-redundant, and efficient toward the stated task — "
        "no needless tool calls, detours, or backtracking."
    ),
    evaluation_params=[
        LLMTestCaseParams.INPUT,
        LLMTestCaseParams.TOOLS_CALLED,
        LLMTestCaseParams.ACTUAL_OUTPUT,
    ],
    threshold=0.7,
    model=judge,
)

# Output quality / convention adherence.
output_quality = GEval(
    name="Output quality",
    criteria=(
        "The output is correct, complete, and consistent with DeliveryOS conventions and the user's "
        "intent. It does not contradict the project's invariants."
    ),
    evaluation_params=[LLMTestCaseParams.INPUT, LLMTestCaseParams.ACTUAL_OUTPUT],
    threshold=0.7,
    model=judge,
)

ALL_METRICS = [tool_correctness, task_completion, trajectory_logic, output_quality]
