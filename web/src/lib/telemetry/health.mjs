const KEY = 'dowiz-health';

export function createHealthMonitor() {
  let log = [];

  function load() {
    try { const raw = localStorage.getItem(KEY); if (raw) log = JSON.parse(raw); } catch {}
  }
  function save() {
    try { localStorage.setItem(KEY, JSON.stringify(log.slice(-200))); } catch {}
  }

  load();

  return {
    signal(type, data) {
      const entry = { t: Date.now(), type, data };
      log.push(entry);
      save();
      return entry;
    },

    signalError(context, error) {
      return this.signal('error', { context, message: error?.message || String(error), stack: error?.stack?.slice(0, 200) });
    },

    signalLatency(context, ms) {
      return this.signal('latency', { context, ms });
    },

    signalCheckout(success, duration, orderId) {
      return this.signal('checkout', { success, duration, orderId });
    },

    recent(count = 20) {
      return log.slice(-count);
    },

    errors() {
      return log.filter(e => e.type === 'error');
    },

    summary() {
      const total = log.length;
      const errors = log.filter(e => e.type === 'error').length;
      const latencies = log.filter(e => e.type === 'latency');
      const avgLatency = latencies.length > 0 ? latencies.reduce((s, e) => s + e.data.ms, 0) / latencies.length : 0;
      const checkouts = log.filter(e => e.type === 'checkout');
      const successRate = checkouts.length > 0 ? checkouts.filter(e => e.data.success).length / checkouts.length : 1;
      return { total, errors, avgLatency, checkouts: checkouts.length, successRate };
    },

    get log() { return log; },
  };
}
