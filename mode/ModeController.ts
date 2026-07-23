import type { ModeType, WaveProfile, TuringProfile, MicroProfile, MapProfile } from '../expanded-types.ts';

type ActiveProfiles = {
  wave: WaveProfile | null;
  turing: TuringProfile | null;
  micro: MicroProfile | null;
  map: MapProfile;
  transitions: boolean;
  haptics: boolean;
};

export class ModeController {
  private mode: ModeType = 'atmosphere';
  private modeChangeCallbacks: Array<(mode: ModeType) => void> = [];

  constructor() {
    this.mode = 'atmosphere';
  }

  getMode(): ModeType {
    return this.mode;
  }

  setMode(mode: ModeType): void {
    if (mode === this.mode) return;
    this.mode = mode;
    for (const cb of this.modeChangeCallbacks) {
      cb(mode);
    }
  }

  toggle(): void {
    this.setMode(this.mode === 'atmosphere' ? 'business' : 'atmosphere');
  }

  autoSelect(context: {
    isFirstVisit: boolean;
    batteryLevel: number;
    isOnline: boolean;
    orderCount: number;
  }): ModeType {
    if (context.isFirstVisit) {
      return 'atmosphere';
    }
    if (context.batteryLevel < 0.2) {
      return 'business';
    }
    if (!context.isOnline) {
      return 'business';
    }
    if (context.orderCount >= 3) {
      return 'business';
    }
    return this.mode;
  }

  getActiveProfiles(): ActiveProfiles {
    if (this.mode === 'business') {
      return {
        wave: { enabled: false },
        turing: { enabled: false },
        micro: { enabled: false },
        map: 'minimal',
        transitions: false,
        haptics: false,
      };
    }

    return {
      wave: {
        enabled: true,
        amplitude: 0.6,
        frequency: 0.5,
        phase: 0,
        decay: 0.1,
      },
      turing: {
        enabled: true,
        diffusion_rate_u: 0.16,
        diffusion_rate_v: 0.08,
        feed_rate: 0.035,
        kill_rate: 0.06,
        dt: 1.0,
        grid_width: 128,
        grid_height: 128,
        injection_strength: 0.3,
        injection_radius: 12,
      },
      micro: {
        enabled: true,
        stiffness: 60,
        damping: 12,
        max_displacement: 30,
        rest_position: 0,
      },
      map: 'explore',
      transitions: true,
      haptics: true,
    };
  }

  onModeChange(cb: (mode: ModeType) => void): void {
    this.modeChangeCallbacks.push(cb);
  }
}
