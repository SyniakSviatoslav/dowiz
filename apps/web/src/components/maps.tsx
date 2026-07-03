import { lazy, Suspense, type ComponentProps } from 'react';
import { SkeletonBase } from '@deliveryos/ui';
// Type-only import — erased at build time, so it creates NO static edge to the
// map chunk. Runtime code reaches the maps only through the dynamic import below.
import type {
  CourierLiveMap as CourierLiveMapImpl,
  MapWithPin as MapWithPinImpl,
  MapWithRadius as MapWithRadiusImpl,
} from '@deliveryos/ui/dist/maps.js';

// Perf seam (UI-PERF roadmap 2.2 follow-up / product-media-OPERATOR-ENABLEMENT
// "Bundle: lazy-load the 1MB map"): every map consumer in apps/web imports from
// THIS module instead of the '@deliveryos/ui' barrel. The barrel import was a
// static edge that pulled the map component code into each route chunk and
// mounted MapLibreBase (whose effect fetches the ~1MB maplibre chunk) on every
// view. React.lazy defers both until a map actually renders.
//
// The specifier '@deliveryos/ui/dist/maps.js' typechecks against the package's
// built declarations; at bundle time a dedicated vite alias points it at
// packages/ui/src/maps.ts (see apps/web/vite.config.ts).
const loadMaps = () => import('@deliveryos/ui/dist/maps.js');

const LazyCourierLiveMap = lazy(() => loadMaps().then(m => ({ default: m.CourierLiveMap })));
const LazyMapWithPin = lazy(() => loadMaps().then(m => ({ default: m.MapWithPin })));
const LazyMapWithRadius = lazy(() => loadMaps().then(m => ({ default: m.MapWithRadius })));

// Skeleton sized like the map it replaces (className carries the height/width box).
function MapFallback({ className }: { className?: string }) {
  return <SkeletonBase className={className || 'h-64 w-full'} />;
}

export function CourierLiveMap(props: ComponentProps<typeof CourierLiveMapImpl>) {
  return (
    <Suspense fallback={<MapFallback className={props.className} />}>
      <LazyCourierLiveMap {...props} />
    </Suspense>
  );
}

export function MapWithPin(props: ComponentProps<typeof MapWithPinImpl>) {
  return (
    <Suspense fallback={<MapFallback className={props.className} />}>
      <LazyMapWithPin {...props} />
    </Suspense>
  );
}

export function MapWithRadius(props: ComponentProps<typeof MapWithRadiusImpl>) {
  return (
    <Suspense fallback={<MapFallback className={props.className} />}>
      <LazyMapWithRadius {...props} />
    </Suspense>
  );
}
