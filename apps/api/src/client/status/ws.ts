export class StatusWSClient {
  private ws: WebSocket | null = null;
  private orderId: string;
  private token: string;
  private reconnectAttempts = 0;
  private maxAttempts = 5;
  private reconnecting = false;

  constructor(orderId: string, token: string) {
    this.orderId = orderId;
    this.token = token;
  }

  connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws/orders/${this.orderId}?token=${this.token}`;
    
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      console.log('WS connected');
      this.reconnectAttempts = 0;
      if (this.reconnecting) {
        this.reconnecting = false;
        this.reconcile();
      }
    };

    this.ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data);
        window.dispatchEvent(new CustomEvent('status:update', { detail: payload }));
      } catch (e) {
        console.error('Invalid WS message', e);
      }
    };

    this.ws.onclose = (event) => {
      if (event.code === 1008) {
        console.error('Auth failed on WS');
        return; // Don't reconnect on auth failure
      }
      this.handleReconnect();
    };

    this.ws.onerror = (err) => {
      console.error('WS Error', err);
      // onclose will trigger reconnect
    };
  }

  private handleReconnect() {
    if (this.reconnectAttempts >= this.maxAttempts) {
      window.dispatchEvent(new CustomEvent('status:offline'));
      window.dispatchEvent(new CustomEvent('fallback:needed', { detail: { reason: 'ws_offline' } }));
      return;
    }

    this.reconnecting = true;
    const baseDelay = Math.pow(2, this.reconnectAttempts) * 1000;
    const jitter = 0.5 + Math.random();
    const delay = Math.min(baseDelay * jitter, 30000);

    this.reconnectAttempts++;
    console.log(`Reconnecting in ${Math.round(delay)}ms (Attempt ${this.reconnectAttempts})`);
    
    setTimeout(() => {
      this.connect();
    }, delay);
  }

  private async reconcile() {
    try {
      const res = await fetch(`/api/orders/${this.orderId}`, {
        headers: { 'Authorization': `Bearer ${this.token}` }
      });
      if (res.ok) {
        const order = await res.json();
        window.dispatchEvent(new CustomEvent('status:reconcile', { detail: order }));
      }
    } catch (e) {
      console.error('Reconcile failed', e);
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
    }
  }
}
