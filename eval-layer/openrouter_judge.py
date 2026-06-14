"""
openrouter_judge.py — Model-agnostic DeepEval judge via OpenRouter.

DeepEval v4 supports any OpenAI-compatible API via OPENAI_BASE_URL.
This module wraps OpenRouter as a DeepEvalBaseLLM for use with GEval
and other DeepEval metrics.

Usage:
    from openrouter_judge import OpenRouterJudge

    judge = OpenRouterJudge(model="openai/gpt-4o")
    # or via env:
    #   DEEPEVAL_JUDGE_MODEL="openai/gpt-4o"
    #   OPENROUTER_API_KEY="sk-or-..."
    #   OPENAI_BASE_URL="https://openrouter.ai/api/v1"
"""

import os
from typing import Optional

from openai import OpenAI


class OpenRouterJudge:
    """Thin wrapper around an OpenAI-compatible API (OpenRouter).

    Provides a .model string that DeepEval's native OpenAI integration
    can use when OPENAI_BASE_URL points at OpenRouter, and a
    DeepEvalBaseLLM subclass for programmatic use.
    """

    DEFAULT_BASE_URL = "https://openrouter.ai/api/v1"

    def __init__(self, model: Optional[str] = None):
        self.model_name = (
            model
            or os.environ.get("DEEPEVAL_JUDGE_MODEL")
            or "openai/gpt-4o"
        )
        self.api_key = os.environ.get("OPENROUTER_API_KEY") or os.environ.get("OPENAI_API_KEY")
        self.base_url = os.environ.get("OPENAI_BASE_URL") or self.DEFAULT_BASE_URL

        if not self.api_key:
            raise ValueError(
                "OpenRouter judge needs OPENROUTER_API_KEY (or OPENAI_API_KEY) set"
            )

        self._client = OpenAI(
            api_key=self.api_key,
            base_url=self.base_url,
        )

    @property
    def model(self) -> str:
        """Return model string for DeepEval's native OpenAI integration."""
        return self.model_name

    def generate(self, prompt: str, **kwargs) -> str:
        """Synchronous generate — used by DeepEval judges."""
        temperature = kwargs.pop("temperature", 0.0)
        resp = self._client.chat.completions.create(
            model=self.model_name,
            messages=[{"role": "user", "content": prompt}],
            temperature=temperature,
            **kwargs,
        )
        return resp.choices[0].message.content or ""

    def generate_with_schema(self, prompt: str, schema, **kwargs) -> str:
        """Generate with JSON schema — used by structured-output metrics."""
        temperature = kwargs.pop("temperature", 0.0)
        resp = self._client.chat.completions.create(
            model=self.model_name,
            messages=[{"role": "user", "content": prompt}],
            temperature=temperature,
            response_format={"type": "json_object", "schema": schema},
            **kwargs,
        )
        return resp.choices[0].message.content or ""
