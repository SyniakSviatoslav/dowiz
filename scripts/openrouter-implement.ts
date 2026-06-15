#!/usr/bin/env node
/**
 * OpenRouter implementation bridge.
 *
 * Reads a complete task spec from stdin (or --task flag), sends it to a free
 * OpenRouter model, and prints the raw completion to stdout.
 *
 * Usage:
 *   echo "$SPEC" | npx tsx scripts/openrouter-implement.ts
 *   npx tsx scripts/openrouter-implement.ts --task "..."
 *
 * Env:
 *   ***REDACTED***       (required)
 *   OPENROUTER_MODEL         (optional, overrides primary model)
 *   OPENROUTER_MODEL_FALLBACKS (optional, comma-separated list, overrides built-in fallbacks)
 *   OPENROUTER_MAX_TOKENS    (optional, default: 8192)
 */

const API_KEY = process.env.***REDACTED***;
const MAX_TOKENS = Number(process.env.OPENROUTER_MAX_TOKENS ?? 8192);

// Free models in priority order — tried sequentially on rate-limit / error
const DEFAULT_MODELS = [
  'nvidia/nemotron-3-super-120b-a12b:free',
  'qwen/qwen-2.5-coder-32b-instruct:free',
  'deepseek/deepseek-r1-0528:free',
  'google/gemma-3-27b-it:free',
  'mistralai/mistral-small-3.2-24b-instruct:free',
];

function buildModelChain(): string[] {
  if (process.env.OPENROUTER_MODEL) {
    return [process.env.OPENROUTER_MODEL];
  }
  if (process.env.OPENROUTER_MODEL_FALLBACKS) {
    return process.env.OPENROUTER_MODEL_FALLBACKS.split(',').map(s => s.trim()).filter(Boolean);
  }
  return DEFAULT_MODELS;
}

if (!API_KEY) {
  process.stderr.write('[openrouter-implement] ***REDACTED*** is not set\n');
  process.exit(1);
}

async function readStdin(): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) chunks.push(chunk as Buffer);
  return Buffer.concat(chunks).toString('utf-8').trim();
}

async function callModel(model: string, prompt: string): Promise<string | null> {
  const res = await fetch('https://openrouter.ai/api/v1/chat/completions', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${API_KEY}`,
      'Content-Type': 'application/json',
      'HTTP-Referer': 'https://dowiz.fly.dev',
      'X-Title': 'DeliveryOS Orchestrator',
    },
    body: JSON.stringify({
      model,
      messages: [
        {
          role: 'system',
          content:
            'You are a precise code implementation assistant. ' +
            'Output ONLY the complete modified file content — no markdown fences, ' +
            'no explanations, no commentary. Raw code only.',
        },
        { role: 'user', content: prompt },
      ],
      temperature: 0.05,
      max_tokens: MAX_TOKENS,
    }),
  });

  if (res.status === 429 || res.status === 503 || res.status === 502) {
    process.stderr.write(`[openrouter-implement] Model ${model} returned ${res.status} — trying next\n`);
    return null;
  }

  if (!res.ok) {
    const body = await res.text();
    process.stderr.write(`[openrouter-implement] Model ${model} error ${res.status}: ${body}\n`);
    return null;
  }

  const data = (await res.json()) as {
    choices: Array<{ message: { content: string } }>;
    error?: { message: string };
  };

  if (data.error) {
    process.stderr.write(`[openrouter-implement] Model ${model} error: ${data.error.message}\n`);
    return null;
  }

  const content = data.choices?.[0]?.message?.content ?? '';
  if (!content) {
    process.stderr.write(`[openrouter-implement] Model ${model} returned empty response\n`);
    return null;
  }

  return content;
}

async function main() {
  const taskIdx = process.argv.indexOf('--task');
  const prompt =
    taskIdx !== -1 && process.argv[taskIdx + 1]
      ? process.argv[taskIdx + 1]
      : await readStdin();

  if (!prompt) {
    process.stderr.write('[openrouter-implement] No task provided\n');
    process.exit(1);
  }

  const models = buildModelChain();
  const tried: string[] = [];
  const MAX_ATTEMPTS = 3;
  const RETRY_DELAY_MS = 1000;

  for (const model of models) {
    tried.push(model);
    for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
      process.stderr.write(`[openrouter-implement] Trying model: ${model} (attempt ${attempt}/${MAX_ATTEMPTS})\n`);
      const result = await callModel(model, prompt);
      if (result !== null) {
        process.stdout.write(result + '\n');
        return;
      }
      if (attempt < MAX_ATTEMPTS) {
        await new Promise(resolve => setTimeout(resolve, RETRY_DELAY_MS));
      }
    }
  }

  process.stderr.write(
    `[openrouter-implement] All models failed after ${MAX_ATTEMPTS} attempts each.\n` +
    `Models tried: ${tried.join(', ')}\n`
  );
  process.exit(1);
}

main().catch((err) => {
  process.stderr.write(`[openrouter-implement] Fatal: ${err}\n`);
  process.exit(1);
});
