// Telemetry collectors — the data SOURCES the foundation deferred. These turn
// the §2 record's zeros into real numbers:
//   - collectGitMem: git branch/commits + process RSS (VmHWM).
//   - collectSessionTelemetry: parse the Claude Code session JSONL (the same
//     data codeburn reads) over the run's [t_start, t_end] window → tokens by
//     model, cost, skills, agents.
// Pure-ish: filesystem + git only, no network.

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import type { TokensBlock } from './types.js';

// Claude API pricing, $ per 1M tokens (input / output / cache-read). Estimates —
// update alongside model changes. Used only for the cost figure + trend.
const PRICING: Record<string, { in: number; out: number; cache: number }> = {
  'claude-opus-4-8': { in: 15, out: 75, cache: 1.5 },
  'claude-opus': { in: 15, out: 75, cache: 1.5 },
  'claude-sonnet-4-6': { in: 3, out: 15, cache: 0.3 },
  'claude-sonnet': { in: 3, out: 15, cache: 0.3 },
  'claude-haiku-4-5': { in: 0.8, out: 4, cache: 0.08 },
  'claude-haiku': { in: 0.8, out: 4, cache: 0.08 },
};
const DEFAULT_PRICE = { in: 5, out: 15, cache: 0.5 };
function priceFor(model: string) {
  return PRICING[model] ?? PRICING[Object.keys(PRICING).find((k) => model.startsWith(k)) ?? ''] ?? DEFAULT_PRICE;
}

export interface GitMem {
  branch: string;
  commits: number;
  rss_peak_mb: number;
}

/** git branch + commit count (since an optional ref) + this process's peak RSS. */
export function collectGitMem(repoDir: string, sinceRef?: string): GitMem {
  const git = (args: string[]): string => {
    try { return execFileSync('git', ['-C', repoDir, ...args], { encoding: 'utf8' }).trim(); }
    catch { return ''; }
  };
  const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']) || 'unknown';
  const commits = sinceRef ? Number(git(['rev-list', '--count', `${sinceRef}..HEAD`]) || 0) : 0;
  let rss_peak_mb = 0;
  try {
    const status = fs.readFileSync('/proc/self/status', 'utf8');
    const m = status.match(/VmHWM:\s+(\d+)\s+kB/);
    if (m) rss_peak_mb = Math.round(Number(m[1]) / 1024);
  } catch { /* non-linux */ }
  return { branch, commits, rss_peak_mb };
}

export interface SessionTelemetry {
  tokens: TokensBlock;
  tokensByModel: Record<string, number>; // COMPUTE tokens per model (in+out, for eco)
  skills_used: Record<string, number>;
  agents: Record<string, number>;
}

function skillNameFromTool(name: string, input: any): string | null {
  if (name === 'Skill') return input?.skill ? `skill:${input.skill}` : 'skill';
  const mcp = name.match(/^mcp__([a-z0-9-]+(?:_[a-z0-9-]+)*?)__/i);
  if (mcp) return mcp[1]!.replace(/_/g, '-'); // group by MCP server
  return null;
}

function emptyTelemetry(): SessionTelemetry {
  return {
    tokens: { in: 0, out: 0, cache_read: 0, cache_write: 0, by_model: {}, cost_usd: 0 },
    tokensByModel: {}, skills_used: {}, agents: {},
  };
}

// Accumulate one assistant message's usage + tool-use into the running totals.
// Shared by the session-JSONL and workflow-transcript collectors so they stay identical.
function accumulateMessage(out: SessionTelemetry, model: string, usage: any, content: any[] | undefined): void {
  const inTok = usage.input_tokens ?? 0;
  const outTok = usage.output_tokens ?? 0;
  const cacheRead = usage.cache_read_input_tokens ?? 0;
  const cacheWrite = usage.cache_creation_input_tokens ?? 0;
  out.tokens.in! += inTok;
  out.tokens.out! += outTok;
  out.tokens.cache_read! += cacheRead;
  out.tokens.cache_write! += cacheWrite;
  out.tokens.by_model![model] = (out.tokens.by_model![model] ?? 0) + inTok + outTok + cacheRead + cacheWrite;
  // Eco is driven by COMPUTE tokens (input+output) only — cache-read is NOT re-processed
  // (that's the point of caching); counting it inflated a 20-min run to 12.6 kWh.
  out.tokensByModel[model] = (out.tokensByModel[model] ?? 0) + inTok + outTok;
  const p = priceFor(model);
  // cache-write is billed ~1.25× input; fold into cost via the input rate × 1.25.
  out.tokens.cost_usd! += (inTok / 1e6) * p.in + (outTok / 1e6) * p.out + (cacheRead / 1e6) * p.cache + (cacheWrite / 1e6) * p.in * 1.25;
  for (const block of content ?? []) {
    if (block?.type !== 'tool_use') continue;
    if (block.name === 'Agent') {
      const kind = block.input?.subagent_type ?? 'agent';
      out.agents[kind] = (out.agents[kind] ?? 0) + 1;
    }
    const skill = skillNameFromTool(block.name, block.input);
    if (skill) out.skills_used[skill] = (out.skills_used[skill] ?? 0) + 1;
  }
}

