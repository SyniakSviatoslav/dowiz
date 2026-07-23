/// <reference types="@webgpu/types" />

import type {
  MicrophysicsConfig, TuringConfig, SpringDamperConfig, WaveConfig,
} from './microphysics/types.ts';

export type OrderStage = 'discover' | 'browse' | 'cart' | 'order' | 'track' | 'receive' | 'review';

export type OrderArc = {
  stage: OrderStage;
  wave_profile: WaveConfig;
  turing_profile: TuringConfig;
  microphysics_profile: SpringDamperConfig;
  expected_duration_seconds: number;
  discovery_actions: DiscoveryAction[];
};

export type DiscoveryAction = 'splat_reveal' | 'map_pulse' | 'content_unfold' | 'courier_approach';

export type GeoCoord = {
  lat: number;
  lng: number;
  alt: number;
};

export type MapViewport = {
  center: GeoCoord;
  zoom: number;
  bearing: number;
  pitch: number;
};

export type MapTileKey = {
  z: number;
  x: number;
  y: number;
};

export type WeatherCondition = 'clear' | 'rain' | 'cloudy' | 'snow';

export type ConnectivityType = 'wifi' | 'cellular' | 'none';

export type EnvironmentState = {
  weather: {
    condition: WeatherCondition;
    temp: number;
    humidity: number;
  };
  battery: {
    level: number;
    charging: boolean;
  };
  connectivity: {
    online: boolean;
    type: ConnectivityType;
    latency_ms: number;
  };
  time: {
    hour: number;
    day_of_week: number;
    season: string;
  };
};

export type ModeType = 'atmosphere' | 'business';

export type WaveProfile = {
  enabled: boolean;
  amplitude?: number;
  frequency?: number;
  phase?: number;
  decay?: number;
};

export type TuringProfile = {
  enabled: boolean;
  diffusion_rate_u?: number;
  diffusion_rate_v?: number;
  feed_rate?: number;
  kill_rate?: number;
  dt?: number;
  grid_width?: number;
  grid_height?: number;
  injection_strength?: number;
  injection_radius?: number;
};

export type MicroProfile = {
  enabled: boolean;
  stiffness?: number;
  damping?: number;
  max_displacement?: number;
  rest_position?: number;
};

export type MapProfile = 'explore' | 'minimal';

export type ModeConfig = {
  current: ModeType;
  atmosphere_profile: {
    wave: WaveConfig;
    turing: TuringConfig;
    microphysics: SpringDamperConfig;
  };
  business_profile: {
    wave: WaveConfig;
    turing: TuringConfig;
    microphysics: SpringDamperConfig;
  };
};

export type PostOrderHook = {
  type: string;
  channel: 'instagram' | 'telegram' | 'whatsapp';
  text: string;
  icon_splat_config: {
    icon: string;
    size: number;
    color: [number, number, number, number];
    animation: string;
  };
  action: string;
};

export type OfflineQueueEntryStatus = 'pending' | 'syncing' | 'synced' | 'failed';

export type OfflineQueueEntry = {
  id: string;
  stage: OrderStage;
  payload: Record<string, unknown>;
  created_at: number;
  synced_at: number | null;
  retry_count: number;
  status: OfflineQueueEntryStatus;
};

export type DowizRuntimeConfig = {
  viewport: MapViewport;
  environment: EnvironmentState;
  mode: ModeConfig;
  arcs: OrderArc[];
  hooks: PostOrderHook[];
  offline_queue: OfflineQueueEntry[];
  microphysics: MicrophysicsConfig;
  turing: TuringConfig;
  wave: WaveConfig;
};

export const GEO_PARAMS_BYTES = 64;
export const TILE_DATA_BYTES = 32 * 1024;
export const MAX_TILES = 64;
export const MAX_QUEUE = 1000;
