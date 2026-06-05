interface Point {
  lat: number;
  lng: number;
}

function onSegment(p: Point, q: Point, r: Point): boolean {
  return (
    q.lat <= Math.max(p.lat, r.lat) &&
    q.lat >= Math.min(p.lat, r.lat) &&
    q.lng <= Math.max(p.lng, r.lng) &&
    q.lng >= Math.min(p.lng, r.lng)
  );
}

function orientation(p: Point, q: Point, r: Point): number {
  const val = (q.lng - p.lng) * (r.lat - q.lat) - (q.lat - p.lat) * (r.lng - q.lng);
  if (val === 0) return 0;
  return val > 0 ? 1 : 2;
}

function doIntersect(p1: Point, q1: Point, p2: Point, q2: Point): boolean {
  const o1 = orientation(p1, q1, p2);
  const o2 = orientation(p1, q1, q2);
  const o3 = orientation(p2, q2, p1);
  const o4 = orientation(p2, q2, q1);

  if (o1 !== o2 && o3 !== o4) return true;
  if (o1 === 0 && onSegment(p1, p2, q1)) return true;
  if (o2 === 0 && onSegment(p1, q2, q1)) return true;
  if (o3 === 0 && onSegment(p2, p1, q2)) return true;
  if (o4 === 0 && onSegment(p2, q1, q2)) return true;

  return false;
}

/**
 * Ray-casting algorithm to check if a point is inside a polygon.
 */
export function checkDeliveryZone(point: Point, polygon: Point[]): boolean {
  const n = polygon.length;
  if (n < 3) return false;

  const extreme: Point = { lat: 1e9, lng: point.lng };
  let count = 0;

  for (let i = 0; i < n; i++) {
    const next = (i + 1) % n;
    const pi = polygon[i]!;
    const pn = polygon[next]!;
    if (doIntersect(pi, pn, point, extreme)) {
      if (orientation(pi, point, pn) === 0) {
        return onSegment(pi, point, pn);
      }
      count++;
    }
  }

  return count % 2 === 1;
}
