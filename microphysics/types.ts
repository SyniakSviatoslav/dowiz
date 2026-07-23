/// <reference types="@webgpu/types" />

export type BrandMood = {
  energy: number;
  formality: number;
};

export type BrandGeometry = {
  stroke_weight_ratio: number;
};

export type BrandConfig = {
  mood: BrandMood;
  geometry: BrandGeometry;
};

export type SpringDamperGpuParams = {
  stiffness: number;
  damping: number;
  max_displacement: number;
  rest_position: number;
};

export type SpringDamperConfig = {
  stiffness: number;
  damping: number;
  max_displacement: number;
  rest_position: number;
};

export type TuringGpuParams = {
  diffusion_rate_u: number;
  diffusion_rate_v: number;
  feed_rate: number;
  kill_rate: number;
  dt: number;
  grid_width: number;
  grid_height: number;
  injection_strength: number;
  injection_radius: number;
};

export type TuringConfig = TuringGpuParams;

export type WaveConfig = {
  amplitude: number;
  frequency: number;
  phase: number;
  decay: number;
  enabled: boolean;
};

export type HapticConfig = {
  brand_intensity: number;
  fallback_enabled: boolean;
  ios_taptic: boolean;
};

export type MicrophysicsConfig = {
  spring: SpringDamperConfig;
  turing: TuringConfig;
  haptic: HapticConfig;
};

export type PhysicsState = {
  displacement: number;
  displacement_velocity: number;
  position_x: number;
  position_y: number;
  velocity_x: number;
  velocity_y: number;
  pressure: number;
  target_x: number;
  target_y: number;
  active: number;
};

export type PhysicsStateArray = {
  displacement: Float32Array;
  displacement_velocity: Float32Array;
  position_x: Float32Array;
  position_y: Float32Array;
  velocity_x: Float32Array;
  velocity_y: Float32Array;
  pressure: Float32Array;
  target_x: Float32Array;
  target_y: Float32Array;
  active: Uint32Array;
  count: number;
};

export type PointerData = {
  id: number;
  position: [number, number];
  previousPosition: [number, number];
  velocity: [number, number];
  pressure: number;
  displacement: number;
  pointerType: PointerType;
  active: boolean;
  timestamp: number;
};

export type PointerType = 'mouse' | 'touch' | 'pen';

export type HapticPattern = 'press_light' | 'press_medium' | 'press_heavy' | 'release' | 'click';

export type HapticEvent = {
  pattern: HapticPattern;
  intensity: number;
  duration_ms: number;
};

export type MicrophysicsEvent = {
  type: 'press' | 'release' | 'move';
  pointerId: number;
  displacement: number;
  pressure: number;
  position: [number, number];
};

export type TuringGridState = {
  u: Float32Array;
  v: Float32Array;
  width: number;
  height: number;
};

export type InjectionPoint = {
  x: number;
  y: number;
  pressure: number;
  active: number;
};

export type SplatDisplacement = {
  pointerId: number;
  offsetX: number;
  offsetY: number;
  displacement: number;
};

export type DisplacementCallback = (displacements: Map<number, SplatDisplacement>) => void;

export type MicrophysicsEventCallback = (event: MicrophysicsEvent) => void;

export const PHYSICS_STATE_BYTES = 40;
export const INJECTION_POINT_BYTES = 16;
export const SPRING_PARAMS_BYTES = 16;
export const TURING_PARAMS_BYTES = 36;
export const MAX_POINTERS = 10;
export const MAX_INJECTION_POINTS = 10;
