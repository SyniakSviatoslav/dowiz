// Mirrors messages/en.json — kept as a plain TS object here only because the stand-in runtime
// (paraglide-stub.ts) can't `import ... from '*.json'` inside a Svelte island without a bundler
// JSON-module setting we haven't installed yet. Real Paraglide compiles messages/en.json directly.
export const messages = {
  client_menu: 'Menu',
  cart_title: 'Cart',
  cart_empty: 'Cart is empty',
  cart_total: 'Total',
  cart_checkout: 'Checkout',
  cart_clear: 'Clear',
  cart_increase: 'Increase quantity',
  cart_decrease: 'Decrease quantity',
  checkout_title: 'Checkout',
  client_closed_title: 'Closed right now',
} as const;
