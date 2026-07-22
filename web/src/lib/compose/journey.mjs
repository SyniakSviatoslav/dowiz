// web/src/lib/compose/journey.mjs — customer journey FSM (Stage A)
// Mirrors kernel/src/storefront.rs JourneyStep. Pure state machine — zero I/O.

export const Step = Object.freeze({
  STOREFRONT: 'storefront',
  MENU: 'menu',
  DETAIL: 'detail',
  CART: 'cart',
  FULFILLMENT: 'fulfillment',
  PAYMENT: 'payment',
  PLACED: 'placed',
  SUSPENDED: 'suspended',
});

const ARC = [
  Step.STOREFRONT, Step.MENU, Step.DETAIL, Step.CART,
  Step.FULFILLMENT, Step.PAYMENT, Step.PLACED,
];

export function createJourney(initial = Step.STOREFRONT) {
  let step = initial;
  let history = [step];
  return {
    get current() { return step; },
    get canAdvance() {
      const idx = ARC.indexOf(step);
      return idx >= 0 && idx < ARC.length - 1;
    },
    advance() {
      const idx = ARC.indexOf(step);
      if (idx < 0 || idx >= ARC.length - 1) return false;
      step = ARC[idx + 1];
      history.push(step);
      return true;
    },
    retreat() {
      const idx = ARC.indexOf(step);
      if (idx <= 0) return false;
      step = ARC[idx - 1];
      history.push(step);
      return true;
    },
    reset(to = Step.STOREFRONT) {
      step = to;
      history = [step];
    },
    isAfter(s) { return ARC.indexOf(step) > ARC.indexOf(s); },
    isBefore(s) { return ARC.indexOf(step) < ARC.indexOf(s); },
    history() { return [...history]; },
  };
}
