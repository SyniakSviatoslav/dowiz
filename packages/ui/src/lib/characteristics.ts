// Menu Characteristics — the L2 DESCRIPTIVE derivation seam (council: menu-characteristics-model).
//
// Two safety invariants are LAW here, enforced by guardrail #6 (characteristics.test.ts, red→green):
//   (1) descriptive ≠ regulated — a descriptive label may NEVER carry an energy / satiety / health /
//       nutrient-content meaning ("light", "low/high in", "source of", "filling", "healthy"…). Those are
//       regulated nutrition/health claims (EU 1924/2006 + Albanian food law). The regulated subset is
//       OWNER-gated + legal-anchor-gated ELSEWHERE; it is NEVER platform-asserted as a descriptive chip.
//   (2) closed allowlist — only a human-reviewed (en+sq) label may ever render. The allowlist is EMPTY in
//       v1 (taste-first): every candidate must clear register review before it is added.
//
// Derivation runs client-side as a pure function (council decision — NOT in read_public_menu / the hot path).

// The REGULATED REGISTER (denylist) — matched by MEANING, not just the exact word, in en + sq. A candidate
// descriptive label matching ANY of these is forbidden from the allowlist. Over-matching here is SAFE (it
// only ever FORBIDS a label). Extend, never weaken — weakening is what guardrail #6's anti-vacuity test
// catches.
export const REGULATED_REGISTER: readonly RegExp[] = Object.freeze([
  // energy / calorie (en)
  /\blight\b/i, /\blite\b/i, /\blow[- ]?(cal|calorie|calories|energy|fat|sugar|salt|sodium|carb)/i,
  /\bcalorie/i, /\bdiet\b/i, /\bslim/i, /\breduced[- ]?(fat|sugar|calorie|energy)/i,
  // satiety / fullness (en)
  /\bfilling\b/i, /\bsatiat/i, /\bsatisfying\b/i, /\bkeeps?\s+you\s+full/i, /\bhearty\b/i, /\bfull\s+for\s+hours/i,
  // nutrient-content claims (en)
  /\b(high|rich|source|loaded|packed)\b[\s\w]*\b(in|of|with)\b/i, /[- ]rich\b/i, /\bhigh[- ]?protein\b/i,
  /\b(protein|fibre|fiber|vitamin|calcium|iron|omega)[- ]?(rich|packed|boost)/i,
  // health / body-effect (en)
  /\bhealthy\b/i, /\bhealthful\b/i, /\bwholesome\b/i, /\bgood\s+for\s+you\b/i, /\bnutritious\b/i,
  /\bguilt[- ]?free\b/i, /\bsuperfood\b/i, /\bclean\s+eating\b/i, /\bdetox\b/i,
  // Albanian (sq) — substring-loose on purpose
  /leht[ëe]/i,        // "i lehtë" = light
  /kalori/i,          // "pak kalori" / "kalori" = calorie
  /pasur\s+me/i,      // "i pasur me …" = rich in
  /ngop/i,            // "ngopës" = filling / satiating
  /sh[ëe]ndet/i,      // "i shëndetshëm" = healthy
  /diet/i,            // "dietik" = diet
  /proteina[\s-]?(lart|pasur)/i, // "proteina të larta" = high protein
]);

export function isRegulatedTerm(label: string): boolean {
  return REGULATED_REGISTER.some((re) => re.test(label));
}

// The CLOSED, human-reviewed descriptive allowlist. EMPTY in v1 — every candidate (e.g. hearty/rich/
// carb-forward/protein-forward) must clear en+sq register review (and pass guardrail #6) before being added.
// Until then the descriptive band renders nothing; this is the safety default, not an oversight.
export const DESCRIPTIVE_ALLOWLIST: readonly string[] = Object.freeze([]);

// Runtime safety gate — the complement of guardrail #6. Given any candidate descriptive labels, emit ONLY
// those that are in the reviewed allowlist AND clear the regulated register. The double check means a
// regulated term can never surface even if it were mistakenly proposed at the call site.
export function selectDescriptiveLabels(candidates: readonly string[]): string[] {
  return candidates.filter((c) => DESCRIPTIVE_ALLOWLIST.includes(c) && !isRegulatedTerm(c));
}

// ── REGULATED L2 subset (light / low-calorie / source-of-protein …) ────────────────────────────────────
// These ARE regulated nutrition claims (EU 1924/2006 + Albanian food law). They are NEVER platform-asserted
// by default. Rendering one requires ALL THREE: (a) a VERIFIED per-market legal anchor in the table below,
// (b) owner authority (opt-in, enforced at the data/render layer), and (c) the regulated flag ON. The table
// is EMPTY until a human supplies anchors verified against the actual regulation text — NEVER from memory.
// Guardrail #2 keeps the subset dark until then.
export interface RegulatedAnchor {
  label: string;                 // the regulated term (en)
  market: 'EU' | 'AL';
  basis: string;                 // the numeric condition that licenses the claim (human-readable)
  citation: string;              // the regulation + article (e.g. "Reg (EC) 1924/2006, Annex — 'low energy'")
  verifiedBy?: string;           // operator/legal sign-off; an anchor with no verifiedBy is INERT (never activates)
}

// EMPTY in v1 — NEEDS-HUMAN. Supplying a verified anchor here (with citation + verifiedBy) is the deliberate,
// audited act that licenses one regulated label in one market. Do not populate from memory.
export const REGULATED_ANCHORS: readonly RegulatedAnchor[] = Object.freeze([]);

function anchorActive(a: RegulatedAnchor): boolean {
  return !!a.verifiedBy && a.verifiedBy.trim().length > 0;
}

