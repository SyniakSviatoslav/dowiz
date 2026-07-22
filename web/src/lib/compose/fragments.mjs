// web/src/lib/compose/fragments.mjs — fragment vocabulary (Stage A)
// Mirrors engine/src/compose_ui.rs fragment functions. Pure fn(&AppState) -> Vec<Shape>.

import { CHEF_PICKS } from './compose.mjs';

export const VENDOR_ITEMS = CHEF_PICKS;

export function menuFragment(state) {
  const cols = 4, pitchX = 2.4, pitchY = 1.7, hw = 1.0, hh = 0.7, r = 0.18;
  const shapes = [];
  const source = state._menu && state._menu.length ? state._menu : VENDOR_ITEMS;
  const items = state.filter === 'all' ? source : source.filter(i => i.cat === state.filter);
  items.forEach((item, k) => {
    const col = k % cols, row = Math.floor(k / cols);
    const cx = -(Math.min(items.length, cols)) * pitchX * 0.5 + pitchX * 0.5 + col * pitchX;
    const cy = 1.0 - row * pitchY;
    shapes.push({ t: 'rbox', bx: cx, by: cy, hx: hw, hy: hh, r });
    if (!item.drink) {
      shapes.push({ t: 'line', ax: cx - hw * 0.6, ay: cy - hh - 0.05, bx: cx + hw * 0.6, by: cy - hh - 0.05 });
    }
  });
  const cartCount = (state.cart || []).reduce((s, i) => s + i.qty, 0);
  if (cartCount > 0) shapes.push({ t: 'circ', cx: 3.2, cy: 1.6, r: 0.35 });
  return shapes;
}

export function cartFragment(state) {
  const shapes = [];
  const items = state.cart || [];
  const count = items.reduce((s, i) => s + i.qty, 0);
  if (count === 0) {
    shapes.push({ t: 'rbox', bx: 0, by: 0, hx: 2.0, hy: 0.6, r: 0.1 });
    return shapes;
  }
  items.forEach((item, k) => {
    const cy = 1.0 - k * 0.6;
    shapes.push({ t: 'rbox', bx: 0, by: cy, hx: 2.0, hy: 0.25, r: 0.05 });
  });
  const total = items.reduce((s, i) => s + i.price * i.qty, 0);
  if (total > 0) {
    shapes.push({ t: 'line', ax: -1.5, ay: -0.8, bx: 1.5, by: -0.8 });
  }
  return shapes;
}

export function checkoutFragment(state) {
  const shapes = cartFragment(state);
  shapes.push({ t: 'rbox', bx: 0, by: -1.4, hx: 1.2, hy: 0.35, r: 0.12 });
  return shapes;
}

export function ownerDashboardFragment(state) {
  const shapes = [];
  const tiles = [
    { label: 'Orders', value: state._stats?.orders || 0 },
    { label: 'Revenue', value: state._stats?.revenue || 0 },
    { label: 'Active', value: state._stats?.active || 0 },
  ];
  tiles.forEach((tile, k) => {
    const cx = -2.0 + k * 2.0, cy = 0.5;
    shapes.push({ t: 'rbox', bx: cx, by: cy, hx: 0.7, hy: 0.5, r: 0.08 });
  });
  const recentOrders = state._orders || [];
  recentOrders.slice(0, 5).forEach((o, k) => {
    const cy = -0.8 - k * 0.5;
    shapes.push({ t: 'rbox', bx: 0, by: cy, hx: 3.0, hy: 0.2, r: 0.04 });
  });
  return shapes;
}

export function courierBoardFragment(state) {
  const shapes = [];
  const earnings = state._earningsToday || 0;
  const deliveries = state._deliveriesToday || 0;
  const shiftActive = state._shiftActive || false;
  const tasks = state._courierTasks || [];

  // Shift status indicator
  if (shiftActive) {
    shapes.push({ t: 'circ', cx: -2.5, cy: 1.5, r: 0.12 });
  }

  // Earnings badge
  shapes.push({ t: 'rbox', bx: 0, by: 1.2, hx: 1.2, hy: 0.35, r: 0.1 });
  shapes.push({ t: 'line', ax: -1.5, ay: 0.7, bx: 1.5, by: 0.7 });

  // Task cards
  tasks.slice(0, 4).forEach((t, k) => {
    const cy = 0.3 - k * 0.55;
    shapes.push({ t: 'rbox', bx: 0, by: cy, hx: 2.5, hy: 0.22, r: 0.05 });
    if (t.status === 'picked-up') {
      shapes.push({ t: 'circ', cx: 1.2, cy, r: 0.06 });
    }
  });

  // Bottom summary
  shapes.push({ t: 'line', ax: -1.0, ay: -2.0, bx: 1.0, by: -2.0 });
  return shapes;
}

export function confirmWellFragment(state) {
  const shapes = [];
  shapes.push({ t: 'rbox', bx: 0, by: 0.3, hx: 1.5, hy: 0.5, r: 0.1 });
  shapes.push({ t: 'rbox', bx: 0, by: -0.4, hx: 0.8, hy: 0.3, r: 0.08 });
  return shapes;
}

export function sceneForRole(state) {
  const fragments = {
    menu: menuFragment,
    orders: cartFragment,
    analytics: ownerDashboardFragment,
    owner: ownerDashboardFragment,
    courier: courierBoardFragment,
    checkout: checkoutFragment,
  };
  const fn = fragments[state.page] || fragments.menu;
  return fn(state);
}
