import type { PointerData, PointerType } from './types.ts';

export type PointerHandlerCallbacks = {
  onPointerDown?: (data: PointerData) => void;
  onPointerMove?: (data: PointerData) => void;
  onPointerUp?: (data: PointerData) => void;
  onPressureChange?: (id: number, pressure: number, data: PointerData) => void;
};

export class PointerHandler {
  private pointers: Map<number, PointerData>;
  private lastTimestamps: Map<number, DOMHighResTimeStamp>;
  private callbacks: PointerHandlerCallbacks;
  private downHandler: (e: PointerEvent) => void;
  private moveHandler: (e: PointerEvent) => void;
  private upHandler: (e: PointerEvent) => void;
  private leaveHandler: (e: PointerEvent) => void;
  private cancelHandler: (e: PointerEvent) => void;
  private element: EventTarget;

  constructor(element: EventTarget, callbacks: PointerHandlerCallbacks) {
    this.element = element;
    this.callbacks = callbacks;
    this.pointers = new Map();
    this.lastTimestamps = new Map();

    this.downHandler = this.handlePointerDown.bind(this);
    this.moveHandler = this.handlePointerMove.bind(this);
    this.upHandler = this.handlePointerUp.bind(this);
    this.leaveHandler = this.handlePointerUp.bind(this);
    this.cancelHandler = this.handlePointerUp.bind(this);

    element.addEventListener('pointerdown', this.downHandler);
    element.addEventListener('pointermove', this.moveHandler);
    element.addEventListener('pointerup', this.upHandler);
    element.addEventListener('pointerleave', this.leaveHandler);
    element.addEventListener('pointercancel', this.cancelHandler);
  }

  private detectPointerType(e: PointerEvent): PointerType {
    if (e.pointerType === 'mouse') return 'mouse';
    if (e.pointerType === 'touch') return 'touch';
    if (e.pointerType === 'pen') return 'pen';
    return 'mouse';
  }

  private resolvePressure(e: PointerEvent, pointerType: PointerType): number {
    if (e.pressure > 0) return Math.min(e.pressure, 1.0);
    if (pointerType === 'mouse') return e.buttons > 0 ? 0.5 : 0;
    return 0.5;
  }

  private calculateVelocity(
    current: [number, number],
    previous: [number, number],
    dt: number,
  ): [number, number] {
    if (dt < 1e-6) return [0, 0];
    return [
      (current[0] - previous[0]) / dt,
      (current[1] - previous[1]) / dt,
    ];
  }

  private handlePointerDown(e: PointerEvent): void {
    e.preventDefault();
    const pointerType = this.detectPointerType(e);
    const pressure = this.resolvePressure(e, pointerType);

    const data: PointerData = {
      id: e.pointerId,
      position: [e.clientX, e.clientY],
      previousPosition: [e.clientX, e.clientY],
      velocity: [0, 0],
      pressure,
      displacement: 0,
      pointerType,
      active: true,
      timestamp: e.timeStamp,
    };

    this.pointers.set(e.pointerId, data);
    this.lastTimestamps.set(e.pointerId, e.timeStamp);
    this.callbacks.onPointerDown?.(data);
  }

  private handlePointerMove(e: PointerEvent): void {
    const existing = this.pointers.get(e.pointerId);
    if (!existing) return;

    const lastTs = this.lastTimestamps.get(e.pointerId) ?? e.timeStamp;
    const dt = Math.max((e.timeStamp - lastTs) / 1000, 1e-6);
    this.lastTimestamps.set(e.pointerId, e.timeStamp);

    const previousPressure = existing.pressure;
    const pressure = this.resolvePressure(e, existing.pointerType);

    existing.previousPosition = existing.position;
    existing.position = [e.clientX, e.clientY];
    existing.velocity = this.calculateVelocity(existing.position, existing.previousPosition, dt);
    existing.pressure = pressure;
    existing.timestamp = e.timeStamp;

    this.callbacks.onPointerMove?.(existing);

    if (Math.abs(pressure - previousPressure) > 0.01) {
      this.callbacks.onPressureChange?.(e.pointerId, pressure, existing);
    }
  }

  private handlePointerUp(e: PointerEvent): void {
    const data = this.pointers.get(e.pointerId);
    if (!data) return;

    data.active = false;
    data.pressure = 0;
    data.timestamp = e.timeStamp;
    this.callbacks.onPointerUp?.(data);
    this.pointers.delete(e.pointerId);
    this.lastTimestamps.delete(e.pointerId);
  }

  getPointer(id: number): PointerData | undefined {
    return this.pointers.get(id);
  }

  getAllPointers(): Map<number, PointerData> {
    return new Map(this.pointers);
  }

  getActivePointerCount(): number {
    return this.pointers.size;
  }

  hasPointer(id: number): boolean {
    return this.pointers.has(id);
  }

  destroy(): void {
    this.element.removeEventListener('pointerdown', this.downHandler);
    this.element.removeEventListener('pointermove', this.moveHandler);
    this.element.removeEventListener('pointerup', this.upHandler);
    this.element.removeEventListener('pointerleave', this.leaveHandler);
    this.element.removeEventListener('pointercancel', this.cancelHandler);
    this.pointers.clear();
    this.lastTimestamps.clear();
  }
}