// Global legal-anchor gate: regulated labels may render only when the flag is on AND ≥1 verified anchor exists.
// (Owner-authority is the second lock, enforced where the label is derived/rendered.)
export function regulatedSubsetActive(flagOn: boolean): boolean {
  return flagOn && REGULATED_ANCHORS.some(anchorActive);
}

export function activeRegulatedAnchor(label: string, market: 'EU' | 'AL'): RegulatedAnchor | undefined {
  return REGULATED_ANCHORS.find((a) => a.label === label && a.market === market && anchorActive(a));
}

// ── ALLERGEN SURFACE (DETAIL-FLOOR-ONLY — council #5 floor + #4-positive) ────────────────────────────────
// Pure computation of the storefront allergen surface. PRESENCE only — absence is NEVER returned (an owner
// "none" attestation yields hasInfo:false, NOT a "free-from" claim). The known set is a CONSERVATIVE UNION of
// the owner's L3 declaration (status 'listed') and any recipe-derived allergens, so a base-dish allergen
// warning can never be dropped by attestation status (#4-positive). hasInfo:false ⇒ the caller renders the
// floor ("allergen info not provided"), NEVER a blank — data-absence must never read as a clean state.
export interface AllergenSurface {
  known: string[];
  hasInfo: boolean;
}
export function computeAllergenSurface(
  attributes: { allergen_status?: string; declared_allergens?: unknown } | null | undefined,
  bomAllergens: readonly string[] = [],
): AllergenSurface {
  const a = attributes || {};
  const declared =
    a.allergen_status === 'listed' && Array.isArray(a.declared_allergens)
      ? (a.declared_allergens as unknown[]).map((x) => String(x))
      : [];
  const known = Array.from(new Set([...declared, ...bomAllergens.map((x) => String(x))]));
  return { known, hasInfo: known.length > 0 };
}

// ── COMPARISON of exactly two dishes (council §5 + guardrail #8) ─────────────────────────────────────────
// Reuses the same characteristics layer. Directional "lower wins" markers are emitted ONLY on the
// NON-regulated axes (price, prep-time) — NEVER on macros (a kcal "wins" arrow is a regulated lightness
// verdict, R2-H1) and NEVER a global winner (deltas are neutral; the customer's priority decides). Taste is
// side-by-side, not a winner. Allergens: BOTH dishes' surfaces are ALWAYS returned explicitly (#8) — a
// no-data dish yields hasInfo:false (the caller renders the floor), NEVER a blank that reads "free-from".
export interface CompareDishInput {
  id: string;
  name: string;
  price: number;
  prepTimeMinutes?: number | null;
  taste?: Record<string, number> | null;
  attributes?: { allergen_status?: string; declared_allergens?: unknown } | null;
  bomAllergens?: readonly string[];
}
export interface CompareAxis {
  a: number | null;
  b: number | null;
  lower: 'a' | 'b' | 'tie' | null; // null when either side is missing — no fabricated winner
}
export interface DishComparison {
  price: CompareAxis;
  prepTime: CompareAxis;
  taste: { a: Record<string, number>; b: Record<string, number> };
  allergens: { a: AllergenSurface; b: AllergenSurface };
}
function lowerWinsAxis(a: number | null | undefined, b: number | null | undefined): CompareAxis {
  const av = typeof a === 'number' ? a : null;
  const bv = typeof b === 'number' ? b : null;
  let lower: CompareAxis['lower'] = null;
  if (av != null && bv != null) lower = av < bv ? 'a' : bv < av ? 'b' : 'tie';
  return { a: av, b: bv, lower };
}
export function compareDishes(a: CompareDishInput, b: CompareDishInput): DishComparison {
  return {
    price: lowerWinsAxis(a.price, b.price), // lower = cheaper (a fact, not a verdict)
    prepTime: lowerWinsAxis(a.prepTimeMinutes ?? null, b.prepTimeMinutes ?? null), // lower = faster
    taste: { a: a.taste || {}, b: b.taste || {} }, // side-by-side, NO winner
    allergens: {
      a: computeAllergenSurface(a.attributes, a.bomAllergens || []),
      b: computeAllergenSurface(b.attributes, b.bomAllergens || []),
    },
  };
}

// ── MACRO FILTER/SORT LENS (council §8.3 + guardrail #15) ────────────────────────────────────────────────
// A no-data dish is NOT a 0-value dish. Sorting by a macro must place dishes WITHOUT nutrition data in an
// explicit "no data" bucket, NEVER inline at the bottom of the numeric rank (where a no-bom protein:0 would
// masquerade as "lowest protein"). The lens checks DATA PRESENCE (hasData), not the numeric 0 — a real
// kcal:0 / protein:0 dish that HAS data stays ranked; only data-absent dishes drop to the bucket.
// Sort over RAW kcal/protein is a neutral fact; a "light"/"healthy" verdict is the regulated subset (deferred).
export type MacroLens = 'kcal-asc' | 'kcal-desc' | 'protein-asc' | 'protein-desc';
export interface MacroLensItem {
  hasData: boolean; // true iff the dish has a bom (nutrition is known) — independent of the numeric value
  kcal: number;
  protein: number;
}
export function partitionByMacroLens<T extends MacroLensItem>(items: readonly T[], lens: MacroLens): { ranked: T[]; noData: T[] } {
  const noData = items.filter((i) => !i.hasData);
  const withData = items.filter((i) => i.hasData);
  const [axis, dir] = lens.split('-') as ['kcal' | 'protein', 'asc' | 'desc'];
  const val = (i: T) => (axis === 'kcal' ? i.kcal : i.protein);
  withData.sort((a, b) => (dir === 'asc' ? val(a) - val(b) : val(b) - val(a)));
  return { ranked: withData, noData };
}
