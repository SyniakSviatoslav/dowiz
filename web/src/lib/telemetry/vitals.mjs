const KEY = 'dowiz-vitals';

export function observeVitals() {
  let vitals = {};

  function load() {
    try { const raw = localStorage.getItem(KEY); if (raw) vitals = JSON.parse(raw); } catch {}
  }
  function save() {
    try { localStorage.setItem(KEY, JSON.stringify(vitals)); } catch {}
  }

  load();

  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        vitals.LCP = entry.startTime;
        save();
      }
    }).observe({ type: 'largest-contentful-paint', buffered: true });
  } catch {}

  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        vitals.FCP = entry.startTime;
        save();
      }
    }).observe({ type: 'paint', buffered: true });
  } catch {}

  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        vitals.CLS = entry.value;
        save();
      }
    }).observe({ type: 'layout-shift', buffered: true });
  } catch {}

  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        vitals.INP = entry.processingStart - entry.startTime;
        save();
      }
    }).observe({ type: 'first-input', buffered: true });
  } catch {}

  vitals.TTFB = performance.timing ? performance.timing.responseStart - performance.timing.fetchStart : 0;

  return {
    get vitals() { return { ...vitals }; },
    report() {
      const thresholds = { FCP: [1800, 3000], LCP: [2500, 4000], CLS: [0.1, 0.25], INP: [200, 500], TTFB: [800, 1800] };
      const r = {};
      for (const [k, v] of Object.entries(vitals)) {
        const t = thresholds[k];
        if (t && v !== undefined) {
          r[k] = { value: v, rating: v < t[0] ? 'good' : v < t[1] ? 'needs-improvement' : 'poor' };
        }
      }
      return r;
    },
    isHealthy() {
      const r = this.report();
      for (const v of Object.values(r)) { if (v.rating === 'poor') return false; }
      return Object.keys(r).length >= 3;
    },
  };
}
