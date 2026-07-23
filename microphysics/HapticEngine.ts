import type { HapticConfig, HapticEvent, HapticPattern } from './types.ts';

type TapticImpactStyle = 'light' | 'medium' | 'heavy' | 'rigid' | 'soft';

type TapticMessage = {
  type: 'impact' | 'notification' | 'selection';
  style?: TapticImpactStyle;
  intensity: number;
};

interface TapticEngineAPI {
  impact(intensity: number): void;
}

interface HapticFeedbackAPI {
  (style: string, intensity: number): void;
}

interface WebkitWindow {
  webkit?: {
    messageHandlers?: {
      taptic?: {
        postMessage(message: TapticMessage): void;
      };
    };
  };
  TapticEngine?: TapticEngineAPI;
  HapticFeedback?: HapticFeedbackAPI;
}

export class HapticEngine {
  private config: HapticConfig;
  private supportsVibrate: boolean;
  private supportsTaptic: boolean;

  private static readonly PATTERN_PARAMS: Record<HapticPattern, { duration_ms: number; taptic_style: TapticImpactStyle; taptic_type: 'impact' | 'notification' | 'selection' }> = {
    press_light: { duration_ms: 10, taptic_style: 'light', taptic_type: 'impact' },
    press_medium: { duration_ms: 20, taptic_style: 'medium', taptic_type: 'impact' },
    press_heavy: { duration_ms: 35, taptic_style: 'heavy', taptic_type: 'impact' },
    release: { duration_ms: 5, taptic_style: 'soft', taptic_type: 'selection' },
    click: { duration_ms: 15, taptic_style: 'rigid', taptic_type: 'impact' },
  };

  constructor(config: HapticConfig) {
    this.config = { ...config };
    this.supportsVibrate = typeof navigator !== 'undefined' && 'vibrate' in navigator;
    this.supportsTaptic = HapticEngine.detectTapticEngine();
  }

  private static detectTapticEngine(): boolean {
    try {
      const win = window as unknown as WebkitWindow;
      return !!(
        win.webkit?.messageHandlers?.taptic ||
        win.TapticEngine ||
        win.HapticFeedback
      );
    } catch {
      return false;
    }
  }

  private mapIntensity(brandIntensity: number, eventIntensity: number): number {
    return Math.min(1, Math.max(0, brandIntensity * 0.5 + eventIntensity * 0.5));
  }

  async trigger(event: HapticEvent): Promise<void> {
    const intensity = this.mapIntensity(this.config.brand_intensity, event.intensity);

    if (this.config.ios_taptic && this.supportsTaptic) {
      await this.sendTaptic(event, intensity);
    } else if (this.config.fallback_enabled && this.supportsVibrate) {
      this.sendVibrate(event, intensity);
    }
  }

  private sendTaptic(event: HapticEvent, intensity: number): Promise<void> {
    return new Promise<void>((resolve) => {
      try {
        const win = window as unknown as WebkitWindow;
        const params = HapticEngine.PATTERN_PARAMS[event.pattern] ?? HapticEngine.PATTERN_PARAMS.click;

        if (win.webkit?.messageHandlers?.taptic) {
          win.webkit.messageHandlers.taptic.postMessage({
            type: params.taptic_type,
            style: params.taptic_style,
            intensity,
          });
        } else if (win.TapticEngine) {
          win.TapticEngine.impact(intensity);
        } else if (win.HapticFeedback) {
          win.HapticFeedback(params.taptic_style, intensity);
        }
        resolve();
      } catch {
        if (this.config.fallback_enabled && this.supportsVibrate) {
          this.sendVibrate(event, intensity);
        }
        resolve();
      }
    });
  }

  private sendVibrate(event: HapticEvent, intensity: number): void {
    const baseDuration = HapticEngine.PATTERN_PARAMS[event.pattern]?.duration_ms ?? 15;
    const duration = Math.round(baseDuration * (0.5 + intensity * 0.5));
    navigator.vibrate(duration);
  }

  updateConfig(config: Partial<HapticConfig>): void {
    Object.assign(this.config, config);
    this.supportsTaptic = HapticEngine.detectTapticEngine();
  }

  get supportsAny(): boolean {
    return this.supportsVibrate || this.supportsTaptic;
  }

  destroy(): void {
    this.config.fallback_enabled = false;
  }
}
