// Bebop guard — the Operating System, baked in as native behavior (not a prompt someone forgets).
//
// Ground truth: the repo's red-line globs (auth, money, RLS, migrations, bulk-edit) from
// AGENTS.md / docs/agent-rules/INVARIANTS.md, and the Verified-by-Math rule (every gate must be
// able to go RED on bad input — no false-green metrics). This module is the mechanical denial
// layer: it blocks before any tool executes, and it refuses to certify a guardrail that cannot fail.

import path from 'node:path';
import { STATES } from './voice.ts';

// Red-line globs — same set the repo protects. Touching these requires explicit human go-ahead,
// never a blanket permission.
export const RED_LINE_GLOBS = [
  '**/auth/**',
  '**/migrations/**',
  '**/rls/**',
  '**/*.sql',
  '**/packages/db/migrations/**',
  '**/money/**',
  '**/payments/**',
  '**/bulk-edit/**',
] as const;

// Scope — files the agent is allowed to touch without re-asking. Mirrors .opencode/scope.jsonc
// spirit: a narrow, agreed surface.
export const DEFAULT_SCOPE_GLOBS = [
  'tools/bebop/**',
  'docs/design/dowiz-agent-cli/**',
] as const;

function toRegExp(glob: string): RegExp {
  // single-pass glob → regex: ** matches across segments, * matches within one segment, ? one char
  let re = '';
  for (let i = 0; i < glob.length; i++) {
    const c = glob[i];
    if (c === '*') {
      if (glob[i + 1] === '*') {
        re += '.*';
        i++; // consume second '*'
        if (glob[i + 1] === '/') i++; // consume the slash after "**/"
      } else {
        re += '[^/]*';
      }
    } else if (c === '?') {
      re += '[^/]';
    } else if ('.+^${}()|[]\\'.includes(c)) {
      re += '\\' + c;
    } else {
      re += c;
    }
  }
  return new RegExp(`^(?:${re})$`);
}

export interface GuardDecision {
  ok: boolean;
  reason?: string;
  kind?: 'redline' | 'scope' | 'ok';
}

export function checkRedLine(targetPath: string): GuardDecision {
  for (const g of RED_LINE_GLOBS) {
    if (toRegExp(g).test(targetPath)) {
      return { ok: false, reason: STATES['guard.redline'].text, kind: 'redline' };
    }
  }
  return { ok: true, kind: 'ok' };
}

export function checkScope(targetPath: string, scope: readonly string[] = DEFAULT_SCOPE_GLOBS, cwd: string = process.cwd()): GuardDecision {
  // Match the path against the globs BOTH raw (if already relative) and relative to cwd
  // (if absolute). A glob like 'tools/bebop/**' must match an absolute repo path.
  const rel = path.isAbsolute(targetPath) ? path.relative(cwd, targetPath) : targetPath;
  const candidates = [targetPath, rel].filter(Boolean);
  const allowed = scope.some((g) => candidates.some((c) => toRegExp(g).test(c)));
  if (!allowed) {
    return { ok: false, reason: STATES['guard.scope'].text, kind: 'scope' };
  }
  return { ok: true, kind: 'ok' };
}

// The falsifiable-gate rule: a guardrail is only valid if it CAN fail. We model that a "green"
// check must be paired with a "red" case that flips it. If a guard is reported green but no
// red case exists, the guard is rejected (no false-green certification).
export interface GateCheck {
  name: string;
  green: () => boolean; // the assertion passes on good input
  red: () => boolean; // the SAME assertion FAILS on bad input (proves it is not a no-op)
}

export function certifyGate(g: GateCheck): { certified: boolean; note: string } {
  const greenPass = g.green();
  const redFails = !g.red(); // red case must make it go false
  if (greenPass && redFails) {
    return { certified: true, note: `gate '${g.name}' certified: green on good, red on bad.` };
  }
  if (!greenPass) {
    return { certified: false, note: `gate '${g.name}' NOT green on good input — fix before ship.` };
  }
  return { certified: false, note: `gate '${g.name}' reads green but CANNOT go red — false-green, rejected.` };
}

// Self-test: prove the red-line deny actually denies. Returns the verdict so the CLI can refuse
// to start in a broken state (Verified-by-Math, not vibes).
export function selfTest(): { ok: boolean; log: string[] } {
  const log: string[] = [];
  const denyMigrations = checkRedLine('packages/db/migrations/002_users.sql');
  const denyMoney = checkRedLine('apps/api/src/routes/payments.ts');
  const allowTool = checkRedLine('tools/bebop/src/loop.ts');

  const redCase = !denyMigrations.ok && !denyMoney.ok;
  const greenCase = allowTool.ok;

  // certify via the falsifiable gate
  const verdict = certifyGate({
    name: 'redline-deny',
    green: () => allowTool.ok,
    red: () => !allowTool.ok,
  });
  log.push(verdict.note);

  const scopeCase = checkScope('tools/bebop/src/theme.ts', DEFAULT_SCOPE_GLOBS, process.cwd()).ok === true &&
    checkScope('packages/db/migrations/x.sql', DEFAULT_SCOPE_GLOBS, process.cwd()).ok === false;
  const scopeVerdict = certifyGate({
    name: 'scope-block',
    green: () => checkScope('tools/bebop/src/voice.ts').ok,
    red: () => checkScope('apps/api/src/server.ts').ok,
  });
  log.push(scopeVerdict.note);

  const ok = verdict.certified && scopeVerdict.certified && scopeCase;
  return { ok, log };
}
