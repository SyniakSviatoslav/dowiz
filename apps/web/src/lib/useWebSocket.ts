import { useEffect, useRef, useState, useCallback } from 'react';

const protocol = typeof window !== 'undefined' && window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const host = typeof window !== 'undefined' ? window.location.host : 'localhost:3000';
const WS_BASE_URL = import.meta.env?.VITE_WS_BASE_URL || `${protocol}//${host}/ws`;

export type WSConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error' | 'disabled';

interface UseWebSocketOptions {
  room?: string;
  onMessage?: (data: any) => void;
  onReconnect?: () => void;
  enabled?: boolean;
}

export function useWebSocket({ room, onMessage, onReconnect, enabled = true }: UseWebSocketOptions = {}) {
  const [status, setStatus] = useState<WSConnectionStatus>(enabled ? 'disconnected' : 'disabled');
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttempts = useRef(0);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const initialBackoff = 2000;
  const maxBackoff = 15000;

  const onMessageRef = useRef(onMessage);
  const onReconnectRef = useRef(onReconnect);
  onMessageRef.current = onMessage;
  onReconnectRef.current = onReconnect;

  const connect = useCallback(() => {
    if (!enabled) {
      setStatus('disabled');
      return;
    }
    if (!mountedRef.current) return;
    if (wsRef.current?.readyState === WebSocket.OPEN) return;
    if (wsRef.current?.readyState === WebSocket.CONNECTING) return;

    if (reconnectTimer.current) {
      clearTimeout(reconnectTimer.current);
      reconnectTimer.current = null;
    }

    setStatus(reconnectAttempts.current > 0 ? 'reconnecting' : 'connecting');

    try {
      const token = typeof window !== 'undefined' ? localStorage.getItem('dos_access_token') : null;
      const url = new URL(WS_BASE_URL);
      if (token) url.searchParams.set('token', token);

      const ws = new WebSocket(url.toString());
      wsRef.current = ws;

      ws.onopen = () => {
        if (!mountedRef.current) { ws.close(1000); return; }
        setStatus('connected');

        const token = typeof window !== 'undefined' ? localStorage.getItem('dos_access_token') : null;
        if (token) {
          ws.send(JSON.stringify({ type: 'auth', token }));
        } else if (room) {
          ws.send(JSON.stringify({ type: 'subscribe', room }));
        }
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data.type === 'auth_success') {
            reconnectAttempts.current = 0;
            if (room) {
              ws.send(JSON.stringify({ type: 'subscribe', room }));
            }
            onReconnectRef.current?.();
            return;
          }
          onMessageRef.current?.(data);
        } catch (err) {
          console.warn('[useWebSocket] received malformed message:', err);
        }
      };

      ws.onerror = () => {};

      ws.onclose = (event) => {
        wsRef.current = null;
        if (!mountedRef.current) return;
        if (event.code === 1000 || event.code === 1005) {
          setStatus('disconnected');
          return;
        }

        // Reconnect FOREVER with capped backoff. Giving up permanently after N
        // tries (the old behaviour) left owners/couriers on a silently-stale
        // dashboard after any brief outage (deploy, machine restart, network
        // blip) until they manually reloaded — order updates just stopped
        // arriving. The backoff caps at maxBackoff, so this settles into a steady
        // ~15s retry rather than hammering the server.
        setStatus('reconnecting');
        const backoff = Math.min(initialBackoff * Math.pow(1.5, reconnectAttempts.current), maxBackoff);
        const jitter = Math.random() * 1000;
        reconnectTimer.current = setTimeout(() => {
          if (mountedRef.current) {
            reconnectAttempts.current += 1;
            connect();
          }
        }, backoff + jitter);
      };
    } catch (err) {
      console.warn('[useWebSocket] connect failed:', err);
      setStatus('error');
    }
  }, [enabled, room]);

  useEffect(() => {
    mountedRef.current = true;
    if (enabled) {
      connect();
    } else {
      setStatus('disabled');
    }

    // Reconnect promptly when the network comes back or the tab is refocused,
    // instead of waiting out the backoff. Reset the attempt counter so the next
    // connect uses the shortest delay. connect() no-ops if a socket is already
    // open/connecting, so this is safe to fire liberally.
    const resume = () => {
      if (!enabled || !mountedRef.current) return;
      if (typeof document !== 'undefined' && document.visibilityState === 'hidden') return;
      const ws = wsRef.current;
      if (!ws || ws.readyState === WebSocket.CLOSED || ws.readyState === WebSocket.CLOSING) {
        reconnectAttempts.current = 0;
        connect();
      }
    };
    if (typeof window !== 'undefined') {
      window.addEventListener('online', resume);
      window.addEventListener('focus', resume);
      document.addEventListener('visibilitychange', resume);
    }

    return () => {
      mountedRef.current = false;
      if (typeof window !== 'undefined') {
        window.removeEventListener('online', resume);
        window.removeEventListener('focus', resume);
        document.removeEventListener('visibilitychange', resume);
      }
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
      if (wsRef.current) {
        if (room && wsRef.current.readyState === WebSocket.OPEN) {
          try { wsRef.current.send(JSON.stringify({ type: 'unsubscribe', room })); } catch (err) {
            console.debug('[useWebSocket] unsubscribe send failed:', err);
          }
        }
        wsRef.current.close(1000, 'unmount');
        wsRef.current = null;
      }
    };
  }, [connect, enabled, room]);

  const sendMessage = useCallback((message: any) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    }
  }, []);

  return { status, sendMessage };
}
