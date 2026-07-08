// Bebop backend adapters — the conductor's thin executors.
//
// Governing principle (RESEARCH.md §1.6): cross-cutting layers (guard, token, routing, memory) are
// OWNED by Bebop and applied identically to every backend. A backend adapter is therefore THIN: it
// only translates Bebop's canonical task + envelope into that CLI's invocation flags and parses its
// stdout. It does NOT decide routing, hold red-line logic, or meter tokens. The intelligence lives in
// Bebop; the backend is a dumb executor behind a uniform policy envelope.
//
// Ground truth: the invocation shapes mirror scripts/agents-mesh.sh (the existing mesh we promote to
// a first-class core). `native` is Bebop's own deterministic loop (loop.ts).

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export type Backend =
  | 'opencode'
  | 'codex'
  | 'claude'
  | 'hermes'
  | 'aider'
  | 'goose'
  | 'native';

export interface DispatchResult {
  ok: boolean;
  backend: Backend;
  summary: string;
  exitCode: number;
}

export interface BackendAdapter {
  id: Backend;
  /** Human label shown in the selector. */
  label: string;
  /** The binary name Bebop shells out to. `native` has none. */
  binary: string | null;
  /** Env vars that must be present for this backend to be usable (BYOK, read from the vault). */
  requiredEnv: string[];
  /** True if the binary is installed / resolvable on this machine. */
  detect(): boolean;
  /** Build the argv for a one-shot task run. */
  buildArgs(task: string, opts: { model?: string; yolo?: boolean }): string[];
  /** Parse raw stdout into a short summary for the envelope log. */
  parse(stdout: string): string;
}

function which(bin: string): boolean {
  try {
    execFileSync(process.platform === 'win32' ? 'where' : 'which', [bin], {
      stdio: 'ignore',
      timeout: 5000,
    });
    return true;
  } catch {
    return false;
  }
}

function hasEnv(vars: string[]): boolean {
  return vars.some((v) => !!process.env[v]);
}

function defaultParse(stdout: string): string {
  const lines = stdout.split('\n').map((l) => l.trim()).filter(Boolean);
  return lines.slice(-3).join(' ').slice(0, 200) || '(no output)';
}

export const ADAPTERS: Record<Backend, BackendAdapter> = {
  opencode: {
    id: 'opencode',
    label: 'OpenCode',
    binary: 'opencode',
    requiredEnv: ['ANTHROPIC_API_KEY', 'OPENAI_API_KEY', 'OPENROUTER_API_KEY', 'GOOGLE_GENERATIVE_AI_API_KEY', 'GEMINI_API_KEY'],
    detect: () => which('opencode'),
    buildArgs: (task) => ['run', task],
    parse: defaultParse,
  },
  codex: {
    id: 'codex',
    label: 'Codex CLI',
    binary: 'codex',
    requiredEnv: ['OPENAI_API_KEY', 'CODEX_API_KEY'],
    detect: () => which('codex'),
    buildArgs: (task, opts) => {
      const a = ['exec', '--full-auto', task];
      if (opts.model) a.push('--model', opts.model);
      return a;
    },
    parse: defaultParse,
  },
  claude: {
    id: 'claude',
    label: 'Claude Code',
    binary: 'claude',
    requiredEnv: ['ANTHROPIC_API_KEY'],
    detect: () => which('claude'),
    buildArgs: (task, opts) => {
      const a = ['-p', task, '--dangerously-skip-permissions'];
      if (opts.model) a.push('--model', opts.model);
      return a;
    },
    parse: defaultParse,
  },
  hermes: {
    id: 'hermes',
    label: 'Hermes',
    binary: 'hermes',
    requiredEnv: ['ANTHROPIC_API_KEY', 'GOOGLE_API_KEY', 'GEMINI_API_KEY', 'OPENROUTER_API_KEY'],
    detect: () => which('hermes'),
    buildArgs: (task, opts) => {
      const a = ['chat', '-q', task, '--checkpoints', '--source', 'tool'];
      if (opts.yolo) a.push('--yolo');
      return a;
    },
    parse: defaultParse,
  },
  aider: {
    id: 'aider',
    label: 'Aider',
    binary: 'aider',
    requiredEnv: ['ANTHROPIC_API_KEY', 'OPENAI_API_KEY', 'OPENROUTER_API_KEY', 'GEMINI_API_KEY', 'DEEPSEEK_API_KEY'],
    detect: () => which('aider'),
    buildArgs: (task, opts) => {
      // aider one-shot needs --yes-always to run without a TTY (mesh rule: skipped unless yolo).
      const a = ['--message', task];
      if (opts.yolo) a.push('--yes-always');
      if (opts.model) a.push('--model', opts.model);
      return a;
    },
    parse: defaultParse,
  },
  goose: {
    id: 'goose',
    label: 'Goose',
    binary: 'goose',
    requiredEnv: ['OPENAI_API_KEY', 'ANTHROPIC_API_KEY', 'GOOGLE_API_KEY', 'OPENROUTER_API_KEY'],
    detect: () => which('goose'),
    buildArgs: (task) => ['run', '--no-session', '-t', task],
    parse: defaultParse,
  },
  native: {
    id: 'native',
    label: 'Bebop native loop',
    binary: null,
    requiredEnv: [],
    detect: () => true,
    buildArgs: () => [],
    parse: (s) => s.slice(0, 200),
  },
};

