/* Shared Live Dashboard JS — WS client, API helpers, PII masking */

// ─── Token Management ───────────────────────────────────────────────
function getOwnerToken() {
  const params = new URLSearchParams(location.search);
  if (params.has('token')) return params.get('token');
  return localStorage.getItem('dos_owner_token');
}

function getLocationId() {
  const params = new URLSearchParams(location.search);
  if (params.has('locationId')) return params.get('locationId');
  return localStorage.getItem('dos_owner_locationId');
}

// ─── PII Masking ────────────────────────────────────────────────────
function maskName(name) {
  if (!name) return '***';
  return name.charAt(0) + '***';
}

function maskPhone(phone) {
  if (!phone) return '***';
  const cleaned = phone.replace(/\D/g, '');
  if (cleaned.length < 4) return '+*** *** ****';
  return '+*** *** ' + cleaned.substring(cleaned.length - 4);
}

// ─── WS Client ──────────────────────────────────────────────────────
class DashboardWSClient {
  constructor(opts) {
    this.url = opts.url;
    this.token = opts.token;
    this.locationId = opts.locationId;
    this.onEvent = opts.onEvent || (() => {});
    this.onStatusChange = opts.onStatusChange || (() => {});
    this.ws = null;
    this.reconnectAttempt = 0;
    this.maxReconnect = 30;
    this.intentionalClose = false;
    this.connect();
  }

  connect() {
    try {
      this.ws = new WebSocket(this.url);
    } catch (e) {
      this.scheduleReconnect();
      return;
    }

    this.ws.onopen = () => {
      this.reconnectAttempt = 0;
      this.onStatusChange('connecting');
      this.ws.send(JSON.stringify({ type: 'auth', token: this.token }));
    };

    this.ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data);
        if (msg.type === 'auth_success') {
          this.onStatusChange('connected');
          this.ws.send(JSON.stringify({ type: 'subscribe', room: `location:${this.locationId}:dashboard` }));
        } else if (msg.type === 'subscribed') {
          this.onStatusChange('subscribed');
        } else if (msg.room && msg.data) {
          this.onEvent(msg.data);
        } else {
          this.onEvent(msg);
        }
      } catch { /* ignore parse errors */ }
    };

    this.ws.onclose = () => {
      this.onStatusChange('disconnected');
      if (!this.intentionalClose) this.scheduleReconnect();
    };

    this.ws.onerror = () => {
      this.ws?.close();
    };
  }

  scheduleReconnect() {
    if (this.reconnectAttempt >= 6) {
      this.onStatusChange('failed');
      return;
    }
    const base = Math.min(1000 * Math.pow(2, this.reconnectAttempt), 30000);
    const jitter = base * (0.5 + Math.random());
    this.reconnectAttempt++;
    this.onStatusChange('reconnecting');
    setTimeout(() => this.connect(), jitter);
  }

  close() {
    this.intentionalClose = true;
    this.ws?.close();
  }
}

// ─── API Helpers ────────────────────────────────────────────────────
async function fetchSnapshot(locationId, token, opts = {}) {
  const params = new URLSearchParams();
  if (opts.status) params.set('status', opts.status);
  if (opts.limit) params.set('limit', String(opts.limit));
  if (opts.cursor) params.set('cursor', opts.cursor);
  const qs = params.toString();
  const url = `/api/owner/locations/${locationId}/dashboard/snapshot${qs ? '?' + qs : ''}`;
  const res = await fetch(url, { headers: { Authorization: `Bearer ${token}` } });
  if (!res.ok) throw new Error(`Snapshot fetch failed: ${res.status}`);
  return res.json();
}

async function confirmOrder(locationId, orderId, token) {
  const res = await fetch(`/api/owner/locations/${locationId}/orders/${orderId}/confirm`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: '{}',
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw { status: res.status, ...err };
  }
  return res.json();
}

async function rejectOrder(locationId, orderId, reason, token) {
  const res = await fetch(`/api/owner/locations/${locationId}/orders/${orderId}/reject`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ reason: reason || 'Owner rejected' }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw { status: res.status, ...err };
  }
  return res.json();
}

async function assignCourier(locationId, orderId, courierId, token) {
  const res = await fetch(`/api/owner/locations/${locationId}/orders/${orderId}/assign-courier`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ courierId }),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw { status: res.status, ...err };
  }
  return res.json();
}

// ─── Dead Channel Detection ─────────────────────────────────────────
async function checkDeadChannels(locationId, token) {
  try {
    const res = await fetch(`/api/owner/locations/${locationId}/degradation`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!res.ok) return;
    const data = await res.json();
    const banner = document.getElementById('deadChannelBanner');
    if (!banner) return;
    if (data.deadChannels && data.deadChannels.length > 0) {
      banner.classList.remove('hidden');
      document.getElementById('deadChannelText').textContent =
        `Channels offline: ${data.deadChannels.join(', ')}. Notifications may not reach you.`;
    } else {
      banner.classList.add('hidden');
    }
  } catch { /* ignore */ }
}

async function reEnableChannel(locationId, channel, targetId, token) {
  try {
    const res = await fetch(`/api/owner/locations/${locationId}/notifications/targets/${targetId}`, {
      method: 'PUT',
      headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'active' }),
    });
    if (res.ok) {
      checkDeadChannels(locationId, token);
    }
  } catch { /* ignore */ }
}

// ─── Dwell Timer ────────────────────────────────────────────────────
function startDwellTimers(containerEl) {
  return setInterval(() => {
    const els = containerEl.querySelectorAll('[data-created-at]');
    for (const el of els) {
      const createdAt = new Date(el.dataset.createdAt).getTime();
      const dwellSec = Math.floor((Date.now() - createdAt) / 1000);
      el.textContent = formatDwell(dwellSec);
    }
  }, 1000);
}

function formatDwell(sec) {
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function getDwellColor(sec, thresholds) {
  if (!thresholds || !thresholds.pending_s) return '';
  const pct = sec / thresholds.pending_s;
  if (pct >= 1) return 'text-red-500 font-bold';
  if (pct >= 0.5) return 'text-yellow-500';
  return 'text-green-500';
}
