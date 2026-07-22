const WEBHOOK_URL = '/api/telemetry/web';

export function createTelegramBridge() {
  let queue = [];

  function send(data) {
    try {
      const payload = { source: 'web', ts: Date.now(), ...data };
      queue.push(payload);
      if (queue.length >= 5) flush();
      else setTimeout(flush, 30000);
    } catch {}
  }

  async function flush() {
    if (queue.length === 0) return;
    const batch = queue.splice(0);
    try {
      await fetch(WEBHOOK_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ events: batch }),
        keepalive: true,
      });
    } catch {}
  }

  window.addEventListener('beforeunload', () => flush());

  return {
    alert(type, message, meta) {
      send({ type: 'alert', level: type, message, meta });
    },
    vitalsReport(report) {
      send({ type: 'vitals', report });
    },
    checkoutEvent(success, duration, orderId) {
      send({ type: 'checkout', success, duration, orderId });
    },
    errorEvent(context, error) {
      send({ type: 'error', context, message: error?.message || String(error) });
    },
    flush,
  };
}
