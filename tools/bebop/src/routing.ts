// Bebop routing — the conductor's decision spine (mesh fallback promoted to code).
//
// Governing principle (RESEARCH §1.6): orchestration/rotation and model selection are Bebop's, applied
// uniformly to every backend. This module picks the first healthy, available backend from the
// profile's rotation order, and rotates to the next on failure. It does NOT know about any one CLI's
// internals — that lives in backend.ts.

import { ADAPTERS, isAvailable, healthProbe, type Backend } from './backend.ts';
import type { Profile } from './profile.ts';
import { route, type TaskClass, type Model } from './router.ts';

export interface Selected {
  backend: Backend;
  model: Model;
}

/**
 * Choose a backend for a task.
 *  - Applies the token router to pick the model lane (Bebop decides model, not the backend).
 *  - Walks the profile's backendOrder, skipping any backend that is not installed/available.
 *  - Ensures `native` is always a last resort so Bebop never hard-fails with zero executors.
 * Returns null only if even `native` is somehow unreachable (it can't be — detect() is always true).
 */
export function selectBackend(profile: Profile, taskClass: TaskClass = 'doer'): Selected | null {
  const model = route(taskClass).model;
  for (const b of profile.backendOrder) {
    if (b === 'native') return { backend: 'native', model }; // guaranteed fallback
    if (isAvailable(b)) return { backend: b, model };
  }
  return { backend: 'native', model };
}

/**
 * Rotate after a failure: return the next backend in the order after `failed`, that is available.
 * Uniform across all backends — no special-casing (RESEARCH §1.6).
 */
export function rotate(profile: Profile, failed: Backend): Selected | null {
  const idx = profile.backendOrder.indexOf(failed);
  const rest = profile.backendOrder.slice(idx + 1);
  for (const b of rest) {
    if (b === 'native') return { backend: 'native', model: route('doer').model };
    if (isAvailable(b)) return { backend: b, model: route('doer').model };
  }
  return { backend: 'native', model: route('doer').model };
}

/** Probe every backend once and report liveness — used by `bebop status`. */
export function probeAll(profile: Profile): { backend: Backend; installed: boolean; available: boolean; alive: boolean }[] {
  return profile.backendOrder.map((b) => ({
    backend: b,
    installed: ADAPTERS[b].detect(),
    available: isAvailable(b),
    alive: healthProbe(b),
  }));
}
