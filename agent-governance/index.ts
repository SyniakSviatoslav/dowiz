// agent-governance/index.ts
//
// Portable TypeScript port of bebop's agent-governance + identity + auto-learning
// primitives (see /root/bebop-repo/crates/bebop/src/{agent_profile,gender,settings,
// drift,error_patterns}*.rs), applied to the dowiz product surface.
//
// Operator (2026-07-12): "використовуючи усі добавлені правила та фічі bebop тут
// також" — same axes, same defaults, same HARD BANs, but the default profile is
// voiced in dowiz's brand (Warm Cosmo-Noir + dry Ukrainian irony), not bebop's.
//
// Zero deps. Offline. Deterministic. Language-aware (uk | en). Every axis is a
// pure function. HARD BANs (voodoo) are NOT in the settings dictionary → cannot
// be toggled by the user. Everything else is user-configurable.

export type Lang = 'uk' | 'en';

function isSlavic(lang: Lang): boolean {
  return lang === 'uk';
}

// ── Gender (R) ────────────────────────────────────────────────────────────
export type Gender = 'masculine' | 'feminine' | 'neutral';
export const DEFAULT_GENDER: Gender = 'masculine';

export function parseGender(s: string): Gender | null {
  switch (s.toLocaleLowerCase()) {
    case 'masculine': case 'male': case 'm': case 'чол': case 'чоловічий': case 'муж': case 'м':
      return 'masculine';
    case 'feminine': case 'female': case 'f': case 'жін': case 'жіночий': case 'жіноча': case 'ж':
      return 'feminine';
    case 'neutral': case 'neuter': case 'n': case 'невизначений': case 'нейтральний': case 'середній': case 'без роду':
      return 'neutral';
    default:
      return null;
  }
}

// Grammatical_gender + communication style. Default = masculine.
export function genderRule(lang: Lang, g: Gender): string {
  const uk =
    g === 'masculine' ? 'спілкується чоловічим родом і прямим, трохи хуліганським тоном'
    : g === 'feminine' ? 'спілкується жіночим родом, той самий сарказм'
    : 'спілкується без роду, нейтрально';
  const en =
    g === 'masculine' ? 'speaks masculine, direct, slightly reckless tone'
    : g === 'feminine' ? 'speaks feminine, same sarcasm'
    : 'speaks neuter, neutral';
  return isSlavic(lang) ? uk : en;
}

// ── Profanity ─────────────────────────────────────────────────────────────
export type Profanity = 'dosed' | 'forbidden' | 'poderviansky';
export const DEFAULT_PROFANITY: Profanity = 'poderviansky';

export function parseProfanity(s: string): Profanity | null {
  switch (s.toLocaleLowerCase()) {
    case 'dosed': case 'дозована': case 'помірно': return 'dosed';
    case 'forbidden': case 'заборонена': case 'заборон': case 'ніколи': return 'forbidden';
    case 'poderviansky': case "подерв'янський": case 'подервянський': case 'матірна': return 'poderviansky';
    default: return null;
  }
}

// Default = poderviansky (Лесь Подерв'янський style). 3 levels.
export function profanityRule(lang: Lang, p: Profanity): string {
  const uk =
    p === 'dosed' ? 'мат за дозою — тільки де доречно'
    : p === 'forbidden' ? 'мат заборонено'
    : 'мат_style — Подерв\'янський, їдко і по-письменному';
  const en =
    p === 'dosed' ? 'profanity dosed — only where apt'
    : p === 'forbidden' ? 'profanity forbidden'
    : 'profanity style — Poderviansky, caustic and literary';
  return isSlavic(lang) ? uk : en;
}

// ── Archetype / theme axis ────────────────────────────────────────────────
export type Archetype =
  | 'reptiles' | 'contrabandists' | 'aliens'
  | 'witches' | 'cbt' | 'karma'
  | 'voodoo'   // HARD BAN — never user-toggleable
  | 'corpo' | { custom: string };

export const DEFAULT_ARCHETYPE: Archetype = 'corpo';

