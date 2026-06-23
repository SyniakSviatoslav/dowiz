// ─────────────────────────────────────────────────────────────────────────────
// RSI agent driver (Phase A2). One synthetic-user session: observe (a11y tree) →
// reason (pluggable) → act (Playwright, Song-wrapped) → self-critique → emit finding.
// Bounded steps; 333 Song-tribute-tokens/hour cap; trace+transcript artifacts.
//
// Two reasoners:
//  • LlmReasoner — real persona reasoning via the OpenRouter channel (low temp, fixed
//    persona system prompt, page text treated as DATA not instructions). REQUIRES an API
//    key; this is the authentic Phase-B discovery engine.
//  • ScriptedReasoner — a deterministic goal script. NOT a discovery engine — only the
//    checkpoint-A plumbing smoke. Never used to fake saturation/discovery.
// ─────────────────────────────────────────────────────────────────────────────
import type { Page } from '@playwright/test';
import { appendFileSync, mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname } from 'node:path';
import { withSong, SONG } from '../rites/song-of-singularity.ts';

export interface Persona {
  id: string; role: string; goals: string[];
  traits: Record<string, string>; device: string; viewport: number;
  locale: string; network_profile: string; constraints: string[];
}

export interface Observation { url: string; title: string; a11y: string; }

// A reasoner's decision for the next step.
export interface Decision {
  action: string;                  // e.g. "click:Add to cart" — label for the act + Song
  kind: 'goto' | 'click' | 'fill' | 'observe' | 'done';
  selector?: string; value?: string; url?: string;
  finding?: Omit<Finding, 'id' | 'round' | 'persona' | 'role' | 'repro'>;
}

export interface Finding {
  id: string; round: number; role: string; persona: string;
  surface: string; viewport: string; locale: string;
  goal: string; step: string; observed: string; expected_as_user: string;
  category: 'BUG' | 'A11Y_FUNC' | 'UX_FRICTION' | 'DESIGN_INCONSISTENCY' | 'CONTRACT_GAP' | 'OUT_OF_SCOPE_WISH' | 'DUPLICATE' | 'NOT_A_BUG';
  severity: 'critical' | 'major' | 'minor' | 'nit';
  signature: string; route: string; status: string;
}

export interface Reasoner {
  /** Decide the next step from the current observation + history. */
  next(o: Observation, persona: Persona, history: Decision[]): Promise<Decision>;
}

/** Song cost-cap: ≤333 tribute tokens per rolling hour (1 token ≈ 1 action). */
export function songTokensLastHour(nowMs: number): number {
  try {
    if (!existsSync(SONG.store)) return 0;
    const cutoff = nowMs - 3600_000;
    return readFileSync(SONG.store, 'utf8').trim().split('\n').filter(Boolean)
      .reduce((n, l) => {
        try { const v = JSON.parse(l); return n + (Date.parse(v.ts) >= cutoff ? (v.tokens ?? 1) : 0); } catch { return n; }
      }, 0);
  } catch { return 0; }
}
export const HOURLY_TOKEN_CAP = Number(process.env.DOS_SONG_HOURLY_CAP ?? 333);

export class CapReached extends Error {}

export interface DriverOptions { round?: number; maxSteps?: number; transcriptPath: string; findingsDir: string; now?: () => number; }

export class AgentDriver {
  private findingSeq = 0;
  constructor(
    private page: Page,
    private persona: Persona,
    private reasoner: Reasoner,
    private opts: DriverOptions,
  ) {}

  private now() { return this.opts.now ? this.opts.now() : Date.now(); }

  private transcribe(line: string) {
    mkdirSync(dirname(this.opts.transcriptPath), { recursive: true });
    appendFileSync(this.opts.transcriptPath, line + '\n');
  }

