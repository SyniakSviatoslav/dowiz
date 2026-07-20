export interface NotificationProvider {
  notify(target: string, event: string, data: any): Promise<void>;
}

export interface GeocodingProvider {
  geocode(address: string): Promise<{ lat: number; lng: number; confidence: number } | null>;
}

export interface ThemeRenderer {
  render(theme: any): Promise<string>;
}

export class StubNotificationProvider implements NotificationProvider {
  async notify(target: string, event: string, data: any): Promise<void> {
    console.log(`[NotificationProvider] Notify ${target} event: ${event}`, data);
  }
}

export class StubGeocodingProvider implements GeocodingProvider {
  async geocode(address: string): Promise<{ lat: number; lng: number; confidence: number } | null> {
    console.log(`[GeocodingProvider] Geocode: ${address}`);
    return { lat: 0, lng: 0, confidence: 1.0 };
  }
}

export class StubThemeRenderer implements ThemeRenderer {
  async render(theme: any): Promise<string> {
    console.log(`[ThemeRenderer] Render theme:`, theme);
    return "hash_stub";
  }
}
