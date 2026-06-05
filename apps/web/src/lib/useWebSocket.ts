import { useEffect, useRef, useState, useCallback } from 'react';

const WS_BASE_URL = import.meta.env?.VITE_WS_BASE_URL || 'ws://localhost:3000/ws';

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
  const maxReconnectAttempts = 5;
  const initialBackoff = 2000;

  // Store callbacks in refs to avoid effect re-runs
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

    // Clear any pending reconnect timer
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
        reconnectAttempts.current = 0;

        if (room) {
          ws.send(JSON.stringify({ type: 'subscribe', room }));
        }

        onReconnectRef.current?.();
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          onMessageRef.current?.(data);
        } catch {
          // ignore parse errors
        }
      };

      ws.onerror = () => {
        // onclose will fire next, handle reconnection there
      };

      ws.onclose = (event) => {
        wsRef.current = null;
        if (!mountedRef.current) return;
        if (event.code === 1000 || event.code === 1005) {
          setStatus('disconnected');
          return;
        }

        setStatus('error');
        if (reconnectAttempts.current < maxReconnectAttempts) {
          const backoff = Math.min(initialBackoff * Math.pow(1.5, reconnectAttempts.current), 15000);
          const jitter = Math.random() * 1000;
          reconnectTimer.current = setTimeout(() => {
            if (mountedRef.current) {
              reconnectAttempts.current += 1;
              connect();
            }
          }, backoff + jitter);
        } else {
          setStatus('disconnected');
        }
      };
    } catch {
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

    return () => {
      mountedRef.current = false;
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
      if (wsRef.current) {
        if (room && wsRef.current.readyState === WebSocket.OPEN) {
          try { wsRef.current.send(JSON.stringify({ type: 'unsubscribe', room })); } catch {}
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
