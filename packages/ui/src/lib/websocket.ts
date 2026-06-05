export type WsStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting';

export interface WsMessage {
  type: string;
  payload: unknown;
  timestamp: string;
}

type StatusListener = (status: WsStatus) => void;
type MessageHandler = (msg: WsMessage) => void;
type ReconcileHandler = (data: unknown) => void;

function expBackoffWithJitter(attempt: number): number {
  const base = Math.min(1000 * Math.pow(2, attempt), 30000);
  const jitter = Math.random() * 1000;
  return Math.floor(base + jitter);
}

export class WsClient {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string | null = null;
  private statusListeners: Set<StatusListener> = new Set();
  private messageHandlers: Map<string, Set<MessageHandler>> = new Map();
  private reconcileHandlers: Set<ReconcileHandler> = new Set();
  private reconnectAttempt = 0;
  private maxReconnectAttempts = 10;
  private shouldReconnect = true;
  private subscribedRooms: Set<string> = new Set();

  constructor(url: string, token?: string) {
    this.url = url;
    this.token = token ?? null;
  }

  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) return;
    this.shouldReconnect = true;
    this.reconnectAttempt = 0;
    this.emitStatus('connecting');
    this.createConnection();
  }

  disconnect(): void {
    this.shouldReconnect = false;
    this.ws?.close();
    this.ws = null;
    this.emitStatus('disconnected');
  }

  setToken(token: string): void {
    this.token = token;
  }

  subscribe(room: string): void {
    this.subscribedRooms.add(room);
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.send({ type: 'subscribe', payload: { room } });
    }
  }

  unsubscribe(room: string): void {
    this.subscribedRooms.delete(room);
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.send({ type: 'unsubscribe', payload: { room } });
    }
  }

  onMessage(type: string, handler: MessageHandler): () => void {
    if (!this.messageHandlers.has(type)) {
      this.messageHandlers.set(type, new Set());
    }
    this.messageHandlers.get(type)!.add(handler);
    return () => this.messageHandlers.get(type)?.delete(handler);
  }

  onReconcile(handler: ReconcileHandler): () => void {
    this.reconcileHandlers.add(handler);
    return () => this.reconcileHandlers.delete(handler);
  }

  onStatus(handler: StatusListener): () => void {
    this.statusListeners.add(handler);
    return () => this.statusListeners.delete(handler);
  }

  private createConnection(): void {
    const wsUrl = this.token ? `${this.url}?token=${encodeURIComponent(this.token)}` : this.url;
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      this.reconnectAttempt = 0;
      this.emitStatus('connected');
      // Re-subscribe to rooms
      this.subscribedRooms.forEach((room) => {
        this.send({ type: 'subscribe', payload: { room } });
      });
    };

    this.ws.onclose = () => {
      this.emitStatus('disconnected');
      if (this.shouldReconnect && this.reconnectAttempt < this.maxReconnectAttempts) {
        this.reconnectAttempt++;
        this.emitStatus('reconnecting');
        setTimeout(() => this.createConnection(), expBackoffWithJitter(this.reconnectAttempt));
      }
    };

    this.ws.onerror = () => {
      this.ws?.close();
    };

    this.ws.onmessage = (event) => {
      try {
        const msg: WsMessage = JSON.parse(event.data);
        this.messageHandlers.get(msg.type)?.forEach((h) => h(msg));
        if (msg.type === 'reconcile') {
          this.reconcileHandlers.forEach((h) => h(msg.payload));
        }
      } catch {
        // Ignore malformed messages
      }
    };
  }

  private send(msg: { type: string; payload: unknown }): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  private emitStatus(status: WsStatus): void {
    this.statusListeners.forEach((h) => h(status));
  }
}
