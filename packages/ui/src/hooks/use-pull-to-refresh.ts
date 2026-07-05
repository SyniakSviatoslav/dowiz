import { useCallback, useRef, useState } from 'react';
import type { TouchEvent as ReactTouchEvent } from 'react';
import { useHaptics } from './use-haptics.js';

// ── PULL-TO-REFRESH ──────────────────────────────────────────────────────────
// Touch-only "pull down at the top of a list to refetch" gesture. Mouse input
// never reaches these handlers (they are onTouch* only), so desktop is a no-op
// by construction. The gesture arms ONLY when the nearest scrollable ancestor
// of the touch sits at scrollTop <= 0 — a downward drag mid-list is scrolling,
// never a refresh. Content translation is the CONSUMER's choice via
// `pullDistance`; under prefers-reduced-motion the hook reports 0 distance so
// any transform stays inert while progress/refreshing still drive a static
// indicator.
//
// NOTE (React ≥17): root-registered touchmove listeners are passive, so this
// hook deliberately never calls preventDefault(). Native scroll cannot fight
// the gesture because it only arms at scrollTop 0; scroll-chaining to the
// document (Chrome's native page-reload PTR) is suppressed by
// `overscroll-behavior-y: contain` on the app-shell scroll container.

/** Resisted distance (px) at which the release triggers a refresh. */
export const PULL_THRESHOLD_PX = 70;
/** Damping factor applied to raw finger travel — the "heavy" rubber feel. */
export const PULL_RESISTANCE = 0.5;
/** Hard cap on the resisted distance so a huge drag can't drag content off. */
export const MAX_PULL_PX = 140;

/**
 * Pure: raw finger travel → dampened pull distance in px.
 * Non-positive / NaN deltas → 0; capped at `maxPull`.
 */
export function applyPullResistance(
  deltaY: number,
  resistance: number = PULL_RESISTANCE,
  maxPull: number = MAX_PULL_PX,
): number {
  if (!Number.isFinite(deltaY) || deltaY <= 0) return 0;
  return Math.min(deltaY * resistance, maxPull);
}

/**
 * Pure: raw finger travel → arming progress in [0, 1].
 * 1 means "release now refreshes". Degenerate thresholds → 0 (never NaN/∞).
 */
export function computePullProgress(
  deltaY: number,
  threshold: number = PULL_THRESHOLD_PX,
): number {
  if (!Number.isFinite(threshold) || threshold <= 0) return 0;
  return Math.min(applyPullResistance(deltaY) / threshold, 1);
}

/**
 * Pure scroll guard: the gesture may only start at the very top of the
 * scroll container (iOS rubber-band reports negative scrollTop → still top)
 * AND with a downward drag.
 */
export function shouldActivatePull(scrollTop: number, deltaY: number): boolean {
  return scrollTop <= 0 && deltaY > 0;
}

function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
  try {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return false;
  }
}

/** Nearest vertically-scrollable ancestor of the touch target (else the document scroller). */
function findScrollContainer(start: Element | null): Element | null {
  let el: Element | null = start;
  while (el && el !== document.documentElement) {
    const { overflowY } = getComputedStyle(el);
    if ((overflowY === 'auto' || overflowY === 'scroll') && el.scrollHeight > el.clientHeight) {
      return el;
    }
    el = el.parentElement;
  }
  return document.scrollingElement;
}

export interface PullToRefreshHandlers {
  onTouchStart: (e: ReactTouchEvent<HTMLElement>) => void;
  onTouchMove: (e: ReactTouchEvent<HTMLElement>) => void;
  onTouchEnd: () => void;
  onTouchCancel: () => void;
}

export interface UsePullToRefreshOptions {
  /** Existing refetch of the surface — awaited; errors are swallowed (the page's own error state owns them). */
  onRefresh: () => Promise<unknown> | unknown;
  /** Kill-switch (e.g. feature flag off) — handlers become inert. */
  disabled?: boolean;
  /** Resisted px needed to arm the refresh. */
  threshold?: number;
}

export interface UsePullToRefreshResult {
  handlers: PullToRefreshHandlers;
  /** A pull gesture is currently active. */
  pulling: boolean;
  /** 0..1 arming progress (1 = release will refresh). */
  progress: number;
  /** onRefresh is in flight. */
  refreshing: boolean;
  /** Resisted pull distance in px (always 0 under prefers-reduced-motion). */
  pullDistance: number;
}