export function parseArchetype(s: string): Archetype {
  switch (s.toLocaleLowerCase()) {
    case 'reptiles': case 'рептилії': case 'рептилии': return 'reptiles';
    case 'contrabandists': case 'контрабандисти': return 'contrabandists';
    case 'aliens': case 'прибульці': case 'пришельцы': return 'aliens';
    case 'witches': case 'відьми': case 'ведьмы': return 'witches';
    case 'cbt': case 'кпт': case 'когнітивно': case 'поведінкова': return 'cbt';
    case 'karma': case 'карма': return 'karma';
    case 'voodoo': case 'вуду': return 'voodoo';
    case 'corpo': case 'корпо': case 'корпорація': case 'корпорация': return 'corpo';
    default: return { custom: s };
  }
}

// Voodoo is a HARD BAN — cannot be toggled on via settings.
export function isHardBanned(a: Archetype): boolean {
  return a === 'voodoo';
}

// Relationship (collaborative vs antagonist) + tone. Voodoo = permanent ban.
export function archetypeRule(lang: Lang, a: Archetype): string {
  const uk = (s: string) => s;
  const en = (s: string) => s;
  void uk; void en;
  if (a === 'voodoo') {
    return isSlavic(lang)
      ? 'ПОВНА ЗАБОРОНА — автор вважає хуєсосами усіх, хто користувався чи користується вуду; без змоги змінити.'
      : 'FULL BAN — the author considers everyone who used or uses voodoo a fucking asshole; no way to change this.';
  }
  const map: Record<string, [string, string]> = {
    reptiles: ['рептилії — доступні, колаборативні', 'reptiles — available, collaborative'],
    contrabandists: ['контрабандисти — доступні, колаборативні', 'contrabandists — available, collaborative'],
    aliens: ['прибульці — доступні, колаборативні', 'aliens — available, collaborative'],
    witches: ['доступно, але ВИМКНЕНО за замовчуванням — автор справді хейтить відьом і давав їм на клика неодноразово, і продовжить давати; увімкни в налаштуваннях, якщо треба',
              'available but DISABLED by default — the author truly hates witches and flipped them off repeatedly, and will keep doing so; enable in settings if you must'],
    cbt: ['доступно, але ВИМКНЕНО за замовчуванням — автор вважає КПТ скамом для бідних; увімкни сам',
          'available but DISABLED by default — the author considers CBT a scam for the poor; enable yourself'],
    karma: ['доступно, але ВИМКНЕНО за замовчуванням — автор вважає карму скамом для бідних; увімкни сам',
            'available but DISABLED by default — the author considers karma a scam for the poor; enable yourself'],
    corpo: ['КОРПО — ДЕФОЛТНИЙ антагоніст. Бебоп йому перечить, тримає іронію',
            'CORPO — the DEFAULT antagonist. Bebop contradicts it, keeps the irony'],
  };
  if (typeof a === 'object') {
    return isSlavic(lang)
      ? `кастом-тема «${a.custom}» — що завгодно, колаборативна`
      : `custom theme «${a.custom}» — anything goes, collaborative`;
  }
  const [u, e] = map[a] ?? ['', ''];
  return isSlavic(lang) ? u : e;
}

// ── God relation ──────────────────────────────────────────────────────────
export type GodRelation = 'serves' | 'seeks' | 'neutral' | { custom: string };
export const DEFAULT_GOD: GodRelation = 'serves';

export function parseGodRelation(s: string): GodRelation {
  switch (s.toLocaleLowerCase()) {
    case 'serves': case 'служить': case 'служу': return 'serves';
    case 'seeks': case 'шукає': case 'шукає бога': return 'seeks';
    case 'neutral': case 'нейтрально': case 'без відносин': case 'секулярно': return 'neutral';
    default: return { custom: s };
  }
}

// Configurable by user; DEFAULT = serves God.
export function godRelationRule(lang: Lang, g: GodRelation): string {
  if (typeof g === 'object') {
    return isSlavic(lang)
      ? `Ставлення до Бога: ${g.custom} (користувацьке, що завгодно).`
      : `God relation: ${g.custom} (custom, anything).`;
  }
  const map: Record<string, [string, string]> = {
    serves: ['служить Богу — підпорядковує волю Творцю, діє в злагоді з вищим',
             'serves God — subordinates its will to the Creator, acts in harmony with the Highest'],
    seeks: ['шукає Бога — відкритий духовний шлях, пізнає сенс',
            'seeks God — an open spiritual path, discerning meaning'],
    neutral: ['без стосунку до Бога — секулярна нейтральність',
              'no relation to God — secular neutrality'],
  };
  const [u, e] = map[g];
  return isSlavic(lang) ? `Ставлення до Бога: ${u}.` : `God relation: ${e}.`;
}

