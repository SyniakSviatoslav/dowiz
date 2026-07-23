import type { EnvironmentState, WeatherCondition, ConnectivityType } from '../expanded-types.ts';

type RenderDeltas = {
  wave_energy_delta: number;
  turing_feed_delta: number;
  micro_stiffness_delta: number;
  palette_warmth_delta: number;
  bloom_strength_delta: number;
  map_style_delta: 'day' | 'night' | 'rain' | 'snow';
};

function getSeason(date: Date): string {
  const m = date.getMonth();
  if (m >= 2 && m <= 4) return 'spring';
  if (m >= 5 && m <= 7) return 'summer';
  if (m >= 8 && m <= 10) return 'autumn';
  return 'winter';
}

function dayOfWeek(date: Date): number {
  return date.getDay();
}

export class EnvironmentSensor {
  private state: EnvironmentState;
  private pollingInterval: ReturnType<typeof setInterval> | null = null;
  private changeCallbacks: Array<(state: EnvironmentState) => void> = [];
  private networkOnline = true;

  constructor() {
    this.state = {
      weather: { condition: 'clear', temp: 20, humidity: 50 },
      battery: { level: 1, charging: true },
      connectivity: { online: true, type: 'wifi', latency_ms: 10 },
      time: { hour: 12, day_of_week: 1, season: 'summer' },
    };
  }

  async start(): Promise<void> {
    this.updateTime();

    try {
      await this.fetchWeather();
    } catch {
      const cached = this.getCachedState();
      if (cached) {
        this.state.weather = cached.weather;
      }
    }

    try {
      await this.sampleBattery();
    } catch {
      /* battery API not available */
    }

    this.sampleNetwork();

    window.addEventListener('online', this.handleOnline);
    window.addEventListener('offline', this.handleOffline);

    if ('connection' in navigator) {
      const conn = (navigator as unknown as { connection: EventTarget }).connection;
      conn.addEventListener('change', this.handleNetworkChange);
    }

    this.pollingInterval = setInterval(() => {
      this.poll();
    }, 30000);
  }

  stop(): void {
    if (this.pollingInterval) {
      clearInterval(this.pollingInterval);
      this.pollingInterval = null;
    }
    window.removeEventListener('online', this.handleOnline);
    window.removeEventListener('offline', this.handleOffline);
    if ('connection' in navigator) {
      const conn = (navigator as unknown as { connection: EventTarget }).connection;
      conn.removeEventListener('change', this.handleNetworkChange);
    }
  }

  getState(): EnvironmentState {
    return { ...this.state };
  }

  getRenderDeltas(): RenderDeltas {
    const deltas: RenderDeltas = {
      wave_energy_delta: 0,
      turing_feed_delta: 0,
      micro_stiffness_delta: 0,
      palette_warmth_delta: 0,
      bloom_strength_delta: 0,
      map_style_delta: 'day',
    };

    const { weather, battery, time } = this.state;

    if (weather.condition === 'sunny' || weather.condition === 'clear') {
      deltas.wave_energy_delta += 0.2;
      deltas.palette_warmth_delta += 0.15;
    } else if (weather.condition === 'rain') {
      deltas.wave_energy_delta -= 0.2;
      deltas.palette_warmth_delta -= 0.15;
      deltas.map_style_delta = 'rain';
    } else if (weather.condition === 'cloudy') {
      deltas.wave_energy_delta -= 0.1;
      deltas.palette_warmth_delta -= 0.05;
    } else if (weather.condition === 'snow') {
      deltas.wave_energy_delta -= 0.15;
      deltas.palette_warmth_delta += 0.1;
      deltas.map_style_delta = 'snow';
    }

    if (weather.temp > 30) {
      deltas.turing_feed_delta += 0.01;
    } else if (weather.temp < 5) {
      deltas.turing_feed_delta -= 0.01;
    }

    if (battery.level < 0.2 && !battery.charging) {
      deltas.wave_energy_delta -= 0.3;
      deltas.micro_stiffness_delta -= 20;
    } else if (battery.level < 0.5 && !battery.charging) {
      deltas.wave_energy_delta -= 0.1;
      deltas.micro_stiffness_delta -= 5;
    }

    if (time.hour < 6 || time.hour > 20) {
      deltas.bloom_strength_delta += 0.3;
      deltas.palette_warmth_delta -= 0.2;
      deltas.map_style_delta = 'night';
    } else if (time.hour < 8 || time.hour > 17) {
      deltas.bloom_strength_delta += 0.1;
      deltas.palette_warmth_delta -= 0.1;
    } else {
      deltas.bloom_strength_delta -= 0.1;
    }

    if (weather.humidity > 80) {
      deltas.bloom_strength_delta += 0.1;
    }

    return deltas;
  }

