import { useCallback, useReducer, useRef } from 'react';

// Generic bounded undo/redo history for a piece of client state.
//
// The pure core (historyInit / historyPush / historyUndo / historyRedo) carries all
// of the logic and is unit-tested with plain values (use-history-stack.test.ts —
// tsx --test, no DOM). The hook is a thin wrapper that keeps the HistoryState in a
// ref so undo()/redo() can return the restored value SYNCHRONOUSLY (a setState
// updater cannot), with stable callback identities safe for effect deps.
//
// Intended for CLIENT DRAFT state only (form buffers, local editors) — never for
// server state: undoing must not resurrect anything the server already persisted.

export interface HistoryState<T> {
  past: T[];
  present: T;
  future: T[];
}

export interface HistoryOptions<T> {
  /** Max undo depth (bound on `past`). Oldest entries are dropped. Default 50, clamped to ≥1. */
  limit?: number;
  /**
   * Equality used to drop no-op pushes (e.g. a snapshot effect re-firing with
   * unchanged values). Default Object.is — pass a structural comparator when T
   * is rebuilt per render.
   */
  isEqual?: (a: T, b: T) => boolean;
}

export const DEFAULT_HISTORY_LIMIT = 50;

export function historyInit<T>(present: T): HistoryState<T> {
  return { past: [], present, future: [] };
}

/**
 * Record `next` as the new present. No-op (returns the SAME state object) when
 * `next` equals the current present. Truncates the redo branch and bounds `past`.
 */
export function historyPush<T>(state: HistoryState<T>, next: T, options: HistoryOptions<T> = {}): HistoryState<T> {
  const isEqual = options.isEqual ?? Object.is;
  if (isEqual(state.present, next)) return state;
  const limit = Math.max(1, options.limit ?? DEFAULT_HISTORY_LIMIT);
  const past = [...state.past, state.present];
  return {
    past: past.length > limit ? past.slice(past.length - limit) : past,
    present: next,
    future: [],
  };
}

/** Step back one entry. No-op (same state object) when there is nothing to undo. */
export function historyUndo<T>(state: HistoryState<T>): HistoryState<T> {
  if (state.past.length === 0) return state;
  return {
    past: state.past.slice(0, -1),
    present: state.past[state.past.length - 1] as T,
    future: [state.present, ...state.future],
  };
}

/** Step forward one entry. No-op (same state object) when there is nothing to redo. */
export function historyRedo<T>(state: HistoryState<T>): HistoryState<T> {
  if (state.future.length === 0) return state;
  return {
    past: [...state.past, state.present],
    present: state.future[0] as T,
    future: state.future.slice(1),
  };
}

export interface UseHistoryStack<T> {
  /** Current value (the history's `present`). */
  state: T;
  /** Record a new value (accepts a functional update). Equal values are dropped. */
  set: (next: T | ((prev: T) => T)) => void;
  /** Step back; returns the restored value, or undefined when there was nothing to undo. */
  undo: () => T | undefined;
  /** Step forward; returns the restored value, or undefined when there was nothing to redo. */
  redo: () => T | undefined;
  canUndo: boolean;
  canRedo: boolean;
  /** Clear all history and start over at `next` (defaults to the hook's initial value). */
  reset: (next?: T) => void;
}

export function useHistoryStack<T>(initial: T, options: HistoryOptions<T> = {}): UseHistoryStack<T> {
  const initialRef = useRef(initial);
  const historyRef = useRef<HistoryState<T>>(historyInit(initial));
  const optionsRef = useRef(options);
  optionsRef.current = options;
  const [, forceRender] = useReducer((c: number) => c + 1, 0);

  const set = useCallback((next: T | ((prev: T) => T)) => {
    const current = historyRef.current;
    const value = typeof next === 'function' ? (next as (prev: T) => T)(current.present) : next;
    const updated = historyPush(current, value, optionsRef.current);
    if (updated !== current) {
      historyRef.current = updated;
      forceRender();
    }
  }, []);

  const undo = useCallback((): T | undefined => {
    const current = historyRef.current;
    if (current.past.length === 0) return undefined;
    historyRef.current = historyUndo(current);
    forceRender();
    return historyRef.current.present;
  }, []);

  const redo = useCallback((): T | undefined => {
    const current = historyRef.current;
    if (current.future.length === 0) return undefined;
    historyRef.current = historyRedo(current);
    forceRender();
    return historyRef.current.present;
  }, []);

  const reset = useCallback((next?: T) => {
    // Matches the declared `reset(next?: T)` signature exactly (a rest-tuple param here
    // previously type-checked against its own interface but failed `tsc` — see
    // use-history-stack.test.ts for the pure-function coverage). No caller passes an
    // explicit `undefined` today, so falling back to the initial value in that case too
    // is the correct, simplest behavior.
    historyRef.current = historyInit(next !== undefined ? next : initialRef.current);
    forceRender();
  }, []);

  return {
    state: historyRef.current.present,
    set,
    undo,
    redo,
    canUndo: historyRef.current.past.length > 0,
    canRedo: historyRef.current.future.length > 0,
    reset,
  };
}
