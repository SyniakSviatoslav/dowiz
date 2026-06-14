# eval-layer/openrouter_judge.py
# Model-agnostic DeepEval judge: any model behind OpenRouter (OpenAI-compatible).
# Best practice: pick a DIFFERENT model than the one being evaluated, to reduce self-preference bias.
#
# ⚠ DeepEval's custom-model contract (especially structured-output / `schema` handling) varies by
#   version. Some versions call generate(prompt, schema) expecting a parsed pydantic instance, others
#   a JSON string, others support native schema. If GEval errors on parsing, check DeepEval's
#   "custom model" docs for your installed version. Simpler alternative: recent DeepEval can target an
#   OpenAI-compatible base URL via its own model config — confirm in the docs and you may skip this class.

import os
from openai import OpenAI
from deepeval.models import DeepEvalBaseLLM


class OpenRouterJudge(DeepEvalBaseLLM):
    def __init__(self, model: str | None = None):
        self.model = model or os.environ.get("DEEPEVAL_JUDGE_MODEL", "openai/gpt-5.1")
        self.client = OpenAI(
            base_url="https://openrouter.ai/api/v1",
            api_key=os.environ["OPENROUTER_API_KEY"],
        )

    def load_model(self):
        return self.client

    def get_model_name(self) -> str:
        return f"openrouter:{self.model}"

    def generate(self, prompt: str, schema=None):
        kwargs = {"model": self.model, "messages": [{"role": "user", "content": prompt}]}
        if schema is not None:
            # Structured metrics (GEval) request JSON. Newer DeepEval expects a parsed schema instance.
            kwargs["response_format"] = {"type": "json_object"}
        content = self.client.chat.completions.create(**kwargs).choices[0].message.content
        if schema is not None:
            return schema.model_validate_json(content)
        return content

    async def a_generate(self, prompt: str, schema=None):
        # Sync fallback is fine for a CI starter; make it truly async later if throughput matters.
        return self.generate(prompt, schema)