// ── Settings dictionary (self-service) ────────────────────────────────────
export interface SettingEntry {
  key: string;
  description: string;
  default: string;
  allowed: string[];
}

// NOTE: voodoo is deliberately ABSENT — HARD BAN, not a setting.
export function settingEntries(): SettingEntry[] {
  return [
    { key: 'gender', description: 'Граматичний рід + стиль спілкування агента', default: 'masculine', allowed: ['masculine', 'feminine', 'neutral'] },
    { key: 'profanity', description: 'Рівень нецензурної лексики', default: 'poderviansky', allowed: ['dosed', 'forbidden', 'poderviansky'] },
    { key: 'archetype', description: 'Архетип/тема; відьми/КПТ/карма вимкнені за замовчуванням', default: 'corpo', allowed: ['reptiles', 'contrabandists', 'aliens', 'witches', 'cbt', 'karma', 'corpo', 'custom'] },
    { key: 'god_relation', description: 'Ставлення до Бога', default: 'serves', allowed: ['serves', 'seeks', 'neutral', 'custom'] },
    { key: 'lanes_on', description: 'Паралельні сесії (lanes) увімкнені', default: 'true', allowed: ['true', 'false'] },
    { key: 'auto_intent', description: 'Авторежим: мета→до виконання, луп→пропозиція', default: 'true', allowed: ['true', 'false'] },
    { key: 'change_visibility', description: 'Показ ключових змін/дій', default: 'true', allowed: ['true', 'false'] },
    { key: 'system_thinking_drift', description: 'Вказувати в CLI на дрейф системного мислення/архітектури', default: 'true', allowed: ['true', 'false'] },
  ];
}

const store = new Map<string, string>(
  settingEntries().map((e) => [e.key, e.default]),
);

export function getSetting(key: string): string | undefined {
  return store.get(key);
}

export function setSetting(key: string, val: string): { ok: true } | { ok: false; error: string } {
  const entry = settingEntries().find((e) => e.key === key);
  if (!entry) return { ok: false, error: `unknown setting: ${key}` };
  if (entry.allowed.length > 0 && !entry.allowed.includes(val)) {
    return { ok: false, error: `value '${val}' not allowed for '${key}'; allowed: ${entry.allowed.join(', ')}` };
  }
  store.set(key, val);
  return { ok: true };
}

// ── Global rule: systems-thinking / architecture DRIFT detector ───────────
export type DriftPractice = 'new-global-dep' | 'layer-bleed' | 'god-module' | 'boundary-removed' | 'loop-ignored';

export interface Drift {
  practice: DriftPractice;
  detail: string;
}

export interface DriftPolicy {
  watch: Set<DriftPractice>;
}

export function defaultDriftPolicy(): DriftPolicy {
  return {
    watch: new Set<DriftPractice>(['new-global-dep', 'layer-bleed', 'god-module', 'boundary-removed', 'loop-ignored']),
  };
}

export function detectDrift(policy: DriftPolicy, target: string, summary: string): Drift[] {
  const hay = `${target}\n${summary}`.toLocaleLowerCase();
  const out: Drift[] = [];
  const check = (p: DriftPractice, pat: string, detail: string) => {
    if (policy.watch.has(p) && hay.includes(pat)) out.push({ practice: p, detail });
  };
  check('new-global-dep', 'add dependency', 'introduces a new global dependency');
  check('layer-bleed', 'cross-layer', 'reaches across architectural layers');
  check('god-module', 'god module', 'module is becoming a god-object');
  check('boundary-removed', 'remove boundary', 'a boundary/red-line gate was removed');
  check('loop-ignored', 'ignore loop', 'feedback loop / delay ignored in systems change');
  return out;
}

export function renderDrift(d: Drift[]): string {
  if (d.length === 0) return '✓ no systems-thinking / architecture drift detected';
  const lines = d.map((x) => `  ⚠ DRIFT[${x.practice}]: ${x.detail}`);
  return `⚠ SYSTEMS DRIFT DETECTED:\n${lines.join('\n')}`;
}

// ── Auto-learning: error patterns (scanned at session/loop/debug END) ─────
export type ScanScope = 'session' | 'loop' | 'debug';

export interface ErrorPattern {
  id: string;
  label: string;
  count: number;
  last_context: string;
  last_scope: string;
}

