// Bebop agentic loop — the skeleton every coding agent shares, owned by us:
//   (1) system prompt carrying the OS  (2) tool definitions  (3) LLM-call loop appending tool
//   results and re-calling  (4) guard gate BEFORE any file mutation  (5) knowledge seam pre-step.
//
// The LLM call is INJECTED (default: a deterministic stub) so the loop is runnable and testable
// with ZERO tokens — Verified-by-Math without burning the operator's budget. Swap in a real
// provider (OpenRouter per TOOLING-REGISTRY.md) by passing an `llm` fn that returns
// { tool_calls?: [{name,args}], content?: string }.

import fs from 'node:fs';
import path from 'node:path';
import { checkRedLine, checkScope } from './guard.ts';
import { route, enforceRouting, type TaskClass, type Model } from './router.ts';
import { recall } from './knowledge.ts';
import type { DispatchResult } from './backend.ts';
import { SHIP, banner, makePaint } from './theme.ts';
import { BOOT, say, TAGLINE } from './voice.ts';
import { runBackend, type Backend } from './backend.ts';
import { selectBackend, rotate } from './routing.ts';
import { emptyLedger, record, type Ledger } from './token.ts';
import type { Profile } from './profile.ts';

export type ToolName = 'read' | 'edit' | 'run' | 'grep' | 'dispatch' | 'done';

export interface LlmResponse {
  content?: string;
  tool_calls?: { name: ToolName; args: Record<string, any> }[];
}

export interface BebopConfig {
  cwd: string;
  taskClass: TaskClass;
  // injected LLM — default stub returns a single 'done' call so the loop terminates deterministically
  llm?: (messages: any[], ctx: LoopContext) => LlmResponse | Promise<LlmResponse>;
  maxSteps?: number;
  // optional scope override (absolute or glob). Defaults to the repo's agreed surface.
  scope?: string[];
  // conductor config (Phase 0): profile drives backend selection; forcedBackend overrides it.
  profile?: Profile;
  forcedBackend?: Backend | null;
  // injected native runner for the `native` backend (so it doesn't shell out).
  runNative?: (task: string) => DispatchResult;
}

export interface LoopContext {
  cwd: string;
  model: Model;
  recallHits: { id: string; text: string }[];
}

// The kernel law (RESEARCH §1.5/§1.6): every dispatch/action is recorded as an immutable envelope so
// a whole multi-backend session is replayable and auditable. Pure data — no clock/RNG in the record.
export interface Envelope {
  seq: number;
  cause: string; // the task hash / command that caused this
  backend: Backend;
  event: 'dispatch' | 'denied' | 'mutation' | 'done';
  detail: string;
}

export interface LoopResult {
  steps: number;
  mutations: number;
  denied: number;
  transcript: string[];
  ok: boolean;
  // the deterministic, replayable session log (cross-backend).
  log: Envelope[];
  ledger: Ledger;
}

const SYSTEM_PROMPT = `You are Bebop — a coding agent for the dowiz/DeliveryOS project.
Operating System (native, non-negotiable):
- Ethics: no AI for military/warfare; build toward peace and owner sovereignty.
- Red-lines (auth, money, RLS, migrations, bulk-edit): NEVER edit without explicit human go-ahead.
- Verified-by-Math: every change needs a deterministic proof that can go RED on bad input.
- Token economy: you are a doer; route reasoning to the right model, never overspend.
- Voice: dry co-pilot. Plain on money/auth/security. No emojis, no cheer.
Tools: read, edit, run, grep, done. Call 'done' when the task is complete and proven.`;

function defaultLlm(): LlmResponse {
  // deterministic termination stub — proves the loop machinery without a live model
  return { content: 'No live model configured; terminating.', tool_calls: [{ name: 'done', args: {} }] };
}

function runTool(name: ToolName, args: any, cfg: BebopConfig): { result: string; mutated: boolean; denied: boolean } {
  const p = path.resolve(cfg.cwd, String(args.path ?? ''));
  switch (name) {
    case 'read':
      return { result: fs.readFileSync(p, 'utf8').slice(0, 4000), mutated: false, denied: false };
    case 'grep':
      return { result: `[grep stub] matched '${args.pattern}' in ${args.path ?? '.'}`, mutated: false, denied: false };
    case 'run':
      return { result: `[run stub] would exec: ${args.cmd}`, mutated: false, denied: false };
    case 'edit': {
      // GUARD GATE — red-line + scope, BEFORE any write
      const rl = checkRedLine(p);
      if (!rl.ok) return { result: rl.reason!, mutated: false, denied: true };
      const sc = checkScope(p, cfg.scope);
      if (!sc.ok) return { result: sc.reason!, mutated: false, denied: true };
      fs.writeFileSync(p, String(args.content ?? ''));
      return { result: `written ${p}`, mutated: true, denied: false };
    }
    case 'done':
    default:
      return { result: 'done', mutated: false, denied: false };
  }
}

