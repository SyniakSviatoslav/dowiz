import type { OrderStage, OrderArc, DiscoveryAction } from '../expanded-types.ts';
import type { WaveConfig, SpringDamperConfig, TuringConfig } from '../microphysics/types.ts';

type StageWeight = {
  wave_amplitude: number;
  turing_evolution_speed: number;
  microphysics_stiffness: number;
  duration_seconds: number;
};

const STAGE_WEIGHTS: Record<OrderStage, StageWeight> = {
  discover: { wave_amplitude: 0.3, turing_evolution_speed: 0.002, microphysics_stiffness: 40, duration_seconds: 120 },
  browse: { wave_amplitude: 0.5, turing_evolution_speed: 0.005, microphysics_stiffness: 60, duration_seconds: 300 },
  cart: { wave_amplitude: 0.7, turing_evolution_speed: 0.008, microphysics_stiffness: 80, duration_seconds: 180 },
  order: { wave_amplitude: 1.0, turing_evolution_speed: 0.012, microphysics_stiffness: 100, duration_seconds: 90 },
  track: { wave_amplitude: 0.6, turing_evolution_speed: 0.006, microphysics_stiffness: 50, duration_seconds: 600 },
  receive: { wave_amplitude: 0.8, turing_evolution_speed: 0.01, microphysics_stiffness: 70, duration_seconds: 120 },
  review: { wave_amplitude: 0.4, turing_evolution_speed: 0.003, microphysics_stiffness: 30, duration_seconds: 300 },
};

const DISCOVERY_ACTIONS: Record<OrderStage, DiscoveryAction[]> = {
  discover: ['splat_reveal', 'map_pulse'],
  browse: ['content_unfold'],
  cart: ['splat_reveal', 'content_unfold'],
  order: ['splat_reveal', 'courier_approach'],
  track: ['map_pulse', 'courier_approach'],
  receive: ['splat_reveal', 'content_unfold'],
  review: ['splat_reveal'],
};

const STAGE_ORDER: OrderStage[] = ['discover', 'browse', 'cart', 'order', 'track', 'receive', 'review'];

export class OrderArcSystem {
  private currentStage: OrderStage = 'discover';
  private elapsedInStage = 0;
  private stageEnterCallbacks: Array<(stage: OrderStage) => void> = [];
  private stageExitCallbacks: Array<(stage: OrderStage) => void> = [];

  constructor(
    private wave: { setAmplitude: (a: number) => void },
    private turing: { setFeedRate: (r: number) => void },
    private microphysics: { setStiffness: (s: number) => void },
  ) {}

  setStage(stage: OrderStage): void {
    if (stage === this.currentStage) return;

    for (const cb of this.stageExitCallbacks) {
      cb(this.currentStage);
    }

    const prevWeight = STAGE_WEIGHTS[this.currentStage];
    const nextWeight = STAGE_WEIGHTS[stage];

    this.currentStage = stage;
    this.elapsedInStage = 0;

    this.wave.setAmplitude(nextWeight.wave_amplitude);
    this.turing.setFeedRate(nextWeight.turing_evolution_speed);
    this.microphysics.setStiffness(nextWeight.microphysics_stiffness);

    for (const cb of this.stageEnterCallbacks) {
      cb(stage);
    }
  }

  getCurrentArc(): OrderArc {
    const stage = this.currentStage;
    const weight = STAGE_WEIGHTS[stage];
    const actions = DISCOVERY_ACTIONS[stage];

    return {
      stage,
      wave_profile: {
        amplitude: weight.wave_amplitude,
        frequency: 0.5,
        phase: 0,
        decay: 0.1,
        enabled: true,
      },
      turing_profile: {
        diffusion_rate_u: 0.16,
        diffusion_rate_v: 0.08,
        feed_rate: weight.turing_evolution_speed * 100,
        kill_rate: 0.06,
        dt: 1.0,
        grid_width: 128,
        grid_height: 128,
        injection_strength: weight.wave_amplitude * 0.5,
        injection_radius: 12,
      },
      microphysics_profile: {
        stiffness: weight.microphysics_stiffness,
        damping: 12,
        max_displacement: 30,
        rest_position: 0,
      },
      expected_duration_seconds: weight.duration_seconds,
      discovery_actions: actions,
    };
  }

  advance(): void {
    const idx = STAGE_ORDER.indexOf(this.currentStage);
    if (idx < STAGE_ORDER.length - 1) {
      this.setStage(STAGE_ORDER[idx + 1]);
    }
  }

  onStageEnter(cb: (stage: OrderStage) => void): void {
    this.stageEnterCallbacks.push(cb);
  }

  onStageExit(cb: (stage: OrderStage) => void): void {
    this.stageExitCallbacks.push(cb);
  }

  getCurrentStage(): OrderStage {
    return this.currentStage;
  }

  update(dt: number): void {
    this.elapsedInStage += dt;
  }

  getElapsedInStage(): number {
    return this.elapsedInStage;
  }
}
