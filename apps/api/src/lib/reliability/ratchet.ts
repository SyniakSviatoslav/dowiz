// Degrade-storm ratchet (trap #15).
//
// 2026-07-05 incident: a Node restart auto-degraded ALL non-money surfaces
// silently while Rust was healthy; the degrade flags PERSISTED in a store and
// the trap re-armed on every boot. The ratchet has three parts:
//   1. boot-grace — no persistent degrade decision before the first successful
//      health cycle OR a bounded grace deadline.
//   2. real alert — when a real degrade happens, emit a bus event + Sentry.
//   3. restart-regression proof — boot flags RESET on restart (the persisted
//      flag trap is disarmed), so a fresh process starts clean.
//
// RATCHET_RESET_ON_BOOT + boot-grace are wired here; server.ts calls
// resetBootFlags() at startup and injects the messageBus as the alert sink.
//
// NOTE: this module is intentionally dependency-free (no DB/Sentry import) so
// the restart-regression test runs without a live database.

export const BOOT_GRACE_MS = Number(process.env.BOOT_GRACE_MS ?? 120_000);

// Non-money surfaces that trap #15 flipped (S1–S4, S6, S10). Money surfaces are
// REFUSED by design (REV-C5) and are never part of this set.
export const DEGRADE_FLAG_KEYS = [
  'S1', 'S2', 'S3', 'S4', 'S6', 'S10',
] as const;
export type DegradeFlagKey = (typeof DEGRADE_FLAG_KEYS)[number];

export interface RatchetAlertSink {
  publish(channel: string, msg: any): Promise<void>;
}

export interface BootRatchetDeps {
  instanceId?: string;
  alertSink?: RatchetAlertSink | null;
  onAlert?: (info: { surface: string; reason: string; channel: string }) => void;
  /** Defaults to true. When true, degrade() is suppressed during boot-grace. */
  resetOnBoot?: boolean;
}

/**
 * Backend abstraction for the persisted degrade-flag store. The bug: a previous
 * run left flags=true and the store was READ on boot (so they persisted). The
 * fix: resetAll() is called at startup, before any health cycle.
 */
export interface FlagBackend {
  read(): Record<string, boolean>;
  write(flags: Record<string, boolean>): void;
}

export class DegradeFlagStore {
  constructor(private readonly backend: FlagBackend) {}

  readAll(): Record<string, boolean> {
    return this.backend.read();
  }

  /** The restart-regression fix: a fresh boot starts with EVERY surface clean. */
  resetAll(): void {
    const clean: Record<string, boolean> = {};
    for (const k of DEGRADE_FLAG_KEYS) clean[k] = false;
    this.backend.write(clean);
  }

  writeAll(flags: Record<string, boolean>): void {
    this.backend.write(flags);
  }
}

export class BootRatchet {
  readonly bootAt: number;
  private degraded = false;
  private firstOkAt: number | null = null;
  private readonly instanceId: string;
  private readonly sink: RatchetAlertSink | null;
  private readonly onAlert?: BootRatchetDeps['onAlert'];
  private readonly resetOnBoot: boolean;

  constructor(deps: BootRatchetDeps = {}) {
    this.bootAt = Date.now();
    this.instanceId = deps.instanceId || process.env.FLY_MACHINE_ID || process.env.HOSTNAME || 'local';
    this.sink = deps.alertSink ?? null;
    this.onAlert = deps.onAlert;
    this.resetOnBoot = deps.resetOnBoot ?? true;
  }

  /** True until the first successful probe OR the grace deadline elapses. */
  get inBootGrace(): boolean {
    if (this.firstOkAt !== null) return false;
    return Date.now() - this.bootAt < BOOT_GRACE_MS;
  }

  /** Successful upstream health probe — exits boot grace. */
  recordOk(): void {
    if (this.firstOkAt === null) this.firstOkAt = Date.now();
  }

  /** Persistent degrade decisions are only permitted outside boot-grace. */
  canDegrade(): boolean {
    if (!this.resetOnBoot) return true; // RED baseline: grace ignored
    return !this.inBootGrace;
  }

  /**
   * Record a degrade decision. During boot-grace it is SUPPRESSED (returns
   * false, no flag written). Otherwise it flips the flag, alerts, returns true.
   */
  async degrade(surface: string, reason: string): Promise<boolean> {
    if (this.resetOnBoot) {
      if (this.inBootGrace) {
        console.warn(`[Ratchet] boot-grace active — SUPPRESSING degrade of "${surface}" (${reason})`);
        return false;
      }
    }
    this.degraded = true;
    const channel = 'ops.cutover_degrade';
    this.onAlert?.({ surface, reason, channel });
    console.error(`[Ratchet] DEGRADE ${surface}: ${reason}`);
    if (this.sink) {
      try {
        await this.sink.publish(channel, {
          surface,
          reason,
          instanceId: this.instanceId,
          at: new Date().toISOString(),
        });
      } catch (err) {
        console.error('[Ratchet] alert publish failed:', (err as Error).message);
      }
    }
    return true;
  }

  get isDegraded(): boolean {
    return this.degraded;
  }
}

/** Convenience: reset all degrade flags on startup (the restart-regression fix). */
export function resetBootFlags(store: DegradeFlagStore): void {
  store.resetAll();
}
