import { Page } from '@playwright/test';

/**
 * Helper to wait for a WebSocket connection to a specific room.
 */
export async function waitForWS(
  page: Page,
  roomPattern: string,
  timeout: number = 10000
): Promise<string | null> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(null), timeout);

    page.on('websocket', (ws) => {
      const url = ws.url();
      if (url.includes(roomPattern)) {
        clearTimeout(timer);
        resolve(url);
      }
    });
  });
}

/**
 * Simulate a WebSocket message from server to test UI updates.
 */
export async function simulateWSMessage(
  page: Page,
  eventType: string,
  payload: Record<string, unknown>
): Promise<void> {
  await page.evaluate(
    ({ type, data }) => {
      window.dispatchEvent(
        new CustomEvent(`dos:ws:${type}`, { detail: data })
      );
    },
    { type: eventType, data: payload }
  );
}

/**
 * Wait for a WebSocket message to be processed by the UI.
 */
export async function waitForWSEvent(
  page: Page,
  eventType: string,
  timeout: number = 10000
): Promise<any> {
  return page.evaluate(
    (type) => {
      return new Promise((resolve) => {
        const handler = (e: Event) => {
          window.removeEventListener(`dos:ws:${type}`, handler);
          resolve((e as CustomEvent).detail);
        };
        window.addEventListener(`dos:ws:${type}`, handler);
      });
    },
    eventType
  );
}

/**
 * Kill WebSocket connection to simulate dead channel.
 * Call this to test reconnection/reconciliation logic.
 */
export async function killWebSocket(page: Page): Promise<void> {
  await page.evaluate(() => {
    // Close all WebSocket connections
    const sockets = (window as any).__activeSockets || [];
    sockets.forEach((ws: WebSocket) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.close(1001, 'Simulated disconnect');
      }
    });
  });
}

/**
 * Track all WebSocket connections in the page for later inspection.
 */
export async function trackWebSockets(page: Page): Promise<void> {
  await page.evaluate(() => {
    (window as any).__activeSockets = [];
    const OrigWebSocket = window.WebSocket;
    (window as any).WebSocket = function (...args: any[]) {
      const ws = new OrigWebSocket(...args);
      (window as any).__activeSockets.push(ws);
      return ws;
    };
    (window as any).WebSocket.prototype = OrigWebSocket.prototype;
  });
}