export function usePullToRefresh({
  onRefresh,
  disabled = false,
  threshold = PULL_THRESHOLD_PX,
}: UsePullToRefreshOptions): UsePullToRefreshResult {
  const [pulling, setPulling] = useState(false);
  const [progress, setProgress] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const [pullDistance, setPullDistance] = useState(0);
  const { trigger: haptic } = useHaptics();

  const startYRef = useRef(0);
  const startXRef = useRef(0);
  const scrollElRef = useRef<Element | null>(null);
  const eligibleRef = useRef(false);
  const axisLockedRef = useRef<null | 'x' | 'y'>(null);
  const armedHapticRef = useRef(false);
  const progressRef = useRef(0);
  const refreshingRef = useRef(false);

  const reset = useCallback(() => {
    eligibleRef.current = false;
    axisLockedRef.current = null;
    armedHapticRef.current = false;
    progressRef.current = 0;
    setPulling(false);
    setProgress(0);
    setPullDistance(0);
  }, []);

  const onTouchStart = useCallback((e: ReactTouchEvent<HTMLElement>) => {
    if (disabled || refreshingRef.current) return;
    if (e.touches.length !== 1) { eligibleRef.current = false; return; }
    const target = e.target as Element | null;
    // A pull inside an open dialog/sheet must never refresh the page behind it.
    if (target?.closest?.('[aria-modal="true"], [role="dialog"]')) { eligibleRef.current = false; return; }
    const touch = e.touches[0];
    if (!touch) return;
    scrollElRef.current = findScrollContainer(target);
    startYRef.current = touch.clientY;
    startXRef.current = touch.clientX;
    axisLockedRef.current = null;
    armedHapticRef.current = false;
    // Provisional — re-checked on every move so a scroll that starts mid-list stays a scroll.
    eligibleRef.current = (scrollElRef.current?.scrollTop ?? 0) <= 0;
  }, [disabled]);

  const onTouchMove = useCallback((e: ReactTouchEvent<HTMLElement>) => {
    if (disabled || refreshingRef.current || !eligibleRef.current) return;
    const touch = e.touches[0];
    if (!touch || e.touches.length !== 1) { reset(); return; }
    const deltaY = touch.clientY - startYRef.current;
    const deltaX = touch.clientX - startXRef.current;

    // Axis lock on first meaningful movement: a horizontal swipe (category
    // chips, carousels) must never hijack into a refresh.
    if (axisLockedRef.current === null && (Math.abs(deltaX) > 6 || Math.abs(deltaY) > 6)) {
      axisLockedRef.current = Math.abs(deltaX) > Math.abs(deltaY) ? 'x' : 'y';
    }
    if (axisLockedRef.current === 'x') { reset(); return; }

    const scrollTop = scrollElRef.current?.scrollTop ?? 0;
    if (!shouldActivatePull(scrollTop, deltaY)) {
      // Dragged back up past the origin or the container scrolled — disarm visuals
      // but keep the gesture watchable (the finger may come back down at top).
      if (scrollTop > 0) { reset(); return; }
      progressRef.current = 0;
      setPulling(false);
      setProgress(0);
      setPullDistance(0);
      return;
    }

    const p = computePullProgress(deltaY, threshold);
    progressRef.current = p;
    setPulling(true);
    setProgress(p);
    setPullDistance(prefersReducedMotion() ? 0 : applyPullResistance(deltaY));
    if (p >= 1 && !armedHapticRef.current) {
      armedHapticRef.current = true;
      haptic('tap'); // light "armed" tick — release will refresh
    }
  }, [disabled, threshold, haptic, reset]);

  const onTouchEnd = useCallback(() => {
    const armed = progressRef.current >= 1;
    reset();
    if (disabled || refreshingRef.current || !armed) return;
    refreshingRef.current = true;
    setRefreshing(true);
    Promise.resolve()
      .then(() => onRefresh())
      .catch(() => { /* surface-level error handling belongs to the page's own fetch */ })
      .finally(() => {
        refreshingRef.current = false;
        setRefreshing(false);
      });
  }, [disabled, onRefresh, reset]);

  const onTouchCancel = useCallback(() => { reset(); }, [reset]);

  return {
    handlers: { onTouchStart, onTouchMove, onTouchEnd, onTouchCancel },
    pulling,
    progress,
    refreshing,
    pullDistance,
  };
}
