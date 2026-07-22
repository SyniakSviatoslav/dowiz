export function sanitize(str) {
  const div = typeof document !== 'undefined' ? document.createElement('div') : null;
  if (div) { div.textContent = str || ''; return div.innerHTML; }
  return String(str || '').replace(/[<>&"']/g, c => ({ '<':'&lt;','>':'&gt;','&':'&amp;','"':'&quot;',"'":'&#39;' })[c]);
}

export function rateLimit(fn, ms = 3000) {
  let last = 0;
  return function (...args) {
    const now = Date.now();
    if (now - last < ms) { return; }
    last = now;
    return fn.apply(this, args);
  };
}

export function formatETA(minutes) {
  if (minutes < 1) return 'зараз';
  if (minutes < 60) return `~${minutes} хв`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return `~${h} год ${m} хв`;
}

export function estimateETA(order) {
  if (!order || order.status === 'delivered' || order.status === 'cancelled') return null;
  const etaByStatus = { pending: '~5 хв', confirmed:'~10 хв', preparing:'~15 хв', ready:'~5 хв', 'in-delivery':'~15-20 хв' };
  return etaByStatus[order.status] || null;
}

export function validatePhone(v) { return /^[\+\d\s\-\(\)]{7,20}$/.test(v.trim()); }

export function validateAddress(v) { return v.trim().length >= 5; }

export function generateId() { return Date.now() + Math.floor(Math.random() * 1000); }

export function randomItems(count) { return 1 + Math.floor(Math.random() * count); }

export function randomTotal() { return (5 + Math.floor(Math.random() * 30)) * 100; }