function finalizeTotals(out: SessionTelemetry): SessionTelemetry {
  out.tokens.cost_usd = Math.round(out.tokens.cost_usd! * 100) / 100;
  out.tokens.read_edit_ratio = Math.round((out.tokens.cache_read! / Math.max(1, out.tokens.in!)) * 10) / 10;
  return out;
}

/**
 * Aggregate a workflow's subagent transcripts (subagents/workflows/<runId>/agent-*.jsonl)
 * into the SAME SessionTelemetry shape. Background-Workflow loops run their agents in
 * separate transcripts that the main session JSONL never sees — without this, a
 * workflow loop's §5 TELEMETRY block is all zeros (the bug this fixes). Pass a single
 * run dir, or the parent `subagents/workflows` to roll up every run under it.
 */
export function collectWorkflowTelemetry(transcriptDir: string): SessionTelemetry {
  const out = emptyTelemetry();
  if (!fs.existsSync(transcriptDir)) return out;
  const files: string[] = [];
  const walk = (dir: string): void => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.name.endsWith('.jsonl')) files.push(p);
    }
  };
  walk(transcriptDir);
  for (const f of files) {
    for (const line of fs.readFileSync(f, 'utf8').split('\n')) {
      if (!line.includes('"usage"')) continue;
      let rec: any;
      try { rec = JSON.parse(line); } catch { continue; }
      const msg = rec.message ?? rec;
      if (!msg?.usage) continue;
      accumulateMessage(out, msg.model ?? 'unknown', msg.usage, msg.content);
    }
  }
  return finalizeTotals(out);
}

/** Sum any number of telemetry blocks (session + one-or-more workflow runs). */
export function mergeTelemetry(...parts: (SessionTelemetry | null | undefined)[]): SessionTelemetry {
  const out = emptyTelemetry();
  for (const p of parts) {
    if (!p) continue;
    out.tokens.in! += p.tokens.in ?? 0;
    out.tokens.out! += p.tokens.out ?? 0;
    out.tokens.cache_read! += p.tokens.cache_read ?? 0;
    out.tokens.cache_write! += p.tokens.cache_write ?? 0;
    out.tokens.cost_usd! += p.tokens.cost_usd ?? 0;
    for (const [m, n] of Object.entries(p.tokens.by_model ?? {})) out.tokens.by_model![m] = (out.tokens.by_model![m] ?? 0) + n;
    for (const [m, n] of Object.entries(p.tokensByModel)) out.tokensByModel[m] = (out.tokensByModel[m] ?? 0) + n;
    for (const [k, n] of Object.entries(p.skills_used)) out.skills_used[k] = (out.skills_used[k] ?? 0) + n;
    for (const [k, n] of Object.entries(p.agents)) out.agents[k] = (out.agents[k] ?? 0) + n;
  }
  return finalizeTotals(out);
}

/**
 * Parse the Claude Code session JSONL over [tStartIso, tEndIso] (inclusive) and
 * aggregate assistant-turn token usage by model + cost, plus tool/skill/agent
 * use. This is the source codeburn reads; here it's scoped to one loop run.
 */
export function collectSessionTelemetry(sessionFile: string, tStartIso: string, tEndIso: string): SessionTelemetry {
  const out = emptyTelemetry();
  if (!fs.existsSync(sessionFile)) return out;
  const t0 = Date.parse(tStartIso);
  const t1 = Date.parse(tEndIso);

  for (const line of fs.readFileSync(sessionFile, 'utf8').split('\n')) {
    if (!line) continue;
    let rec: any;
    try { rec = JSON.parse(line); } catch { continue; }
    const ts = Date.parse(rec.timestamp ?? '');
    if (!(ts >= t0 && ts <= t1)) continue;
    if (rec.type !== 'assistant' || !rec.message) continue;
    accumulateMessage(out, rec.message.model ?? 'unknown', rec.message.usage ?? {}, rec.message.content);
  }
  return finalizeTotals(out);
}
