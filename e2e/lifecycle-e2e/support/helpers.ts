/* eslint-disable @typescript-eslint/no-explicit-any, max-params -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import type { Page, BrowserContext } from '@playwright/test';

export function collectWsFrames(page: Page): { frames: string[] } {
  const frames: string[] = [];
  page.on('websocket', (ws) => {
    ws.on('framereceived', (e) => {
      if (typeof e.payload === 'string') frames.push(e.payload);
    });
  });
  return { frames };
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

export async function driveAlongTrack(
  context: BrowserContext,
  from: { latitude: number; longitude: number },
  to: { latitude: number; longitude: number },
  steps: number,
  pingIntervalMs: number,
): Promise<void> {
  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    await context.setGeolocation({
      latitude: lerp(from.latitude, to.latitude, t),
      longitude: lerp(from.longitude, to.longitude, t),
    });
    await new Promise((r) => setTimeout(r, pingIntervalMs));
  }
}

export function extractOrderId(body: unknown): string {
  const b = body as Record<string, any>;
  const id = b?.id ?? b?.order_id ?? b?.data?.id ?? b?.order?.id;
  if (!id) throw new Error(`[e2e] Could not find order id in response: ${JSON.stringify(body)}`);
  return String(id);
}

export function extractToken(body: unknown): string {
  const b = body as Record<string, any>;
  const token = b?.access_token ?? b?.token ?? b?.jwt ?? b?.data?.token;
  if (!token) throw new Error(`[e2e] Could not find token in login response: ${JSON.stringify(body)}`);
  return String(token);
}
