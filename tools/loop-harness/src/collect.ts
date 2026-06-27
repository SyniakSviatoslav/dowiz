// Telemetry collectors — the data SOURCES the foundation deferred. These turn
// the §2 record's zeros into real numbers:
//   - collectGitMem: git branch/commits + process RSS (VmHWM).
//   - collectSessionTelemetry: parse the Claude Code session JSONL (the same
//     data codeburn reads) over the run's [t_start, t_end] window → tokens by
//     model, cost, skills, agents.
// Pure-ish: filesystem + git only, no network.

import fs from 'node:fs';
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
  tokensByModel: Record<string, number>; // total tokens per model (for eco)
  skills_used: Record<string, number>;
  agents: Record<string, number>;
}

function skillNameFromTool(name: string, input: any): string | null {
  if (name === 'Skill') return input?.skill ? `skill:${input.skill}` : 'skill';
  const mcp = name.match(/^mcp__([a-z0-9-]+(?:_[a-z0-9-]+)*?)__/i);
  if (mcp) return mcp[1]!.replace(/_/g, '-'); // group by MCP server
  return null;
}

/**
 * Parse the Claude Code session JSONL over [tStartIso, tEndIso] (inclusive) and
 * aggregate assistant-turn token usage by model + cost, plus tool/skill/agent
 * use. This is the source codeburn reads; here it's scoped to one loop run.
 */
export function collectSessionTelemetry(sessionFile: string, tStartIso: string, tEndIso: string): SessionTelemetry {
  const out: SessionTelemetry = {
    tokens: { in: 0, out: 0, cache_read: 0, by_model: {}, cost_usd: 0 },
    tokensByModel: {}, skills_used: {}, agents: {},
  };
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

    const model: string = rec.message.model ?? 'unknown';
    const u = rec.message.usage ?? {};
    const inTok = u.input_tokens ?? 0;
    const outTok = u.output_tokens ?? 0;
    const cacheTok = (u.cache_read_input_tokens ?? 0) + (u.cache_creation_input_tokens ?? 0);
    out.tokens.in! += inTok;
    out.tokens.out! += outTok;
    out.tokens.cache_read! += cacheTok;
    out.tokens.by_model![model] = (out.tokens.by_model![model] ?? 0) + inTok + outTok + cacheTok;
    // Eco is driven by COMPUTE tokens (input+output) only — cache-read tokens are
    // not re-processed through the model (that's the point of caching), so they
    // must not inflate the energy estimate.
    out.tokensByModel[model] = (out.tokensByModel[model] ?? 0) + inTok + outTok;
    const p = priceFor(model);
    out.tokens.cost_usd! += (inTok / 1e6) * p.in + (outTok / 1e6) * p.out + (cacheTok / 1e6) * p.cache;

    for (const block of rec.message.content ?? []) {
      if (block?.type !== 'tool_use') continue;
      if (block.name === 'Agent') {
        const kind = block.input?.subagent_type ?? 'agent';
        out.agents[kind] = (out.agents[kind] ?? 0) + 1;
      }
      const skill = skillNameFromTool(block.name, block.input);
      if (skill) out.skills_used[skill] = (out.skills_used[skill] ?? 0) + 1;
    }
  }
  out.tokens.cost_usd = Math.round(out.tokens.cost_usd! * 100) / 100;
  const resolvedDenom = out.tokens.out! || 1;
  out.tokens.read_edit_ratio = Math.round((out.tokens.cache_read! / Math.max(1, out.tokens.in!)) * 10) / 10;
  void resolvedDenom;
  return out;
}
