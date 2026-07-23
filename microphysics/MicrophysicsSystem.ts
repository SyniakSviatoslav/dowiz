import type {
  MicrophysicsConfig, BrandConfig, PhysicsState, PointerData,
  SplatDisplacement, MicrophysicsEvent,
  DisplacementCallback, MicrophysicsEventCallback,
} from './types.ts';
import {
  MAX_POINTERS, MAX_INJECTION_POINTS,
  PHYSICS_STATE_BYTES, SPRING_PARAMS_BYTES,
  TURING_PARAMS_BYTES, INJECTION_POINT_BYTES,
} from './types.ts';
import { PointerHandler } from './PointerHandler.ts';
import { HapticEngine } from './HapticEngine.ts';

type PointerStateRaw = [
  displacement: number, displacement_velocity: number,
  position_x: number, position_y: number,
  velocity_x: number, velocity_y: number,
  pressure: number, target_x: number, target_y: number,
  active: number,
];

function readPointerState(data: Float32Array, index: number): PointerStateRaw {
  const base = index * 10;
  return [
    data[base], data[base + 1], data[base + 2], data[base + 3],
    data[base + 4], data[base + 5], data[base + 6], data[base + 7],
    data[base + 8], data[base + 9],
  ];
}

export class MicrophysicsSystem {
  private device: GPUDevice;
  private config: MicrophysicsConfig;
  private brand: BrandConfig;

  private pointerHandler: PointerHandler;
  private hapticEngine: HapticEngine;

  private springParamsBuffer!: GPUBuffer;
  private physicsStateBuffer!: GPUBuffer;
  private stagingBuffer!: GPUBuffer;
  private uniformDtBuffer!: GPUBuffer;
  private springPipeline!: GPUComputePipeline;
  private springBindGroup!: GPUBindGroup;

  private turingParamsBuffer!: GPUBuffer;
  private turingGridU: [GPUBuffer, GPUBuffer] = null as unknown as [GPUBuffer, GPUBuffer];
  private turingGridV: [GPUBuffer, GPUBuffer] = null as unknown as [GPUBuffer, GPUBuffer];
  private injectionPointBuffer!: GPUBuffer;
  private turingPipeline!: GPUComputePipeline;
  private turingBindGroups!: [GPUBindGroup, GPUBindGroup];
  private turingPing: 0 | 1 = 0;

  private turingGridWidth: number;
  private turingGridHeight: number;

  private displacementCallbacks = new Set<DisplacementCallback>();
  private eventCallbacks = new Set<MicrophysicsEventCallback>();

  private readbackData: Float32Array | null = null;
  private readbackPromise: Promise<void> | null = null;
  private readbackReady = false;

  private previousPressureMap = new Map<number, number>();

  private pointerSlotMap = new Map<number, number>();
  private nextFreeSlot = 0;

  constructor(
    device: GPUDevice,
    config: MicrophysicsConfig,
    brand: BrandConfig,
    element: EventTarget,
  ) {
    this.device = device;
    this.config = config;
    this.brand = brand;

    this.turingGridWidth = config.turing.grid_width;
    this.turingGridHeight = config.turing.grid_height;

    this.hapticEngine = new HapticEngine(config.haptic);
    this.pointerHandler = new PointerHandler(element, {
      onPointerDown: this.onPointerDown,
      onPointerMove: this.onPointerMove,
      onPointerUp: this.onPointerUp,
      onPressureChange: this.onPressureChange,
    });
  }

  static async create(
    device: GPUDevice,
    config: MicrophysicsConfig,
    brand: BrandConfig,
    element: EventTarget,
    springWGSL?: string,
    turingWGSL?: string,
  ): Promise<MicrophysicsSystem> {
    const system = new MicrophysicsSystem(device, config, brand, element);
    await system.init(springWGSL, turingWGSL);
    return system;
  }

  private async init(springWGSL?: string, turingWGSL?: string): Promise<void> {
    const springCode = springWGSL ?? await this.loadShader('wgsl/microphysics.wgsl');
    const turingCode = turingWGSL ?? await this.loadShader('wgsl/turing_pressure.wgsl');

    this.createSpringBuffers();
    this.createSpringPipeline(springCode);
    this.createTuringBuffers();
    this.createTuringPipeline(turingCode);
    this.initTuringGrid();
  }

  private async loadShader(path: string): Promise<string> {
    const base = import.meta?.url
      ? new URL(path, import.meta.url).href
      : `/dowiz/microphysics/${path}`;
    const resp = await fetch(base);
    return resp.text();
  }