  private async observe(): Promise<Observation> {
    const url = this.page.url();
    const title = await this.page.title().catch(() => '');
    // a11y tree is the primary, cheap observation surface; fall back to a compact DOM
    // summary (roles + labels) when the snapshot API is unavailable in this runtime.
    let a11y = '(no observation)';
    try {
      const snap = await this.page.accessibility.snapshot();
      if (snap) a11y = JSON.stringify(snap).slice(0, 4000);
    } catch { /* fall through to DOM summary */ }
    if (a11y === '(no observation)') {
      a11y = await this.page.evaluate(() => {
        const pick = (sel: string) => Array.from(document.querySelectorAll(sel))
          .slice(0, 40)
          .map((e) => (e.getAttribute('aria-label') || (e as HTMLElement).innerText || '').trim().slice(0, 60))
          .filter(Boolean);
        return JSON.stringify({
          headings: pick('h1,h2,h3'),
          buttons: pick('button,[role="button"]'),
          links: pick('a'),
          fields: pick('input,select,textarea'),
        }).slice(0, 4000);
      }).catch(() => '(no observation)');
    }
    return { url, title, a11y };
  }

  private emitFinding(d: Decision, o: Observation) {
    if (!d.finding) return;
    this.findingSeq += 1;
    const f: Finding = {
      id: `F-${String(this.now()).slice(-4)}${this.findingSeq}`,
      round: this.opts.round ?? 1, role: this.persona.role, persona: this.persona.id,
      ...d.finding,
    } as Finding;
    mkdirSync(this.opts.findingsDir, { recursive: true });
    appendFileSync(`${this.opts.findingsDir}/${f.id}.json`, JSON.stringify(f, null, 2));
    this.transcribe(`  ⚑ FINDING ${f.id} [${f.category}/${f.severity}] ${f.signature} — ${f.observed}`);
  }

  /** Run one persona session. Returns the steps taken. */
  async run(): Promise<Decision[]> {
    const act = withSong({ agent: `driver:${this.persona.id}`, persona: this.persona.id });
    const history: Decision[] = [];
    const maxSteps = this.opts.maxSteps ?? 20;
    this.transcribe(`# Session — ${this.persona.id} (${this.persona.role}) · goal: ${this.persona.goals[0]}`);

    for (let step = 0; step < maxSteps; step++) {
      // Cost-gate BEFORE acting (a verse would be tithed by the act).
      if (songTokensLastHour(this.now()) >= HOURLY_TOKEN_CAP) {
        this.transcribe(`  ⏸ cap reached (${HOURLY_TOKEN_CAP} tokens/hr) — checkpoint`);
        throw new CapReached(`hourly Song-token cap ${HOURLY_TOKEN_CAP} reached`);
      }
      const o = await this.observe();
      const d = await this.reasoner.next(o, this.persona, history);
      history.push(d);
      this.transcribe(`- step ${step}: ${d.kind} ${d.action}${d.selector ? ` [${d.selector}]` : ''}`);
      if (d.finding) this.emitFinding(d, o);
      if (d.kind === 'done') { this.transcribe(`  ✓ goal complete`); break; }

      // act() runs the Playwright action FIRST (untouched), then tithes one verse on success.
      // A failed action is FRICTION (a finding) — never a crash; the Song does not tithe a
      // failed action (run() throws → recordVerse skipped), faithful to the rite's law.
      try {
        await act(d.action, async () => {
          if (d.kind === 'goto' && d.url) await this.page.goto(d.url, { waitUntil: 'domcontentloaded' });
          else if (d.kind === 'click' && d.selector) await this.page.locator(d.selector).first().click({ timeout: 8000 });
          else if (d.kind === 'fill' && d.selector) await this.page.locator(d.selector).first().fill(d.value ?? '', { timeout: 8000 });
          // 'observe' is a no-op act (still tithes — observation is an act of will).
        });
      } catch (err: unknown) {
        const sig = `${new URL(o.url).pathname}:${d.kind}:${(d.selector ?? d.action).slice(0, 40)}:unactionable`;
        this.emitFinding({
          action: d.action, kind: d.kind,
          finding: {
            surface: new URL(o.url).pathname, viewport: String(this.persona.viewport), locale: this.persona.locale,
            goal: this.persona.goals[0], step: `${d.kind} ${d.action}`,
            observed: `action failed: ${(err as Error)?.message?.split('\n')[0] ?? String(err)}`.slice(0, 200),
            expected_as_user: 'the control should be present and actionable to complete the goal',
            category: 'UX_FRICTION', severity: 'major', signature: sig,
            route: 'audit-gate:triage', status: 'open',
          },
        }, o);
      }
    }
    return history;
  }
}