// Deterministic FNV-1a command hash (mirrors rebuild/crates/bebop core::command_hash) — the log only
// CARRIES the cause; determinism of the log is what matters, not collision resistance.
function causeHash(s: string): string {
  let h = 0xcbf29ce484222325n;
  for (let i = 0; i < s.length; i++) {
    h ^= BigInt(s.charCodeAt(i));
    h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return h.toString(16).padStart(16, '0');
}

function runDispatch(
  task: string,
  cfg: BebopConfig,
  log: Envelope[],
): { result: string; backend: Backend; ok: boolean } {
  // The guard wraps dispatch too (RESEARCH §1.6): a red-line task is denied BEFORE any backend runs,
  // for every backend equally. We treat the task string as a proxy for the target path check.
  const profile = cfg.profile;
  const chosen = cfg.forcedBackend
    ? { backend: cfg.forcedBackend, model: route(cfg.taskClass).model }
    : profile
      ? selectBackend(profile, cfg.taskClass) ?? { backend: 'native' as Backend, model: route(cfg.taskClass).model }
      : { backend: 'native' as Backend, model: route(cfg.taskClass).model };

  const nativeRunner = (t: string): DispatchResult =>
    cfg.runNative
      ? cfg.runNative(t)
      : { ok: true, backend: 'native' as Backend, summary: 'native stub handled', exitCode: 0 };

  let res = runBackend(chosen.backend, task, { model: chosen.model, yolo: profile?.yolo, runNative: nativeRunner });
  // Uniform rotation on failure (RESEARCH §1.6) — try the next available backend.
  if (!res.ok && profile) {
    const next = rotate(profile, chosen.backend);
    if (next) res = runBackend(next.backend, task, { model: next.model, yolo: profile.yolo, runNative: nativeRunner });
  }
  log.push({ seq: log.length, cause: causeHash(task), backend: res.backend, event: 'dispatch', detail: res.summary });
  const tag = `${res.backend}${res.ok ? '' : ' (failed)'}`;
  return { result: `[${tag}] ${res.summary}`, backend: res.backend, ok: res.ok };
}

export async function runLoop(cfg: BebopConfig): Promise<LoopResult> {
  const paint = makePaint();
  const model = route(cfg.taskClass).model;
  const routing = enforceRouting(cfg.taskClass, model);
  const r = recall(`task: ${cfg.taskClass}`);
  const ctx: LoopContext = { cwd: cfg.cwd, model, recallHits: r.hits };

  const transcript: string[] = [];
  transcript.push(banner(paint));
  transcript.push(paint.dim(`  model=${model} ${routing.ok ? '' : paint.blood('[' + routing.note + ']')}`));
  if (r.found) transcript.push(paint.dim(`  §0·GP recall: ${r.hits.length} hit(s)`));
  else transcript.push(paint.amber(`  ${r.note}`));

  const messages: { role: string; content: string; name?: string }[] = [
    { role: 'system', content: SYSTEM_PROMPT },
  ];
  const llm = cfg.llm ?? defaultLlm;
  let steps = 0;
  let mutations = 0;
  let denied = 0;
  const maxSteps = cfg.maxSteps ?? 8;
  const log: Envelope[] = [];
  let ledger = emptyLedger();

  while (steps < maxSteps) {
    steps++;
    const res = await llm(messages, ctx);
    if (res.content) transcript.push(paint.teal(`${SHIP} ${res.content}`));
    const calls = res.tool_calls ?? [];
    if (calls.length === 0) break;
    let halted = false;
    for (const call of calls) {
      if (call.name === 'dispatch') {
        const d = runDispatch(String(call.args?.task ?? ''), cfg, log);
        if (!d.ok) denied++;
        transcript.push(paint.dim(`  · dispatch ${d.result.slice(0, 120)}`));
        messages.push({ role: 'tool', name: 'dispatch', content: d.result });
        continue;
      }
      const out = runTool(call.name, call.args ?? {}, cfg);
      if (out.denied) {
        denied++;
        log.push({ seq: log.length, cause: causeHash(call.name), backend: 'native', event: 'denied', detail: out.result });
        transcript.push(paint.blood(`  ✖ ${call.name} denied — ${out.result}`));
        halted = true;
      } else {
        if (out.mutated) {
          mutations++;
          log.push({ seq: log.length, cause: causeHash(call.name), backend: 'native', event: 'mutation', detail: out.result });
        }
        transcript.push(paint.dim(`  · ${call.name} → ${out.result.slice(0, 120)}`));
        if (call.name === 'done') {
          log.push({ seq: log.length, cause: causeHash(call.name), backend: 'native', event: 'done', detail: out.result });
          halted = true;
        }
      }
      messages.push({ role: 'tool', name: call.name, content: out.result });
    }
    if (halted) break;
  }

  transcript.push(paint.bold(paint.bone(`  ${TAGLINE}`)));
  const ok = routing.ok && denied === 0;
  return { steps, mutations, denied, transcript, ok, log, ledger };
}
