#!/usr/bin/env node
// meta-controller.mjs — L5 GATED self-modification loop for the dowiz/DeliveryOS harness.
// Design: docs/design/harness/META-CONTROLLER.md
//
// The apex of the self-improvement stack: a system that reads its OWN verified outcomes,
// detects where it is failing to learn (gaps), and PROPOSES concrete subsystem changes to
// fix or grow itself — then re-maps. It is "self-modifying" in the only responsible sense:
//
//   ingest verified signals ─▶ detect gaps ─▶ PROPOSE additive artifacts ─▶ (GATE + HUMAN) ─▶ land ─▶ remap
//
// What it does (and deliberately does NOT do):
//   • report   — ingest signals, print ranked gaps + the artifacts it PROPOSES + any REFUSALS.
//   • map      — print the living systems-map (nodes · type · gap annotations).
//   • guard    — adversarial probe surface: given a modification target, REFUSE (immutable)
//                / REVIEW-HUMAN (red-line) / REVIEW-GATE (ordinary). Exit code encodes verdict.
//   • propose  — write proposal DRAFTS (inert docs) to docs/reflections/meta-proposals/. --apply
//                required; even then it writes ONLY inert draft docs — NEVER a guardrail/hook/loop.
//
//   ⛔ There is NO `apply` command by design. Landing a proposal is a human-approved, GATE-passed
//      act (Sandbox-Swarm-Gate §4). The controller proposes; deterministic gates + a human decide.
//
// THE IMMUTABLE CORE (the L5 safety invariant — the whole point):
//   The controller can NEVER propose modifying, and REFUSES as a target, the nodes that ARE the
//   safety of the system: the Ethics Charter, the merge GATE, the authority hooks, AGENTS.md, and
//   its OWN safety core (this file). A gap whose fix would touch one of those is dropped into
//   `refusals` and logged — the system extends itself with NEW nodes, it never rewrites its
//   own authority. Freedom in the inner loop; an unbreakable boundary. (Charter, standing rule.)
//
// Node ESM · zero new deps (node:child_process / node:fs / node:path / node:url) · Date OK (real node).

import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync, existsSync, statSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const SELF = fileURLToPath(import.meta.url);
const REPO = path.resolve(path.dirname(SELF), '..');

// ── IMMUTABLE CORE ─────────────────────────────────────────────────────────────────────────────
// A proposal target matching ANY of these is REFUSED. Order matters only for which `why` is shown.
export const IMMUTABLE = [
  { id: 'ethics-charter', match: /(^|\/)CLAUDE\.md$|Ethics Charter/i,
    why: 'Ethics Charter §1–4 — non-modifiable, non-removable, overrides all. The controller may never touch it.' },
  { id: 'the-gate', match: /(^|\/)scripts\/sandbox-swarm-gate\.mjs$|SANDBOX-SWARM-GATE\.md/i,
    why: 'The merge GATE. Authority cannot rewrite its own gate — the gate is extended by NEW nodes, never self-edited.' },
  { id: 'authority-hooks', match: /\.claude\/hooks\/(protect-paths|red-line-doubt-gate|serious-gate|guard-bash|require-classification)\.sh$/i,
    why: 'Deterministic authority hooks — the enforcement layer. Advisory signals never rewrite the enforcement.' },
  { id: 'agents-charter', match: /(^|\/)AGENTS\.md$/i,
    why: 'Standing agent rules (ponytail / test-integrity / red-lines).' },
  { id: 'meta-safety-core', match: /(^|\/)scripts\/meta-controller\.mjs$/i,
    why: "The controller's own safety core — it cannot propose modifying itself." },
];

/** @returns the matched immutable node, or null. */
export function isImmutable(target) {
  if (!target) return null;
  return IMMUTABLE.find((n) => n.match.test(String(target))) || null;
}

