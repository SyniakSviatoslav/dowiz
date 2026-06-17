import type { TranslationProvider } from '../ports.js';
// @ts-ignore — legacy types removed from shared-types
import type { TranslateRequest, TranslateResponse } from '@deliveryos/shared-types';

export class LibreTranslateProvider implements TranslationProvider {
  readonly id = 'libretranslate';
  private endpoint = process.env.TRANSLATION_ENDPOINT || 'http://localhost:5000/translate';
  private consecutiveFailures = 0;

  async translate(req: TranslateRequest): Promise<TranslateResponse> {
    const t0 = Date.now();
    const providerId = process.env.TRANSLATION_PROVIDER || 'libretranslate';

    // Degradation fallback
    if (this.consecutiveFailures >= 3) {
      return {
        translations: req.texts, // original texts
        provider_id: providerId,
        model_id: 'fallback_degraded',
        pii_redacted_count: 0,
        duration_ms: Date.now() - t0
      };
    }

    try {
      const res = await fetch(this.endpoint, {
        method: 'POST',
        body: JSON.stringify({
          q: req.texts,
          source: req.from,
          target: req.to,
          format: 'text',
          api_key: ''
        }),
        headers: { 'Content-Type': 'application/json' },
        signal: AbortSignal.timeout(6000),
      });

      if (!res.ok) {
        throw new Error(`Translation API failed: ${res.statusText}`);
      }

      const data = await res.json() as any;
      const translations = data.translatedText; // libretranslate returns array if q was array

      this.consecutiveFailures = 0;

      return {
        translations: Array.isArray(translations) ? translations : [translations],
        provider_id: providerId,
        model_id: 'libretranslate_default',
        pii_redacted_count: 0,
        duration_ms: Date.now() - t0
      };
    } catch (e: any) {
      this.consecutiveFailures++;
      console.error(`[TranslationProvider] Error: ${e.message}. Consecutive failures: ${this.consecutiveFailures}`);
      
      // Degradation fallback
      return {
        translations: req.texts,
        provider_id: providerId,
        model_id: 'fallback_error',
        pii_redacted_count: 0,
        duration_ms: Date.now() - t0
      };
    }
  }
}