  onChange(cb: (state: EnvironmentState) => void): void {
    this.changeCallbacks.push(cb);
  }

  getCachedState(): EnvironmentState | null {
    try {
      const raw = localStorage.getItem('dowiz_env_cache');
      if (raw) return JSON.parse(raw) as EnvironmentState;
    } catch {
      /* ignore */
    }
    return null;
  }

  private cacheState(): void {
    try {
      localStorage.setItem('dowiz_env_cache', JSON.stringify(this.state));
    } catch {
      /* ignore */
    }
  }

  private emitChange(): void {
    for (const cb of this.changeCallbacks) {
      cb(this.state);
    }
  }

  private poll(): void {
    this.updateTime();
    this.sampleNetwork();

    if (this.networkOnline) {
      this.fetchWeather().catch(() => {});
      this.sampleBattery().catch(() => {});
    }
  }

  private updateTime(): void {
    const now = new Date();
    this.state.time = {
      hour: now.getHours(),
      day_of_week: dayOfWeek(now),
      season: getSeason(now),
    };
  }

  private async fetchWeather(): Promise<void> {
    try {
      const lat = 50.45;
      const lng = 30.52;

      const url = `https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lng}&current_weather=true&hourly=temperature_2m,relative_humidity_2m&forecast_days=1`;
      const resp = await fetch(url);
      if (!resp.ok) return;
      const data = await resp.json();

      const wcode = data.current_weather?.weathercode ?? 0;
      let condition: WeatherCondition = 'clear';
      if (wcode >= 51 && wcode <= 67) condition = 'rain';
      else if (wcode >= 71 && wcode <= 77) condition = 'snow';
      else if (wcode >= 80 && wcode <= 99) condition = 'rain';
      else if (wcode >= 21 && wcode <= 29) condition = 'cloudy';
      else if (wcode >= 41 && wcode <= 49) condition = 'cloudy';

      const temp = data.current_weather?.temperature ?? 20;
      const humidity = data.hourly?.relative_humidity_2m?.[0] ?? 50;

      this.state.weather = { condition, temp, humidity };
      this.cacheState();
      this.emitChange();
    } catch {
      throw new Error('Failed to fetch weather');
    }
  }

  private async sampleBattery(): Promise<void> {
    try {
      const battery = await (navigator as unknown as { getBattery(): Promise<{ level: number; charging: boolean }> }).getBattery();
      this.state.battery = {
        level: battery.level,
        charging: battery.charging,
      };
      this.cacheState();
      this.emitChange();
    } catch {
      throw new Error('Battery API not available');
    }
  }

  private sampleNetwork(): void {
    const conn = (navigator as unknown as { connection?: { effectiveType?: string; rtt?: number } }).connection;
    let type: ConnectivityType = 'wifi';
    let latencyMs = 10;

    if (conn) {
      const et = conn.effectiveType;
      if (et === 'cellular' || et === '4g' || et === '3g' || et === '2g') {
        type = 'cellular';
      }
      if (conn.rtt && conn.rtt > 0) {
        latencyMs = conn.rtt;
      }
    }

    this.state.connectivity = {
      online: this.networkOnline,
      type,
      latency_ms: latencyMs,
    };
    this.emitChange();
  }

  private handleOnline = (): void => {
    this.networkOnline = true;
    this.sampleNetwork();
  };

  private handleOffline = (): void => {
    this.networkOnline = false;
    this.sampleNetwork();
  };

  private handleNetworkChange = (): void => {
    this.sampleNetwork();
  };
}