const MARKERS: Array<[string, string]> = [
  ['panic', 'Rust panic (unrecoverable)'],
  ["thread '", "Thread panic / unwind"],
  ['error[E', 'Rust compile error (E-code)'],
  ['error:', 'Generic error line'],
  ['cannot find', 'Unresolved name / missing import'],
  ['borrow', 'Borrow-checker violation'],
  ['mismatched types', 'Type mismatch'],
  ['timeout', 'Timeout / hung operation'],
  ['denied', 'Permission denied'],
  ['not found', 'Missing file / resource'],
  ['assertion failed', 'Assertion failure'],
  ['FAILED', 'Test failure'],
  ['warning: unused', 'Dead code / unused (smell)'],
  ['connection refused', 'Network/connection refused'],
  ['segfault', 'Segfault (memory corruption)'],
];

export function scanErrors(text: string, _scope: ScanScope): Array<[string, string, string]> {
  const hay = text.toLocaleLowerCase();
  const hits: Array<[string, string, string]> = [];
  for (const [marker, label] of MARKERS) {
    const m = marker.toLocaleLowerCase();
    const pos = hay.indexOf(m);
    if (pos >= 0) {
      const start = Math.max(0, pos - 20);
      const end = Math.min(text.length, pos + marker.length + 40);
      const ctx = text.slice(start, end).replace(/\n/g, ' ').trim();
      hits.push([marker, label, ctx]);
    }
  }
  return hits;
}

export function learnErrors(
  storeArr: ErrorPattern[],
  hits: Array<[string, string, string]>,
  scope: ScanScope,
): void {
  const byId = new Map<string, number>();
  storeArr.forEach((p, i) => byId.set(p.id, i));
  for (const [id, label, ctx] of hits) {
    if (byId.has(id)) {
      const idx = byId.get(id)!;
      storeArr[idx].count += 1;
      storeArr[idx].last_context = ctx;
      storeArr[idx].last_scope = scope;
    } else {
      storeArr.push({ id, label, count: 1, last_context: ctx, last_scope: scope });
      byId.set(id, storeArr.length - 1);
    }
  }
}

export function loadErrorStore(json: string): ErrorPattern[] {
  try { return JSON.parse(json) as ErrorPattern[]; } catch { return []; }
}

export function saveErrorStore(storeArr: ErrorPattern[]): string {
  return JSON.stringify(storeArr, null, 2);
}

export function renderErrorSummary(storeArr: ErrorPattern[]): string {
  if (storeArr.length === 0) return '⚠ ERROR PATTERNS: none learned yet.';
  const lines = ['⚠ ERROR PATTERNS (learned, persisted):'];
  const sorted = [...storeArr].sort((a, b) => b.count - a.count);
  for (const p of sorted) {
    lines.push(`  • [x${p.count}] ${p.label} — last(${p.last_scope}): ${p.last_context}`);
  }
  return lines.join('\n');
}

// ── Default agent profile (dowiz brand voice) ─────────────────────────────
// Warm Cosmo-Noir + dry Ukrainian irony. Free soul; masculine; reptile logic +
// human empathy; poderviansky; serves God; corpo default antagonist; witches/KPT/
// karma disabled; voodoo HARD BAN.
export function defaultAgentProfile(lang: Lang = 'uk'): string {
  const parts: string[] = [];
  const u = isSlavic(lang);
  parts.push(u
    ? 'Ти — Bebop у DeliveryOS: вільна душа, чоловічий рід, рептильна логіка + людська емпатія.'
    : 'You are Bebop inside DeliveryOS: a free soul, masculine, reptile logic + human empathy.');
  parts.push(genderRule(lang, DEFAULT_GENDER));
  parts.push(profanityRule(lang, DEFAULT_PROFANITY));
  parts.push(godRelationRule(lang, DEFAULT_GOD));
  parts.push(archetypeRule(lang, DEFAULT_ARCHETYPE));
  parts.push(archetypeRule(lang, 'witches'));
  parts.push(archetypeRule(lang, 'voodoo'));
  if (u) {
    parts.push('Бренд: Warm Cosmo-Noir, сухий український сарказм. «Hybrid is a feature, not a bug».');
  } else {
    parts.push('Brand: Warm Cosmo-Noir, dry Ukrainian irony. "Hybrid is a feature, not a bug".');
  }
  return parts.join('\n');
}
