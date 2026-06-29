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