/** A backend is "available" if installed AND has its required keys (or is native). */
export function isAvailable(b: Backend): boolean {
  const a = ADAPTERS[b];
  if (!a.detect()) return false;
  if (a.requiredEnv.length === 0) return true; // native
  return hasEnv(a.requiredEnv);
}

/** Run a backend. `runNative` is injected so `native` doesn't shell out (no binary). */
export function runBackend(
  b: Backend,
  task: string,
  opts: { model?: string; yolo?: boolean; runNative?: (task: string) => DispatchResult },
): DispatchResult {
  const a = ADAPTERS[b];
  if (b === 'native') {
    return opts.runNative ? opts.runNative(task) : { ok: false, backend: b, summary: 'no native runner', exitCode: 1 };
  }
  // Safety: never shell out to an unavailable backend (not installed or missing keys). This prevents
  // hanging on a missing/blocking binary and enforces the conductor's availability gate uniformly.
  if (!isAvailable(b)) {
    return { ok: false, backend: b, summary: `${a.label} unavailable (not installed or missing ${a.requiredEnv.join('/')})`, exitCode: 2 };
  }
  if (!a.binary) return { ok: false, backend: b, summary: 'no binary', exitCode: 2 };
  try {
    const out = execFileSync(a.binary, a.buildArgs(task, opts), {
      encoding: 'utf8',
      timeout: 120_000,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return { ok: true, backend: b, summary: a.parse(out), exitCode: 0 };
  } catch (e: any) {
    const code = typeof e.status === 'number' ? e.status : 1;
    const stderr = (e.stderr ?? '').toString().slice(0, 200);
    return { ok: false, backend: b, summary: stderr || a.parse((e.stdout ?? '').toString()), exitCode: code };
  }
}

/** Cheap liveness probe — never runs a task, just asks the binary for a version/help flag. */
export function healthProbe(b: Backend): boolean {
  const a = ADAPTERS[b];
  if (b === 'native') return true;
  if (!a.binary) return false;
  try {
    execFileSync(a.binary, ['--version'], { stdio: 'ignore', timeout: 5000 });
    return true;
  } catch {
    try {
      execFileSync(a.binary, ['--help'], { stdio: 'ignore', timeout: 5000 });
      return true;
    } catch {
      return false;
    }
  }
}

// Keep fs/os/path referenced for future native-run persistence (no lint noise).
export const _internal = { fs, os, path };
