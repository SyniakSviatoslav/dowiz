/// <reference types="@webgpu/types" />

import type { MapViewport, GeoCoord, MapTileKey } from '../expanded-types.ts';
import { GEO_PARAMS_BYTES, TILE_DATA_BYTES, MAX_TILES } from '../expanded-types.ts';

const TILE_SIZE = 256;
const EARTH_CIRCUMFERENCE = 40075016.686;

type TileDataHeader = {
  type: number;
  count: number;
  coords: Float32Array;
};

type LoadedTile = {
  key: MapTileKey;
  data: TileDataHeader[];
  byteOffset: number;
  byteLength: number;
};

function webMercatorX(lng: number, zoom: number): number {
  return ((lng + 180) / 360) * Math.pow(2, zoom);
}

function webMercatorY(lat: number, zoom: number): number {
  const latRad = lat * Math.PI / 180;
  const y = (1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2;
  return y * Math.pow(2, zoom);
}

function lonLatToTile(lng: number, lat: number, zoom: number): MapTileKey {
  const x = Math.floor(webMercatorX(lng, zoom));
  const y = Math.floor(webMercatorY(lat, zoom));
  return { z: Math.round(zoom), x, y };
}

function tileBounds(key: MapTileKey): { north: number; south: number; east: number; west: number } {
  const n = Math.pow(2, key.z);
  const west = key.x / n * 360 - 180;
  const east = (key.x + 1) / n * 360 - 180;
  const latNorthRad = Math.atan(Math.sinh(Math.PI * (1 - 2 * key.y / n)));
  const latSouthRad = Math.atan(Math.sinh(Math.PI * (1 - 2 * (key.y + 1) / n)));
  return {
    north: latNorthRad * 180 / Math.PI,
    south: latSouthRad * 180 / Math.PI,
    east,
    west,
  };
}

function distanceKm(a: GeoCoord, b: GeoCoord): number {
  const R = 6371;
  const dLat = (b.lat - a.lat) * Math.PI / 180;
  const dLng = (b.lng - a.lng) * Math.PI / 180;
  const sinDLat = Math.sin(dLat / 2);
  const sinDLng = Math.sin(dLng / 2);
  const aVal = sinDLat * sinDLat + Math.cos(a.lat * Math.PI / 180) * Math.cos(b.lat * Math.PI / 180) * sinDLng * sinDLng;
  return R * 2 * Math.atan2(Math.sqrt(aVal), Math.sqrt(1 - aVal));
}

function latAtRadius(centerLat: number, radiusKm: number): number {
  return radiusKm / 6371 * 180 / Math.PI;
}

function lngAtRadius(centerLat: number, centerLng: number, radiusKm: number): number {
  const latRad = centerLat * Math.PI / 180;
  return radiusKm / 6371 * 180 / Math.PI / Math.cos(latRad);
}

export class GeoEngine {
  private device: GPUDevice;
  private viewport: MapViewport;
  private viewportUniformBuffer: GPUBuffer;
  private tileDataBuffer: GPUBuffer;
  private tileCountBuffer: GPUBuffer;
  private pipeline: GPUComputePipeline;
  private bindGroup: GPUBindGroup;
  private loadedTiles: Map<string, LoadedTile> = new Map();
  private routeCoords: GeoCoord[] = [];
  private dbPromise: Promise<IDBDatabase> | null = null;

  constructor(device: GPUDevice) {
    this.device = device;
    this.viewport = {
      center: { lat: 50.45, lng: 30.52, alt: 0 },
      zoom: 13,
      bearing: 0,
      pitch: 0,
    };

    this.viewportUniformBuffer = device.createBuffer({
      size: GEO_PARAMS_BYTES,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    this.tileDataBuffer = device.createBuffer({
      size: MAX_TILES * TILE_DATA_BYTES,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });

    this.tileCountBuffer = device.createBuffer({
      size: 4,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });

    this.pipeline = this.createComputePipeline();
    this.bindGroup = this.createBindGroup();
  }

  private createComputePipeline(): GPUComputePipeline {
    const shaderModule = this.device.createShaderModule({
      code: `
        struct ViewportUniforms {
          center_lat: f32,
          center_lng: f32,
          zoom: f32,
          bearing: f32,
          pitch: f32,
          screen_w: f32,
          screen_h: f32,
          tile_size: f32,
        }

        struct TileVertex {
          pos_x: f32,
          pos_y: f32,
          color_r: f32,
          color_g: f32,
          color_b: f32,
        }

        struct TileHeader {
          type: u32,
          count: u32,
        }

        @group(0) @binding(0) var<uniform> uniforms: ViewportUniforms;
        @group(0) @binding(1) var<storage, read> tile_data: array<u32>;
        @group(0) @binding(2) var<storage, read_write> tile_count: array<u32>;
        @group(0) @binding(3) var<storage, read_write> output_verts: array<TileVertex>;

        const PI: f32 = 3.141592653589793;

        fn mercator_x(lng: f32, zoom: f32) -> f32 {
          return ((lng + 180.0) / 360.0) * pow(2.0, zoom);
        }

        fn mercator_y(lat: f32, zoom: f32) -> f32 {
          let lat_rad: f32 = lat * PI / 180.0;
          let term: f32 = tan(lat_rad) + 1.0 / cos(lat_rad);
          return (1.0 - log(term) / PI) / 2.0 * pow(2.0, zoom);
        }

        fn screen_x(tile_x: f32, center_x: f32, zoom_scale: f32, bearing_cos: f32, bearing_sin: f32, sw: f32) -> f32 {
          let dx: f32 = tile_x - center_x;
          let rotated: f32 = dx * bearing_cos;
          return rotated * 256.0 * zoom_scale + sw / 2.0;
        }

        fn screen_y(tile_y: f32, center_y: f32, zoom_scale: f32, bearing_cos: f32, bearing_sin: f32, sh: f32) -> f32 {
          let dy: f32 = tile_y - center_y;
          let rotated: f32 = dy * bearing_cos;
          return rotated * 256.0 * zoom_scale + sh / 2.0;
        }

        fn tile_color(t: u32) -> vec3<f32> {
          if (t == 1u) {
            return vec3<f32>(0.25, 0.25, 0.25);
          } else if (t == 2u) {
            return vec3<f32>(0.7, 0.7, 0.75);
          } else if (t == 3u) {
            return vec3<f32>(0.3, 0.7, 0.3);
          }
          return vec3<f32>(0.9, 0.9, 0.92);
        }

        @compute @workgroup_size(64)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
          let tile_idx: u32 = id.x;
          let max_tiles: u32 = 64u;
          if (tile_idx >= max_tiles) { return; }

          let header_offset: u32 = tile_idx * 8192u;
          let header_word: u32 = tile_data[header_offset];
          let feature_type: u32 = header_word & 0xFFFFu;
          let feature_count: u32 = (header_word >> 16u) & 0xFFFFu;

          if (feature_count == 0u) { return; }

          let center_lat: f32 = uniforms.center_lat;
          let center_lng: f32 = uniforms.center_lng;
          let zoom: f32 = uniforms.zoom;
          let bearing: f32 = uniforms.bearing;
          let sw: f32 = uniforms.screen_w;
          let sh: f32 = uniforms.screen_h;
          let tile_size: f32 = uniforms.tile_size;

          let bearing_rad: f32 = bearing * PI / 180.0;
          let bearing_cos: f32 = cos(bearing_rad);
          let bearing_sin: f32 = sin(bearing_rad);
          let zoom_scale: f32 = pow(2.0, zoom) * tile_size / 256.0;

          let center_x: f32 = mercator_x(center_lng, zoom);
          let center_y: f32 = mercator_y(center_lat, zoom);

          let color: vec3<f32> = tile_color(feature_type);
          let base_vert: u32 = tile_idx * 4096u;

          for (var i: u32 = 0u; i < feature_count && i < 1000u; i = i + 1u) {
            let coord_offset: u32 = header_offset + 1u + i * 2u;
            let lng_raw: u32 = tile_data[coord_offset];
            let lat_raw: u32 = tile_data[coord_offset + 1u];

            let lng_f: f32 = (f32(lng_raw) / 10000000.0) * 0.001;
            let lat_f: f32 = (f32(lat_raw) / 10000000.0) * 0.001;

            let tx: f32 = mercator_x(lng_f, zoom);
            let ty: f32 = mercator_y(lat_f, zoom);

            let sx: f32 = screen_x(tx, center_x, zoom_scale, bearing_cos, bearing_sin, sw);
            let sy: f32 = screen_y(ty, center_y, zoom_scale, bearing_cos, bearing_sin, sh);

            let out_idx: u32 = base_vert + i;
            output_verts[out_idx].pos_x = (sx / sw) * 2.0 - 1.0;
            output_verts[out_idx].pos_y = (sy / sh) * 2.0 - 1.0;
            output_verts[out_idx].color_r = color.r;
            output_verts[out_idx].color_g = color.g;
            output_verts[out_idx].color_b = color.b;
          }
        }
      `,
    });

    return this.device.createComputePipeline({
      layout: 'auto',
      compute: { module: shaderModule, entryPoint: 'main' },
    });
  }

  private createBindGroup(): GPUBindGroup {
    return this.device.createBindGroup({
      layout: this.pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: this.viewportUniformBuffer } },
        { binding: 1, resource: { buffer: this.tileDataBuffer } },
        { binding: 2, resource: { buffer: this.tileCountBuffer } },
        { binding: 3, resource: { buffer: this.tileDataBuffer } },
      ],
    });
  }

  setViewport(vp: MapViewport): void {
    this.viewport = vp;
    const data = new Float32Array([
      vp.center.lat, vp.center.lng,
      vp.zoom, vp.bearing, vp.pitch,
      1920, 1080, TILE_SIZE,
    ]);
    this.device.queue.writeBuffer(this.viewportUniformBuffer, 0, data);
  }

  async cacheTiles(center: GeoCoord, radius_km: number, zoom: number): Promise<void> {
    const dLat = latAtRadius(center.lat, radius_km);
    const dLng = lngAtRadius(center.lat, center.lng, radius_km);

    const bounds = {
      north: center.lat + dLat,
      south: center.lat - dLat,
      east: center.lng + dLng,
      west: center.lng - dLng,
    };

    const tiles = this.getTileCoverage(bounds, zoom);
    const db = await this.getDatabase();

    for (const tile of tiles) {
      if (this.loadedTiles.has(`${tile.z}/${tile.x}/${tile.y}`)) continue;

      const existing = await this.loadTileFromDb(db, tile);
      if (existing) {
        this.loadedTiles.set(`${tile.z}/${tile.x}/${tile.y}`, existing);
      } else {
        const fetched = await this.fetchTileData(tile);
        if (fetched) {
          this.loadedTiles.set(`${tile.z}/${tile.x}/${tile.y}`, fetched);
          await this.saveTileToDb(db, tile, fetched);
        }
      }
    }
  }

  async loadCachedTiles(zoom: number, tiles: MapTileKey[]): Promise<void> {
    const db = await this.getDatabase();
    for (const tile of tiles) {
      const key = `${tile.z}/${tile.x}/${tile.y}`;
      if (this.loadedTiles.has(key)) continue;
      const loaded = await this.loadTileFromDb(db, tile);
      if (loaded) {
        this.loadedTiles.set(key, loaded);
      }
    }
  }

  renderToTexture(encoder: GPUCommandEncoder, target: GPUTexture): void {
    const tileCount = this.loadedTiles.size;
    this.device.queue.writeBuffer(this.tileCountBuffer, 0, new Uint32Array([tileCount]));

    let offset = 0;
    for (const [, tile] of this.loadedTiles) {
      if (offset + tile.byteLength > MAX_TILES * TILE_DATA_BYTES) break;
      if (tile.byteLength > 0) {
        this.device.queue.writeBuffer(this.tileDataBuffer, offset, tile.data as unknown as ArrayBuffer);
      }
      offset += TILE_DATA_BYTES;
    }

    const pass = encoder.beginComputePass();
    pass.setPipeline(this.pipeline);
    pass.setBindGroup(0, this.bindGroup);
    pass.dispatchWorkgroups(Math.ceil(tileCount / 64));
    pass.end();
  }

  geoToScreen(coords: GeoCoord[], outBuffer: GPUBuffer): void {
    const data = new Float32Array(coords.length * 2);
    const zoom = this.viewport.zoom;
    const centerLng = this.viewport.center.lng;
    const centerLat = this.viewport.center.lat;
    const bearingRad = this.viewport.bearing * Math.PI / 180;
    const bearingCos = Math.cos(bearingRad);
    const bearingSin = Math.sin(bearingRad);
    const zoomScale = Math.pow(2, zoom) * TILE_SIZE / 256;
    const cx = webMercatorX(centerLng, zoom);
    const cy = webMercatorY(centerLat, zoom);

    for (let i = 0; i < coords.length; i++) {
      const tx = webMercatorX(coords[i].lng, zoom);
      const ty = webMercatorY(coords[i].lat, zoom);
      const dx = tx - cx;
      const dy = ty - cy;
      const rx = dx * bearingCos - dy * bearingSin;
      const ry = dx * bearingSin + dy * bearingCos;
      data[i * 2] = rx * TILE_SIZE * zoomScale + 960;
      data[i * 2 + 1] = ry * TILE_SIZE * zoomScale + 540;
    }

    this.device.queue.writeBuffer(outBuffer, 0, data);
  }

  setRoute(coords: GeoCoord[]): void {
    this.routeCoords = coords;
  }

  getVisibleTiles(): MapTileKey[] {
    const vp = this.viewport;
    const zoom = Math.round(vp.zoom);
    const centerTile = lonLatToTile(vp.center.lng, vp.center.lat, zoom);
    const tiles: MapTileKey[] = [];
    const range = 2;

    for (let dx = -range; dx <= range; dx++) {
      for (let dy = -range; dy <= range; dy++) {
        const n = Math.pow(2, zoom);
        tiles.push({
          z: zoom,
          x: ((centerTile.x + dx) % n + n) % n,
          y: ((centerTile.y + dy) % n + n) % n,
        });
      }
    }
    return tiles;
  }

  getTileCoverage(
    bounds: { north: number; south: number; east: number; west: number },
    zoom: number,
  ): MapTileKey[] {
    const z = Math.round(zoom);
    const n = Math.pow(2, z);
    const xMin = Math.max(0, Math.floor(((bounds.west + 180) / 360) * n));
    const xMax = Math.min(n - 1, Math.floor(((bounds.east + 180) / 360) * n));
    const yMin = Math.max(0, Math.floor(webMercatorY(bounds.north, zoom)));
    const yMax = Math.min(n - 1, Math.floor(webMercatorY(bounds.south, zoom)));
    const tiles: MapTileKey[] = [];

    for (let x = xMin; x <= xMax; x++) {
      for (let y = yMin; y <= yMax; y++) {
        tiles.push({ z, x, y });
      }
    }
    return tiles;
  }

  private async getDatabase(): Promise<IDBDatabase> {
    if (this.dbPromise) return this.dbPromise;
    this.dbPromise = new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open('dowiz_geo_cache', 1);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains('tiles')) {
          db.createObjectStore('tiles', { keyPath: 'key' });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
    return this.dbPromise;
  }

  private async loadTileFromDb(db: IDBDatabase, tile: MapTileKey): Promise<LoadedTile | null> {
    return new Promise((resolve) => {
      const tx = db.transaction('tiles', 'readonly');
      const store = tx.objectStore('tiles');
      const key = `${tile.z}/${tile.x}/${tile.y}`;
      const req = store.get(key);
      req.onsuccess = () => {
        if (req.result) {
          const arr = req.result.data as ArrayBuffer;
          const loaded: LoadedTile = {
            key: tile,
            data: this.parseTileData(new Uint8Array(arr)),
            byteOffset: 0,
            byteLength: arr.byteLength,
          };
          resolve(loaded);
        } else {
          resolve(null);
        }
      };
      req.onerror = () => resolve(null);
    });
  }

  private async saveTileToDb(db: IDBDatabase, tile: MapTileKey, loaded: LoadedTile): Promise<void> {
    return new Promise((resolve, reject) => {
      const tx = db.transaction('tiles', 'readwrite');
      const store = tx.objectStore('tiles');
      const key = `${tile.z}/${tile.x}/${tile.y}`;
      const buffer = this.serializeTileData(loaded);
      store.put({ key, data: buffer });
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
  }

  private parseTileData(bytes: Uint8Array): TileDataHeader[] {
    const headers: TileDataHeader[] = [];
    let offset = 0;
    while (offset + 8 <= bytes.length) {
      const type = new Uint32Array(bytes.buffer, offset, 1)[0];
      const count = new Uint32Array(bytes.buffer, offset + 4, 1)[0];
      offset += 8;
      if (count > 0 && offset + count * 8 <= bytes.length) {
        const coords = new Float32Array(bytes.buffer, offset, count * 2);
        headers.push({ type, count, coords });
        offset += count * 8;
      } else {
        break;
      }
    }
    return headers;
  }

  private serializeTileData(tile: LoadedTile): ArrayBuffer {
    const headerSize = tile.data.length * 8;
    let coordsSize = 0;
    for (const h of tile.data) {
      coordsSize += h.count * 8;
    }
    const buf = new ArrayBuffer(headerSize + coordsSize);
    const headerView = new Uint32Array(buf);
    const coordView = new Float32Array(buf);
    let offset = 0;
    let coordOffset = 0;
    for (const h of tile.data) {
      headerView[offset] = h.type;
      headerView[offset + 1] = h.count;
      offset += 2;
      const base = (headerSize / 4) + coordOffset;
      for (let i = 0; i < h.count * 2; i++) {
        coordView[base + i] = h.coords[i];
      }
      coordOffset += h.count * 2;
    }
    return buf;
  }

  private async fetchTileData(tile: MapTileKey): Promise<LoadedTile | null> {
    const url = `https://tile.openstreetmap.org/${tile.z}/${tile.x}/${tile.y}.pbf`;
    try {
      const resp = await fetch(url);
      if (!resp.ok) return null;
      const buf = await resp.arrayBuffer();
      const bytes = new Uint8Array(buf);
      const parsed = this.parseTileData(bytes);
      return {
        key: tile,
        data: parsed,
        byteOffset: 0,
        byteLength: buf.byteLength,
      };
    } catch {
      return null;
    }
  }

  destroy(): void {
    this.viewportUniformBuffer.destroy();
    this.tileDataBuffer.destroy();
    this.tileCountBuffer.destroy();
    this.loadedTiles.clear();
    this.routeCoords = [];
  }
}
