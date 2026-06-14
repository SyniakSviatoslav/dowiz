"""
metrics.py — DeepEval 4.x metric factories for DeliveryOS eval-layer.

Bundles three evaluation dimensions:
  1. ToolCorrectness — did the agent call the right tools?
  2. TaskCompletion — did it finish the task?
  3. GEval(trajectory) — was the approach logical & efficient?
  4. GEval(quality) — was the output well-structured & correct?

Each factory returns a (metric, list_of_case_fields_needed) tuple so
eval_runs.py can validate required fields before calling evaluate().
"""

from deepeval.metrics import GEval, ToolCorrectnessMetric, TaskCompletionMetric
from deepeval.test_case import LLMTestCase, ToolCall


def build_tool_correctness(
    model: str,
    threshold: float = 0.5,
) -> tuple[ToolCorrectnessMetric, list[str]]:
    """Requires case.expected_tools + case.input."""
    metric = ToolCorrectnessMetric(
        model=model,
        threshold=threshold,
        include_reason=True,
        should_exact_match=False,
        should_consider_ordering=False,
    )
    return metric, ["input", "expected_tools"]


def build_task_completion(
    model: str,
    task: str | None = None,
    threshold: float = 0.5,
) -> tuple[TaskCompletionMetric, list[str]]:
    """Requires case.input + case.actual_output."""
    metric = TaskCompletionMetric(
        model=model,
        task=task,
        threshold=threshold,
        include_reason=True,
    )
    return metric, ["input", "actual_output"]


def build_trajectory_g_eval(
    model: str,
    threshold: float = 0.5,
) -> tuple[GEval, list[str]]:
    """GEval that scores trajectory logic & efficiency.

    Requires case.input + case.actual_output.
    """
    metric = GEval(
        name="Trajectory logic",
        model=model,
        threshold=threshold,
        evaluation_params=["input", "actual_output", "tools_called"],
        criteria=(
            "Assess whether the agent's approach is logical, efficient, "
            "and follows a coherent chain of reasoning. Penalise: "
            "unnecessary steps, redundant tool calls, backtracking, "
            "hallucinated capabilities, or solving a different problem "
            "than the one asked."
        ),
        evaluation_steps=[
            "Does the agent understand the task correctly?",
            "Does the chain of tool calls follow a logical order?",
            "Are there unnecessary, redundant, or hallucinated steps?",
            "Could the task be completed in fewer steps without losing quality?",
            "Does the agent reason correctly about tool results before the next action?",
        ],
    )
    return metric, ["input", "actual_output", "tools_called"]


def build_quality_g_eval(
    model: str,
    threshold: float = 0.5,
) -> tuple[GEval, list[str]]:
    """GEval that scores output quality / correctness.

    Requires case.input + case.actual_output + case.expected_output.
    Falls back gracefully if expected_output is missing.
    """
    metric = GEval(
        name="Output quality",
        model=model,
        threshold=threshold,
        evaluation_params=["input", "actual_output", "expected_output"],
        criteria=(
            "Assess whether the output is well-structured, correct, "
            "complete, and follows the requested format. Penalise: "
            "missing sections, incorrect data, formatting mistakes, "
            "partial responses, or outputs that don't address the input."
        ),
        evaluation_steps=[
            "Does the output address every requirement in the input?",
            "Is the output well-structured and easy to read?",
            "Are there factual or logical errors in the output?",
            "Does the output follow the requested format?",
            "Would this output be useful as-is?",
        ],
    )
    return metric, ["input", "actual_output", "expected_output"]


def default_metrics(model: str) -> list[tuple]:
    """Build all four metrics with sensible defaults.

    Returns list of (metric, required_fields).
    """
    return [
        build_task_completion(model),
        build_trajectory_g_eval(model),
        build_quality_g_eval(model),
    ]