  private allocateSlot(pointerId: number): number {
    let slot = this.nextFreeSlot;
    this.nextFreeSlot = (this.nextFreeSlot + 1) % MAX_POINTERS;
    for (let attempt = 0; attempt < MAX_POINTERS; attempt++) {
      let occupied = false;
      for (const s of this.pointerSlotMap.values()) {
        if (s === slot) { occupied = true; break; }
      }
      if (!occupied) break;
      slot = this.nextFreeSlot;
      this.nextFreeSlot = (this.nextFreeSlot + 1) % MAX_POINTERS;
    }
    this.pointerSlotMap.set(pointerId, slot);
    return slot;
  }

  private freeSlot(pointerId: number): void {
    this.pointerSlotMap.delete(pointerId);
  }

  private getSlot(pointerId: number): number {
    return this.pointerSlotMap.get(pointerId) ?? 0;
  }

  private createSpringBuffers(): void {
    this.springParamsBuffer = this.device.createBuffer({
      size: SPRING_PARAMS_BYTES,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    });
    const springParams = new Float32Array([
      this.config.spring.stiffness,
      this.config.spring.damping,
      this.config.spring.max_displacement,
      this.config.spring.rest_position,
    ]);
    new Float32Array(this.springParamsBuffer.getMappedRange()).set(springParams);
    this.springParamsBuffer.unmap();

    const stateBufferSize = MAX_POINTERS * PHYSICS_STATE_BYTES;
    this.physicsStateBuffer = this.device.createBuffer({
      size: stateBufferSize,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC | GPUBufferUsage.COPY_DST,
    });
    this.stagingBuffer = this.device.createBuffer({
      size: stateBufferSize,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });
    this.uniformDtBuffer = this.device.createBuffer({
      size: 4,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
  }

  private createSpringPipeline(code: string): void {
    const shaderModule = this.device.createShaderModule({ code });
    this.springPipeline = this.device.createComputePipeline({
      layout: 'auto',
      compute: { module: shaderModule, entryPoint: 'main' },
    });
    this.springBindGroup = this.device.createBindGroup({
      layout: this.springPipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: this.springParamsBuffer } },
        { binding: 1, resource: { buffer: this.physicsStateBuffer } },
        { binding: 2, resource: { buffer: this.uniformDtBuffer } },
      ],
    });
  }

  private createTuringBuffers(): void {
    this.turingParamsBuffer = this.device.createBuffer({
      size: TURING_PARAMS_BYTES,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    });
    const tp = this.config.turing;
    const turingParams = new Float32Array([
      tp.diffusion_rate_u, tp.diffusion_rate_v,
      tp.feed_rate, tp.kill_rate,
      tp.dt, this.turingGridWidth, this.turingGridHeight,
      tp.injection_strength, tp.injection_radius,
    ]);
    new Float32Array(this.turingParamsBuffer.getMappedRange()).set(turingParams);
    this.turingParamsBuffer.unmap();

    const gridElemSize = 4;
    const gridSize = this.turingGridWidth * this.turingGridHeight * gridElemSize;
    this.turingGridU = [
      this.device.createBuffer({
        size: gridSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      }),
      this.device.createBuffer({
        size: gridSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      }),
    ];
    this.turingGridV = [
      this.device.createBuffer({
        size: gridSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      }),
      this.device.createBuffer({
        size: gridSize, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      }),
    ];

    this.injectionPointBuffer = this.device.createBuffer({
      size: MAX_INJECTION_POINTS * INJECTION_POINT_BYTES,
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    });
  }

  private createTuringPipeline(code: string): void {
    const shaderModule = this.device.createShaderModule({ code });
    this.turingPipeline = this.device.createComputePipeline({
      layout: 'auto',
      compute: { module: shaderModule, entryPoint: 'main' },
    });

    const layout = this.turingPipeline.getBindGroupLayout(0);
    this.turingBindGroups = [
      this.device.createBindGroup({
        layout,
        entries: [
          { binding: 0, resource: { buffer: this.turingParamsBuffer } },
          { binding: 1, resource: { buffer: this.turingGridU[0] } },
          { binding: 2, resource: { buffer: this.turingGridV[0] } },
          { binding: 3, resource: { buffer: this.turingGridU[1] } },
          { binding: 4, resource: { buffer: this.turingGridV[1] } },
          { binding: 5, resource: { buffer: this.injectionPointBuffer } },
        ],
      }),
      this.device.createBindGroup({
        layout,
        entries: [
          { binding: 0, resource: { buffer: this.turingParamsBuffer } },
          { binding: 1, resource: { buffer: this.turingGridU[1] } },
          { binding: 2, resource: { buffer: this.turingGridV[1] } },
          { binding: 3, resource: { buffer: this.turingGridU[0] } },
          { binding: 4, resource: { buffer: this.turingGridV[0] } },
          { binding: 5, resource: { buffer: this.injectionPointBuffer } },
        ],
      }),
    ];
  }

  private initTuringGrid(): void {
    const len = this.turingGridWidth * this.turingGridHeight;
    const uInit = new Float32Array(len);
    const vInit = new Float32Array(len);

    const cx = Math.floor(this.turingGridWidth / 2);
    const cy = Math.floor(this.turingGridHeight / 2);

    for (let y = 0; y < this.turingGridHeight; y++) {
      for (let x = 0; x < this.turingGridWidth; x++) {
        const i = y * this.turingGridWidth + x;
        uInit[i] = 1.0;
        const dx = x - cx;
        const dy = y - cy;
        const dist = Math.sqrt(dx * dx + dy * dy);
        vInit[i] = dist < 8 ? Math.random() * 0.05 : 0.0;
      }
    }

    this.device.queue.writeBuffer(this.turingGridU[0], 0, uInit);
    this.device.queue.writeBuffer(this.turingGridU[1], 0, uInit);
    this.device.queue.writeBuffer(this.turingGridV[0], 0, vInit);
    this.device.queue.writeBuffer(this.turingGridV[1], 0, vInit);
  }

  private onPointerDown = (data: PointerData): void => {
    this.allocateSlot(data.id);
    this.previousPressureMap.set(data.id, data.pressure);

    this.emitEvent({
      type: 'press',
      pointerId: data.id,
      displacement: 0,
      pressure: data.pressure,
      position: data.position,
    });

    this.hapticEngine.trigger({
      pattern: data.pressure > 0.7 ? 'press_heavy' : data.pressure > 0.3 ? 'press_medium' : 'press_light',
      intensity: data.pressure,
      duration_ms: 20,
    });
  };

  private onPointerMove = (data: PointerData): void => {
    this.emitEvent({
      type: 'move',
      pointerId: data.id,
      displacement: data.displacement,
      pressure: data.pressure,
      position: data.position,
    });
  };

  private onPointerUp = (data: PointerData): void => {
    const slot = this.getSlot(data.id);
    this.device.queue.writeBuffer(
      this.physicsStateBuffer,
      slot * PHYSICS_STATE_BYTES + 36,
      new Uint32Array([0]),
    );
    this.freeSlot(data.id);
    this.previousPressureMap.delete(data.id);

    this.emitEvent({
      type: 'release',
      pointerId: data.id,
      displacement: 0,
      pressure: 0,
      position: data.position,
    });

    this.hapticEngine.trigger({
      pattern: 'release',
      intensity: 0.3,
      duration_ms: 5,
    });
  };

  private onPressureChange = (id: number, pressure: number, _data: PointerData): void => {
    const prev = this.previousPressureMap.get(id) ?? 0;
    const delta = pressure - prev;
    this.previousPressureMap.set(id, pressure);

    if (delta > 0.15) {
      this.hapticEngine.trigger({
        pattern: pressure > 0.6 ? 'press_heavy' : 'press_medium',
        intensity: delta,
        duration_ms: Math.round(15 + delta * 30),
      });
    } else if (delta < -0.15 && pressure < 0.3) {
      this.hapticEngine.trigger({
        pattern: 'release',
        intensity: Math.abs(delta),
        duration_ms: 8,
      });
    }
  };

  private writePointerDataToGPU(): void {
    const ptrs = this.pointerHandler.getAllPointers();

    const data = this.readbackData
      ? new Float32Array(this.readbackData)
      : new Float32Array(MAX_POINTERS * 10);

    for (let i = 0; i < MAX_POINTERS; i++) {
      const base = i * 10;
      data[base + 6] = 0;
      data[base + 7] = 0;
      data[base + 8] = 0;
      data[base + 9] = 0;
    }

    for (const [id, ptr] of ptrs) {
      const slot = this.pointerSlotMap.get(id);
      if (slot === undefined) continue;
      const base = slot * 10;
      data[base + 6] = ptr.pressure;
      data[base + 7] = ptr.position[0];
      data[base + 8] = ptr.position[1];
      data[base + 9] = 1;
    }

    this.device.queue.writeBuffer(this.physicsStateBuffer, 0, data);
  }

  private writeInjectionPointsToGPU(): void {
    const ptrs = this.pointerHandler.getAllPointers();
    const data = new Float32Array(MAX_INJECTION_POINTS * 4);

    let idx = 0;
    for (const [id, ptr] of ptrs) {
      if (idx >= MAX_INJECTION_POINTS) break;
      const base = idx * 4;
      data[base] = ptr.position[0];
      data[base + 1] = ptr.position[1];
      data[base + 2] = ptr.pressure;
      data[base + 3] = 1;
      idx++;
    }

    this.device.queue.writeBuffer(this.injectionPointBuffer, 0, data);
  }

  private processReadback(): void {
    if (!this.readbackData) return;

    const displacements = new Map<number, SplatDisplacement>();
    const ptrs = this.pointerHandler.getAllPointers();

    for (const [id] of ptrs) {
      const slot = this.pointerSlotMap.get(id);
      if (slot === undefined) continue;
      const raw = readPointerState(this.readbackData, slot);
      const displacement = raw[0];
      const posX = raw[2];
      const posY = raw[3];

      displacements.set(id, {
        pointerId: id,
        offsetX: posX,
        offsetY: posY,
        displacement,
      });

      const existing = ptrs.get(id);
      if (existing) {
        existing.displacement = displacement;
      }
    }

    for (const cb of this.displacementCallbacks) {
      cb(displacements);
    }
  }

  private scheduleReadback(): void {
    if (this.readbackPromise) return;

    this.readbackPromise = (async () => {
      try {
        await this.device.queue.onSubmittedWorkDone();
        await this.stagingBuffer.mapAsync(GPUMapMode.READ);
        const mapped = this.stagingBuffer.getMappedRange();
        this.readbackData = new Float32Array(mapped.slice(0));
        this.stagingBuffer.unmap();
        this.readbackReady = true;
      } finally {
        this.readbackPromise = null;
      }
    })();
  }

  update(dt: number): void {
    if (this.readbackReady) {
      this.processReadback();
      this.readbackReady = false;
      this.readbackData = null;
    }

    this.device.queue.writeBuffer(this.uniformDtBuffer, 0, new Float32Array([dt]));

    this.writePointerDataToGPU();
    this.writeInjectionPointsToGPU();

    const encoder = this.device.createCommandEncoder();

    const springPass = encoder.beginComputePass();
    springPass.setPipeline(this.springPipeline);
    springPass.setBindGroup(0, this.springBindGroup);
    springPass.dispatchWorkgroups(Math.ceil(MAX_POINTERS / 64));
    springPass.end();

    encoder.copyBufferToBuffer(
      this.physicsStateBuffer, 0,
      this.stagingBuffer, 0,
      MAX_POINTERS * PHYSICS_STATE_BYTES,
    );

    const turingPass = encoder.beginComputePass();
    turingPass.setPipeline(this.turingPipeline);
    turingPass.setBindGroup(0, this.turingBindGroups[this.turingPing]);
    turingPass.dispatchWorkgroups(
      Math.ceil(this.turingGridWidth / 8),
      Math.ceil(this.turingGridHeight / 8),
    );
    turingPass.end();

    this.turingPing = this.turingPing === 0 ? 1 : 0;

    this.device.queue.submit([encoder.finish()]);

    this.scheduleReadback();
  }

  getDisplacements(): Map<number, SplatDisplacement> {
    const result = new Map<number, SplatDisplacement>();
    if (!this.readbackData) return result;

    const ptrs = this.pointerHandler.getAllPointers();
    for (const [id] of ptrs) {
      const slot = this.pointerSlotMap.get(id);
      if (slot === undefined) continue;
      const raw = readPointerState(this.readbackData, slot);
      result.set(id, {
        pointerId: id,
        offsetX: raw[2],
        offsetY: raw[3],
        displacement: raw[0],
      });
    }
    return result;
  }

  getTuringGridU(): Float32Array | null {
    return null;
  }

  getPointerState(id: number): PhysicsState | null {
    if (!this.readbackData) return null;
    const slot = this.pointerSlotMap.get(id);
    if (slot === undefined) return null;
    const raw = readPointerState(this.readbackData, slot);
    return {
      displacement: raw[0],
      displacement_velocity: raw[1],
      position_x: raw[2],
      position_y: raw[3],
      velocity_x: raw[4],
      velocity_y: raw[5],
      pressure: raw[6],
      target_x: raw[7],
      target_y: raw[8],
      active: raw[9],
    };
  }

  onDisplacement(cb: DisplacementCallback): () => void {
    this.displacementCallbacks.add(cb);
    return () => this.displacementCallbacks.delete(cb);
  }

  onEvent(cb: MicrophysicsEventCallback): () => void {
    this.eventCallbacks.add(cb);
    return () => this.eventCallbacks.delete(cb);
  }

  private emitEvent(event: MicrophysicsEvent): void {
    for (const cb of this.eventCallbacks) {
      cb(event);
    }
  }

  getConfig(): MicrophysicsConfig {
    return { ...this.config };
  }

  destroy(): void {
    this.pointerHandler.destroy();
    this.hapticEngine.destroy();
    this.displacementCallbacks.clear();
    this.eventCallbacks.clear();
    this.previousPressureMap.clear();
    this.pointerSlotMap.clear();
    this.readbackPromise = null;
    this.readbackData = null;
    this.readbackReady = false;
  }
}
