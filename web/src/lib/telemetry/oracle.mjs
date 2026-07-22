const KEY = 'dowiz-telemetry';

export function createOracle() {
  let marks = {};
  let interactions = [];

  function load() {
    try {
      const raw = localStorage.getItem(KEY);
      if (raw) marks = JSON.parse(raw);
    } catch {}
  }

  function save() {
    try { localStorage.setItem(KEY, JSON.stringify(marks)); } catch {}
  }

  load();

  return {
    mark(name) {
      performance.mark(name);
      marks[name] = marks[name] || [];
      marks[name].push(performance.now());
      save();
    },

    measure(name, startMark, endMark) {
      performance.measure(name, startMark, endMark);
    },

    trackInteraction(label, fn) {
      const start = performance.now();
      performance.mark(`${label}-start`);
      interactions.push({ label, start });
      let result;
      try { result = fn(); } catch (e) { result = e; }
      const dur = performance.now() - start;
      performance.mark(`${label}-end`);
      performance.measure(label, `${label}-start`, `${label}-end`);
      marks[label] = marks[label] || [];
      marks[label].push(dur);
      save();
      return result;
    },

    async trackInteractionAsync(label, fn) {
      const start = performance.now();
      performance.mark(`${label}-start`);
      let result;
      try { result = await fn(); } catch (e) { result = e; }
      const dur = performance.now() - start;
      performance.mark(`${label}-end`);
      performance.measure(label, `${label}-start`, `${label}-end`);
      marks[label] = marks[label] || [];
      marks[label].push(dur);
      save();
      return result;
    },

    observeVitals() {
      if ('webVitals' in self) return;
      try {
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            marks[entry.name] = marks[entry.name] || [];
            marks[entry.name].push(entry.startTime);
            save();
          }
        }).observe({ type: 'largest-contentful-paint', buffered: true });
      } catch {}

      try {
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            marks['CLS'] = marks['CLS'] || [];
            marks['CLS'].push(entry.value);
            save();
          }
        }).observe({ type: 'layout-shift', buffered: true });
      } catch {}
    },

    getSummary() {
      const summary = {};
      for (const [key, vals] of Object.entries(marks)) {
        if (vals.length > 0) {
          summary[key] = {
            count: vals.length,
            avg: vals.reduce((a, b) => a + b, 0) / vals.length,
            min: Math.min(...vals),
            max: Math.max(...vals),
            last: vals[vals.length - 1],
          };
        }
      }
      return summary;
    },

    getReport() {
      return { marks, interactions };
    },
  };
}
