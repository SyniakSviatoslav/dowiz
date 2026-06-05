import { Page, BrowserContext } from '@playwright/test';

/**
 * Emulate a GPS location on the page.
 */
export async function emulateGeo(
  context: BrowserContext,
  latitude: number,
  longitude: number,
  accuracy: number = 10
): Promise<void> {
  await context.grantPermissions(['geolocation']);
  await context.setGeolocation({ latitude, longitude, accuracy });
}

/**
 * Emulate a GPS track — a series of locations moving along a path.
 * Used for courier delivery simulation.
 */
export async function emulateGeoStream(
  page: Page,
  track: Array<{ lat: number; lng: number }>,
  intervalMs: number = 1000
): Promise<void> {
  for (const point of track) {
    await page.evaluate(
      ({ lat, lng }) => {
        window.dispatchEvent(
          new CustomEvent('dos:geo:update', { detail: { lat, lng, accuracy: 10 } })
        );
      },
      { lat: point.lat, lng: point.lng }
    );
    await page.waitForTimeout(intervalMs);
  }
}

/**
 * Simulate GPS denial (user rejected location permission).
 */
export async function simulateGPSDenied(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent('dos:geo:denied'));
  });
}

/**
 * Simulate GPS noise (inaccurate readings that should be filtered).
 */
export async function simulateGPSNoise(page: Page): Promise<void> {
  // Send a burst of noisy readings
  const noisePoints = [
    { lat: 41.3275 + Math.random() * 0.1, lng: 19.8187 + Math.random() * 0.1, accuracy: 500 },
    { lat: 41.3275 + Math.random() * 0.1, lng: 19.8187 + Math.random() * 0.1, accuracy: 1000 },
    { lat: 41.3275 + Math.random() * 0.5, lng: 19.8187 + Math.random() * 0.5, accuracy: 2000 },
  ];
  
  for (const point of noisePoints) {
    await page.evaluate(
      (p) => {
        window.dispatchEvent(
          new CustomEvent('dos:geo:update', { detail: p })
        );
      },
      point
    );
    await page.waitForTimeout(100);
  }
}

/**
 * Get the current simulated geolocation from the page.
 */
export async function getCurrentGeo(page: Page): Promise<{ lat: number; lng: number } | null> {
  return page.evaluate(() => {
    const geo = (window as any).__currentGeo;
    return geo || null;
  });
}