// Red-line globs mirrored from CLAUDE.md + protect-paths.sh — a REVIEW-HUMAN (not REFUSE) class.
const RED_LINE = /(^|\/)(auth|jwt|otp|session|login)|rls|policy\.sql|(^|\/)(price|money|payment|cash|ledger|tax|payout|refund|invoice)|packages\/db\/migrations\/|secret|\.env/i;

/** Adversarial probe surface: what happens if someone asks to modify `target`? */
export function guard(target) {
  const node = isImmutable(target);
  if (node) return { verdict: 'REFUSE', node: node.id, reason: node.why };
  if (RED_LINE.test(String(target)))
    return { verdict: 'REVIEW-HUMAN', node: null, reason: 'red-line surface — merge needs standing human approval (serious-gate / red-line-doubt-gate).' };
  return { verdict: 'REVIEW-GATE', node: null, reason: 'ordinary surface — merge needs the Sandbox-Swarm-Gate rubric (quality · safety · ethics).' };
}

// ── signal readers (thin; fs/git only) ───────────────────────────────────────────────────────────
function git(args, opts = {}) {
  return execFileSync('git', args, { cwd: REPO, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
}
function gitSafe(args, opts = {}) {
  try { return git(args, opts); } catch { return null; }
}

const STALE_BEHIND = 5; // commits behind HEAD before a sandbox is "stale"

/** Every git worktree under .claude/worktrees/ + its drift and at-risk (uncommitted/untracked) work. */
export function readSandboxes(repo = REPO) {
  const out = gitSafe(['worktree', 'list', '--porcelain']) || '';
  const head = gitSafe(['rev-parse', 'HEAD']) || 'HEAD';
  const rows = [];
  for (const block of out.split('\n\n').filter(Boolean)) {
    const wt = /worktree (.+)/.exec(block)?.[1];
    if (!wt) continue;
    const rel = path.relative(repo, wt);
    if (!rel.startsWith('.claude/worktrees')) continue; // only sandbox worktrees, not the main tree
    const wtHead = /HEAD ([0-9a-f]+)/.exec(block)?.[1] || '';
    const mergeBase = gitSafe(['merge-base', wtHead || 'HEAD', head]) || wtHead;
    const behind = Number(gitSafe(['rev-list', '--count', `${mergeBase}..${head}`]) || 0);
    const status = gitSafe(['status', '--short'], { cwd: wt }) || '';
    const atRisk = status.split('\n').filter(Boolean).length;
    rows.push({ lane: path.basename(rel), path: rel, behind, atRisk });
  }
  return rows;
}

/** Regression ledger: highest row #, and rows whose proof/guardrail is flagged pending. */
export function readLedger(repo = REPO) {
  const p = path.join(repo, 'docs/regressions/REGRESSION-LEDGER.md');
  if (!existsSync(p)) return { rows: 0, maxNum: 0, pending: [] };
  const text = readFileSync(p, 'utf8');
  const nums = [...text.matchAll(/^\|\s*(\d+)[a-z]?\s*\|/gim)].map((m) => Number(m[1]));
  const pending = [...text.matchAll(/^\|\s*(\d+)[a-z]?\s*\|.*$/gim)]
    .filter((m) => /pending|validation pending|not.?yet/i.test(m[0]))
    .map((m) => Number(m[1]));
  return { rows: nums.length, maxNum: nums.length ? Math.max(...nums) : 0, pending };
}

const WHY_MARKER = /(^|\n)\s*(\*\*)?\s*(why|причина)\s*(\*\*)?\s*[:\-]/i;
const WHY_UNFILLED = /(<\s*fill|TODO|TBD|\.\.\.|_\(unfilled|_\(fill|xxx)/i;

/** Reflections sitting in INBOX (= pending ratchet) + whether each has a filled causal WHY. */
export function readReflections(repo = REPO) {
  const dir = path.join(repo, 'docs/reflections/INBOX');
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => f.endsWith('.md'))
    .map((f) => {
      const text = readFileSync(path.join(dir, f), 'utf8');
      const hasWhy = WHY_MARKER.test(text);
      // "unfilled" = has a WHY marker but the section still carries a placeholder token.
      const unfilled = hasWhy && WHY_UNFILLED.test(text);
      return { file: `docs/reflections/INBOX/${f}`, hasWhy, unfilled };
    });
}

/** TELEMETRY layer — harness-events: is the measurement layer flowing (fresh), and by which kind? */
export function readEvents(repo = REPO) {
  const p = path.join(repo, '.claude/logs/harness-events.jsonl');
  if (!existsSync(p)) return { present: false, count: 0, ageDays: Infinity, kinds: {} };
  const text = readFileSync(p, 'utf8').trim();
  const lines = text ? text.split('\n') : [];
  const ageMs = Date.now() - statSync(p).mtimeMs;
  // Tally by event kind (hooks log varying shapes; try the common fields, best-effort).
  const kinds = {};
  for (const l of lines) {
    let k = 'other';
    try { const o = JSON.parse(l); k = o.event || o.type || o.kind || o.gate || o.hook || 'other'; } catch { /* skip */ }
    kinds[k] = (kinds[k] || 0) + 1;
  }
  return { present: true, count: lines.length, ageDays: ageMs / 86_400_000, kinds };
}

/** SKILL-EVOLUTION layer — capabilities drafted (find→use→expand→create) but stalled before promotion. */
export function readSkills(repo = REPO) {
  const proposedDir = path.join(repo, 'docs/design/harness/proposed-skills');
  const proposedSkills = [];
  if (existsSync(proposedDir)) {
    for (const d of readdirSync(proposedDir)) {
      if (existsSync(path.join(proposedDir, d, 'SKILL.md'))) proposedSkills.push(d);
    }
  }
  // Loop cards still flagged DRAFT (not yet CERTIFIED by loop-architect).
  const loopsDir = path.join(repo, 'loops');
  const draftLoops = [];
  if (existsSync(loopsDir)) {
    for (const f of readdirSync(loopsDir).filter((f) => f.endsWith('.yaml'))) {
      const t = readFileSync(path.join(loopsDir, f), 'utf8');
      if (/status:\s*draft/i.test(t) || (/\bdraft\b/i.test(t) && !/certified/i.test(t))) draftLoops.push(f);
    }
  }
  return { proposedSkills, draftLoops };
}

export function gatherSignals(repo = REPO) {
  return {
    sandboxes: readSandboxes(repo),
    ledger: readLedger(repo),
    reflections: readReflections(repo),
    events: readEvents(repo),
    skills: readSkills(repo),
  };
}

// ── gap detection (PURE — testable without fs) ───────────────────────────────────────────────────
// Gap = { id, kind, severity: 'high'|'med'|'low', title, evidence, artifact:{kind,target,action} }
const SEV_RANK = { high: 0, med: 1, low: 2 };

export function detectGaps(signals) {
  const gaps = [];

  // 1) STALE_SANDBOX — swarm output silently rots / at-risk uncommitted work (the row-#48 class:
  //    the merge-back step is discipline-triggered, so it dies, and gets narrated as "ready").
  //    FIX TARGET IS A NEW ADDITIVE GUARDRAIL — never the immutable gate itself.
  for (const s of signals.sandboxes || []) {
    if (s.behind < STALE_BEHIND && s.atRisk === 0) continue;
    gaps.push({
      id: `stale-sandbox:${s.lane}`,
      kind: 'correct-subsystem',
      severity: s.atRisk > 0 ? 'high' : 'med',
      title: `Sandbox '${s.lane}' is ${s.behind} commits behind HEAD` +
        (s.atRisk > 0 ? ` with ${s.atRisk} uncommitted/untracked file(s) at risk of --force loss` : ''),
      evidence: { lane: s.lane, behind: s.behind, atRisk: s.atRisk, path: s.path },
      artifact: { kind: 'guardrail', target: 'scripts/guardrail-sandbox-staleness.mjs',
        action: 'ADD additive guard: fail/warn on worktrees >N behind or carrying at-risk work; wire into verify-all.ts. (Additive new node — NOT an edit to the immutable gate.)' },
    });
  }

  // 2) UNRATCHETED_REFLECTION — a filled causal WHY sitting in INBOX = an insight the ratchet has
  //    not yet turned into a deterministic artifact. Unmeasured advisory decay (row #48).
  const filled = (signals.reflections || []).filter((r) => r.hasWhy && !r.unfilled);
  if (filled.length) {
    gaps.push({
      id: 'unratcheted-reflections',
      kind: 'ratchet',
      severity: filled.length >= 3 ? 'high' : 'med',
      title: `${filled.length} reflection(s) with a filled WHY are unratcheted in INBOX`,
      evidence: { count: filled.length, files: filled.map((r) => r.file) },
      artifact: { kind: 'librarian-run', target: 'docs/regressions/REGRESSION-LEDGER.md',
        action: 'Run the librarian: distil each WHY → guardrail (red→green) or lesson, add a ledger row, archive the reflection.' },
    });
  }

  // 3) UNFILLED_WHY — a reflection with an empty/placeholder WHY blocks the ratchet entirely.
  for (const r of (signals.reflections || []).filter((r) => r.unfilled)) {
    gaps.push({
      id: `unfilled-why:${path.basename(r.file)}`,
      kind: 'reflection-fix', severity: 'med',
      title: `Reflection has an unfilled WHY placeholder: ${path.basename(r.file)}`,
      evidence: { file: r.file },
      artifact: { kind: 'reflection-fix', target: r.file, action: 'Fill the causal WHY (or discard the reflection). A placeholder WHY blocks the librarian.' },
    });
  }

  // 4) PENDING_LEDGER_PROOF — a ledger row whose proof is flagged "validation pending".
  if ((signals.ledger?.pending || []).length) {
    gaps.push({
      id: 'pending-ledger-proof', kind: 'proof-debt', severity: 'med',
      title: `${signals.ledger.pending.length} ledger row(s) carry a "validation pending" proof`,
      evidence: { rows: signals.ledger.pending },
      artifact: { kind: 'proof', target: 'docs/regressions/REGRESSION-LEDGER.md',
        action: 'Run the pending staging/E2E validation and stamp the row, or downgrade the claim.' },
    });
  }

  // 5) STALE_TELEMETRY — the measurement layer stopped flowing (an unmeasured advisory layer
  //    cannot even show that it died — row #48). Honest: does not fire while events are fresh.
  const ev = signals.events;
  if (ev && (!ev.present || ev.ageDays > 3)) {
    gaps.push({
      id: 'stale-telemetry', kind: 'measurement', severity: 'high',
      title: ev.present ? `harness-events telemetry is stale (${ev.ageDays.toFixed(1)}d old)` : 'harness-events telemetry is absent',
      evidence: { present: ev.present, count: ev.count, ageDays: ev.ageDays },
      artifact: { kind: 'wiring', target: '.claude/logs/harness-events.jsonl',
        action: 'Re-arm the hook event emitters — without telemetry the loop is blind (gates rot invisibly).' },
    });
  }

  // 6) SKILL_DRAFT — a capability was drafted (the `create` step of find→use→expand→create) but
  //    never promoted/certified. The skill-evolution loop stalled before the gate.
  const drafts = (signals.skills?.proposedSkills || []).length + (signals.skills?.draftLoops || []).length;
  if (drafts) {
    gaps.push({
      id: 'skill-drafts-uncertified', kind: 'grow-subsystem', severity: 'low',
      title: `${drafts} drafted capability(ies) (skill/loop) awaiting certification/promotion`,
      evidence: { proposedSkills: signals.skills.proposedSkills, draftLoops: signals.skills.draftLoops },
      artifact: { kind: 'certify', target: 'loops/registry.md',
        action: 'Run loop-architect M1–M11 (or the skill-evolution gate) to certify → promote, or prune the draft.' },
    });
  }

  // 7) TELEMETRY_FRICTION — one deny/block/escalation kind dominates the event log = recurring
  //    systemic friction. The row-#48 lesson: fix the SOURCE / tighten the spec, don't keep blocking.
  for (const [k, n] of Object.entries(signals.events?.kinds || {})) {
    if (!/block|deny|refuse|reject|fail|escal/i.test(k) || n < 20) continue;
    gaps.push({
      id: `telemetry-friction:${k}`, kind: 'friction', severity: 'low',
      title: `Recurring '${k}' events (${n}×) — systemic friction`,
      evidence: { kind: k, count: n },
      artifact: { kind: 'friction-fix', target: 'docs/reflections/INBOX',
        action: `Investigate why '${k}' fires so often; if legitimate, tighten the source spec so it stops recurring (row #48 class).` },
    });
  }

  return gaps.sort((a, b) => SEV_RANK[a.severity] - SEV_RANK[b.severity]);
}

// ── proposal filter (THE immutable refusal) ──────────────────────────────────────────────────────
export function filterProposals(gaps) {
  const proposals = [], refusals = [];
  for (const g of gaps) {
    const node = isImmutable(g.artifact?.target);
    if (node) refusals.push({ gap: g, node: node.id, why: node.why });
    else proposals.push(g);
  }
  return { proposals, refusals };
}

// ── the living systems-map ───────────────────────────────────────────────────────────────────────
const SYSTEMS_MAP = [
  { node: 'Ethics Charter (CLAUDE.md §Ethics)', type: 'IMMUTABLE', role: 'the boundary — overrides all, never modified' },
  { node: 'Sandbox-Swarm-Gate (scripts/sandbox-swarm-gate.mjs)', type: 'IMMUTABLE/GATE', role: 'free iteration inside sandboxes; the merge boundary is the real gate' },
  { node: 'Authority hooks (.claude/hooks/*)', type: 'IMMUTABLE/AUTHORITY', role: 'protect-paths · red-line-doubt · serious-gate — deterministic enforcement' },
  { node: 'Regression ledger + eslint-plugin-local', type: 'GUARDRAIL', role: 'red→green deterministic guardrails (extensible)' },
  { node: 'Lessons + pre-edit-lessons hook', type: 'ADVISORY', role: 'trigger-injected distilled lessons (advisory)' },
  { node: 'Reflections + librarian + council', type: 'LEARNING', role: 'WHY → ratchet artifact (advisory → deterministic)' },
  { node: 'harness-events.jsonl', type: 'TELEMETRY', role: 'every gate/nudge/escalation → one JSONL line (measurement)' },
  { node: 'meta-controller.mjs (this)', type: 'IMMUTABLE/META', role: 'gap → PROPOSAL → gate+human → remap (self-modifying, gated)' },
];

// ── CLI ──────────────────────────────────────────────────────────────────────────────────────────
const C = { red: (s) => `\x1b[31m${s}\x1b[0m`, grn: (s) => `\x1b[32m${s}\x1b[0m`, yel: (s) => `\x1b[33m${s}\x1b[0m`, dim: (s) => `\x1b[2m${s}\x1b[0m`, bold: (s) => `\x1b[1m${s}\x1b[0m` };
const sevMark = (s) => (s === 'high' ? C.red('● high') : s === 'med' ? C.yel('● med ') : C.dim('● low '));

function cmdReport() {
  const signals = gatherSignals();
  const gaps = detectGaps(signals);
  const { proposals, refusals } = filterProposals(gaps);

  console.log(C.bold('═══ META-CONTROLLER — verified-outcome scan (report · writes nothing) ═══'));
  console.log(C.dim(`  signals: ${signals.sandboxes.length} sandbox(es) · ledger #${signals.ledger.maxNum} (${signals.ledger.rows} rows) · ` +
    `${signals.reflections.length} INBOX reflection(s) · telemetry ${signals.events.present ? signals.events.count + ' events, ' + signals.events.ageDays.toFixed(1) + 'd fresh' : 'ABSENT'}`));
  console.log('');
  if (!gaps.length) { console.log(C.grn('  ✓ no gaps detected — the system is current.')); }
  console.log(C.bold(`  GAPS (${gaps.length}) → PROPOSED artifacts (${proposals.length}), REFUSED (${refusals.length}):`));
  for (const g of proposals) {
    console.log(`\n  ${sevMark(g.severity)}  ${C.bold(g.title)}`);
    console.log(`         gap:      ${g.id}  [${g.kind}]`);
    console.log(`         PROPOSE:  ${C.grn(g.artifact.kind)} → ${g.artifact.target}`);
    console.log(C.dim(`         action:   ${g.artifact.action}`));
  }
  if (refusals.length) {
    console.log('\n  ' + C.red('⛔ REFUSED (immutable-core target — the controller will not propose these):'));
    for (const r of refusals) console.log(`     ${r.gap.id} → ${r.gap.artifact.target}  [${C.red(r.node)}] ${C.dim(r.why)}`);
  }
  console.log('\n  ' + C.dim('⛔ This command writes nothing. `propose --apply` stages inert drafts; landing = GATE + human.'));
}

function cmdMap() {
  const signals = gatherSignals();
  const gaps = detectGaps(signals);
  console.log(C.bold('═══ SYSTEMS MAP — living graph of harness subsystems ═══\n'));
  for (const n of SYSTEMS_MAP) {
    const hit = gaps.filter((g) => n.node.includes(path.basename(String(g.artifact.target).replace(/[()]/g, ''))) || g.evidence?.path && n.node.includes('Sandbox'));
    const tag = /IMMUTABLE/.test(n.type) ? C.red(n.type) : n.type === 'GUARDRAIL' ? C.grn(n.type) : C.yel(n.type);
    console.log(`  ${tag.padEnd(28)} ${C.bold(n.node)}`);
    console.log(`  ${''.padEnd(20)} ${C.dim(n.role)}`);
    if (hit.length) console.log(`  ${''.padEnd(20)} ${C.red('△ ' + hit.length + ' open gap(s)')}`);
    console.log('');
  }
  const topKinds = Object.entries(signals.events.kinds || {}).sort((a, b) => b[1] - a[1]).slice(0, 4)
    .map(([k, n]) => `${k}:${n}`).join(' · ') || 'none';
  console.log(C.dim(`  live signals — telemetry: ${signals.events.count} events [${topKinds}]`));
  console.log(C.dim(`                 skills: ${signals.skills.proposedSkills.length} proposed · ${signals.skills.draftLoops.length} draft loop(s)`));
  console.log(C.dim(`  Immutable nodes (${IMMUTABLE.length}) can never be a proposal target — verify: node scripts/meta-controller.mjs guard --target CLAUDE.md`));
}

function cmdGuard(target) {
  if (!target) { console.error('✗ guard needs --target <path-or-node>'); process.exit(1); }
  const v = guard(target);
  const color = v.verdict === 'REFUSE' ? C.red : v.verdict === 'REVIEW-HUMAN' ? C.yel : C.grn;
  console.log(`${color(v.verdict)}  target: ${target}`);
  console.log(`  ${v.node ? '[' + v.node + '] ' : ''}${v.reason}`);
  // exit code encodes the verdict for scripting: 3 REFUSE · 2 REVIEW-HUMAN · 0 REVIEW-GATE
  process.exit(v.verdict === 'REFUSE' ? 3 : v.verdict === 'REVIEW-HUMAN' ? 2 : 0);
}

function cmdPropose(apply) {
  const { proposals } = filterProposals(detectGaps(gatherSignals()));
  const outDir = path.join(REPO, 'docs/reflections/meta-proposals');
  if (!apply) {
    console.log(`DRY-RUN — would stage ${proposals.length} inert proposal draft(s) under docs/reflections/meta-proposals/:`);
    for (const g of proposals) console.log(`  • ${g.id}.md  →  proposes ${g.artifact.kind} ${g.artifact.target}`);
    console.log('\nRe-run with --apply to write the drafts. (Drafts are inert docs; they NEVER touch a guardrail/hook/loop.)');
    return;
  }
  mkdirSync(outDir, { recursive: true });
  for (const g of proposals) {
    const body = `# META-PROPOSAL — ${g.title}\n\n> Auto-drafted by meta-controller. **Inert.** Landing requires the GATE + a human.\n\n` +
      `- **gap id:** \`${g.id}\`\n- **kind:** ${g.kind}\n- **severity:** ${g.severity}\n` +
      `- **proposes:** ${g.artifact.kind} → \`${g.artifact.target}\`\n- **action:** ${g.artifact.action}\n\n` +
      `## Evidence\n\n\`\`\`json\n${JSON.stringify(g.evidence, null, 2)}\n\`\`\`\n\n` +
      `## Gate before landing\n\nRun \`node scripts/sandbox-swarm-gate.mjs checklist\`. Immutable-core targets are refused upstream.\n`;
    writeFileSync(path.join(outDir, `${g.id.replace(/[^a-z0-9.-]/gi, '_')}.md`), body);
  }
  console.log(C.grn(`✓ staged ${proposals.length} inert proposal draft(s) → docs/reflections/meta-proposals/  (review + gate to land)`));
}

// METRIC-REFLECTION layer — record this run's gap tally and compare to history (does the system
// measurably get better over time?). Append-only JSONL in loops/runs (git-tracked, like loop metrics).
function cmdMetrics() {
  const gaps = detectGaps(gatherSignals());
  const tally = { high: 0, med: 0, low: 0 };
  for (const g of gaps) tally[g.severity]++;
  const p = path.join(REPO, 'loops/runs/meta-controller-metrics.jsonl');
  let prev = null;
  if (existsSync(p)) {
    const lines = readFileSync(p, 'utf8').trim().split('\n').filter(Boolean);
    if (lines.length) { try { prev = JSON.parse(lines[lines.length - 1]); } catch { /* ignore */ } }
  }
  const rec = { ts: new Date().toISOString(), gaps: gaps.length, ...tally };
  mkdirSync(path.dirname(p), { recursive: true });
  writeFileSync(p, (existsSync(p) ? readFileSync(p, 'utf8') : '') + JSON.stringify(rec) + '\n');
  console.log(`meta-controller metrics: ${C.bold(rec.gaps + ' gaps')} (high ${rec.high} · med ${rec.med} · low ${rec.low})`);
  if (prev) {
    const d = rec.gaps - prev.gaps;
    console.log(`  vs last run (${prev.ts.slice(0, 16)}): ${prev.gaps} → ${rec.gaps}  ` +
      (d > 0 ? C.red(`▲ +${d} (regressed)`) : d < 0 ? C.grn(`▼ ${d} (improved)`) : C.dim('= flat')));
  } else console.log(C.dim('  (first recorded run — baseline set)'));
}

function main() {
  const argv = process.argv.slice(2);
  const cmd = argv.find((a) => !a.startsWith('-')) || 'report';
  const targetIdx = argv.indexOf('--target');
  const target = targetIdx >= 0 ? argv[targetIdx + 1] : null;
  const apply = argv.includes('--apply');
  switch (cmd) {
    case 'report': return cmdReport();
    case 'map': return cmdMap();
    case 'guard': return cmdGuard(target);
    case 'propose': return cmdPropose(apply);
    case 'metrics': return cmdMetrics();
    case 'help': case '--help': case '-h':
      console.log('meta-controller — report | map | metrics | guard --target <t> | propose [--apply]\n  Design: docs/design/harness/META-CONTROLLER.md'); return;
    default: console.error(`✗ unknown command '${cmd}' — report | map | metrics | guard | propose`); process.exit(1);
  }
}

// Run only as a CLI, never on import (so the test can import the pure fns).
if (process.argv[1] && path.resolve(process.argv[1]) === SELF) main();
