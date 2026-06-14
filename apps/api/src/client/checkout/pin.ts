// @ts-nocheck
export interface LocationPin {
  lat: number;
  lng: number;
}

export async function requestGeolocation(): Promise<LocationPin | null> {
  return new Promise((resolve) => {
    if (!navigator.geolocation) {
      window.dispatchEvent(new CustomEvent('fallback:needed', { detail: { reason: 'geocode_failed' } }));
      resolve(null);
      return;
    }

    navigator.geolocation.getCurrentPosition(
      (pos) => {
        resolve({ lat: pos.coords.latitude, lng: pos.coords.longitude });
      },
      (err) => {
        console.warn('Geolocation failed or denied', err);
        window.dispatchEvent(new CustomEvent('fallback:needed', { detail: { reason: 'geocode_failed' } }));
        resolve(null);
      },
      { timeout: 10000, enableHighAccuracy: true }
    );
  });
}
