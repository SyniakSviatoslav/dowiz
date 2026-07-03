// Server-side mirror of the storefront's open/closed computation.
//
// SOURCE OF TRUTH: the isOpen derivation is INLINE in
// apps/api/src/routes/public/menu.ts (the `/public/locations/:slug/info` handler, ~lines
// 335-358). This helper reproduces THAT logic EXACTLY so the order gate AGREES with what the
// customer was shown on the storefront — same SERVER-LOCAL-time semantics (Date#getDay /
// getHours / getMinutes), same hours_json shape, same delivery_paused handling, same
// treat-parse-errors-as-"leave isOpen alone" catch. Do NOT "fix" the timezone or change
// semantics here: a divergence would let the customer see "open" while the server rejects
// (lost orders), which is worse than the gap it closes. If menu.ts changes, change this in
// lockstep. menu.ts is a top code-churn hotspot, so the duplication is deliberate.

const DAYS = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'] as const;

/**
 * Pure mirror of menu.ts:335-358. `now` is injected (menu.ts uses `new Date()` inline) so this
 * is deterministic and testable; the semantics are otherwise byte-identical.
 *
 * Note: menu.ts also assigns `closesAt` inside the `open && close` branch — that is a UI-only
 * side effect with no bearing on the boolean, so it is intentionally omitted here.
 */
export function isVenueOpen(hoursJson: unknown, deliveryPaused: boolean, now: Date): boolean {
  let isOpen = !(deliveryPaused ?? false);
  if (hoursJson) {
    try {
      const dayName = DAYS[now.getDay()] as string;
      const dayData = (hoursJson as Record<string, any>)[dayName];
      if (dayData && typeof dayData === 'object') {
        if (dayData.isOpen === false) {
          isOpen = false;
        } else if (dayData.open && dayData.close) {
          if (isOpen) {
            const [oh, om] = dayData.open.split(':').map(Number);
            const [ch, cm] = dayData.close.split(':').map(Number);
            const nowMins = now.getHours() * 60 + now.getMinutes();
            const openMins = oh * 60 + om;
            const closeMins = ch * 60 + cm;
            isOpen = nowMins >= openMins && nowMins < closeMins;
          }
        }
      }
    } catch { /* ignore parse errors — leaves isOpen at its delivery_paused-derived value, matching menu.ts */ }
  }
  return isOpen;
}
