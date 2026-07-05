/**
 * Cutover surface matcher (REV-C2 / breaker CRIT-1 fix).
 *
 * PROBLEM THIS REPLACES: the original proposal (proposal.md §4) routed on
 * `(method, prefix)`, longest-prefix wins. Ground truth (breaker-findings.md CRIT-1)
 * showed that five distinct surfaces (S3 catalog, S5 orders/money, S7 courier/dispatch,
 * S8 jobs/notifications, S9 GDPR/erase) all live under the SAME prefix
 * `/api/owner/locations/:locationId/*` — the discriminator is an INFIX *after* a
 * variable UUID segment (`.../orders/...` vs `.../gdpr-requests/...`), which no
 * longest-prefix rule can express. Any longest-prefix router collapses all five
 * families into whichever rule owns the shared prefix string — e.g. flipping S5
 * (money) would silently co-flip S9 (irreversible GDPR erase). That is the exact
 * failure this module exists to make structurally impossible.
 *
 * DESIGN: match on a FULL (method, path-template) tuple, never a prefix.
 *   - Every template is either an exact literal segment, a `:param` (matches exactly
 *     one non-empty path segment, never crosses a `/`), or a trailing `*` (matches
 *     one-or-more remaining segments — used only for the two genuine wildcard-proxy
 *     routes, `/images/*` and `/media/*`).
 *   - A request that matches ZERO templates fails CLOSED to Node (`NODE_UNMAPPED`).
 *     This is a design invariant, not a bug: an unrecognized/typo'd/future path must
 *     never guess a surface.
 *   - A request that matches MORE THAN ONE template (should be structurally
 *     impossible given a disjoint template set — see cutover-matcher.test.ts's
 *     `every template pair is pairwise non-colliding` invariant test) ALSO fails
 *     CLOSED to Node, rather than silently picking one. This is the runtime
 *     backstop for "never silently co-flip S5 with S9-erase" even if a future
 *     template edit introduces an accidental overlap that the static test missed.
 *
 * WS (S6) IS NOT A (method, path) MATCH AT ALL — see `isWebSocketUpgrade` below and
 * the "S6 is not a path-routable surface" finding in route-surface-map.generated.md.
 * `apps/api/src/websocket.ts:192` instantiates `new WebSocketServer({ server: fastify.server })`
 * with NO `path` option, so the `ws` package intercepts every HTTP Upgrade request on
 * the shared server regardless of URL. A path-template matcher has nothing to match
 * against for WS; the correct discriminator is protocol-level (the `Upgrade` header),
 * checked BEFORE path-template matching, via `matchSurfaceForRequest`.
 *
 * Zero dependencies. Zero I/O. Zero imports. This file is a reference implementation —
 * NOT wired into apps/api/src/server.ts or any live request path (that is a separate,
 * gated build once the front-door itself is built — see resolution.md §3).
 */

/** The ten strangler surfaces (REBUILD-MAP.md §Phase B) plus two explicit non-flip states. */
export type SurfaceId =
  | 'S1' // storefront-read
  | 'S2' // auth
  | 'S3' // catalog CRUD
  | 'S4' // media
  | 'S5' // orders/money 🔴
  | 'S6' // realtime WS 🔴
  | 'S7' // courier/dispatch 🔴
  | 'S8' // jobs/notifications
  | 'S9' // GDPR/compliance 🔴
  | 'S10' // platform-admin
  | 'UNMAPPED' // a REGISTERED route with no clean S1..S10 home (documented gap — see the map)
  | 'INFRA_NEVER_FLIPS'; // health/metrics/liveness — deliberately never routed through the flip mechanism

export interface RouteTemplate {
  /** HTTP method, upper-case. WS is never expressed as a template — see isWebSocketUpgrade. */
  readonly method: string;
  /**
   * Path template. Always starts with '/'. Segments are either literal, `:name`
   * (matches exactly one non-empty segment), or a single trailing `*` (matches
   * one-or-more remaining segments; only ever the LAST segment).
   */
  readonly template: string;
  readonly surface: SurfaceId;
  /** `file.ts:line` — where this route is actually registered (ground truth, not a guess). */
  readonly source: string;
  /** Set only when this row needed a judgment call — never silently made without one. */
  readonly flag?: string;
}

export interface MatchResult {
  readonly surface: SurfaceId | 'NODE_UNMAPPED';
  readonly matched: boolean;
  readonly template?: RouteTemplate;
  /** Every collided template, only populated when matched === false due to ambiguity. */
  readonly collisions?: readonly RouteTemplate[];
  readonly reason: string;
}

function splitPath(path: string): string[] {
  const withoutQuery = path.split('?')[0] ?? '';
  return withoutQuery.split('/').filter((seg) => seg.length > 0);
}

/**
 * Positional, literal, segment-count-exact match (never a prefix match). This is
 * the entire fix for CRIT-1: two templates that share every segment up to a
 * variable UUID but diverge at the very next literal segment (`orders` vs
 * `gdpr-requests`) are, by construction, never confused — the comparison is
 * segment-by-segment, not "does the path start with X".
 */
function segmentsMatch(templateSegs: readonly string[], pathSegs: readonly string[]): boolean {
  for (let i = 0; i < templateSegs.length; i++) {
    const t = templateSegs[i]!;
    if (t === '*') {
      // Wildcard must be the last template segment; it consumes 1+ remaining path segments.
      return i === templateSegs.length - 1 && pathSegs.length > i;
    }
    if (i >= pathSegs.length) return false;
    if (t.startsWith(':')) continue; // named param — matches any single non-empty segment, never a literal
    if (t !== pathSegs[i]) return false;
  }
  return pathSegs.length === templateSegs.length;
}

/**
 * Core matcher: full (method, path-template) match, never a prefix. Pure function —
 * same input always yields the same output, no I/O, no shared mutable state.
 */
export function matchSurface(
  method: string,
  path: string,
  templates: readonly RouteTemplate[],
): MatchResult {
  const upperMethod = method.toUpperCase();
  const pathSegs = splitPath(path);
  const matches = templates.filter(
    (t) => t.method.toUpperCase() === upperMethod && segmentsMatch(splitPath(t.template), pathSegs),
  );

  if (matches.length === 0) {
    return {
      surface: 'NODE_UNMAPPED',
      matched: false,
      reason: `no template matched ${upperMethod} ${path} — fail-closed to Node (logged as unmapped)`,
    };
  }

  if (matches.length > 1) {
    // Structurally should never happen (disjointness is a static test invariant below),
    // but the runtime NEVER trusts the static proof alone for a red-line surface split —
    // an ambiguous match fails closed exactly like a missing one, and is reported loudly.
    return {
      surface: 'NODE_UNMAPPED',
      matched: false,
      collisions: matches,
      reason:
        `AMBIGUOUS: ${matches.length} templates matched ${upperMethod} ${path} ` +
        `(${matches.map((m) => `${m.surface}:${m.template}`).join(', ')}) — ` +
        `fail-closed to Node, never silently co-flip surfaces`,
    };
  }

  const template = matches[0]!;
  return {
    surface: template.surface,
    matched: true,
    template,
    reason: `matched ${template.method} ${template.template} (${template.source}) → ${template.surface}`,
  };
}

/**
 * True iff this request is an HTTP Upgrade for a WebSocket. `ws`'s
 * `WebSocketServer` is mounted with no `path` filter (apps/api/src/websocket.ts:192),
 * so in the REAL system this predicate — not any path template — is what actually
 * decides "does this request belong to S6". Checked case-insensitively on the header
 * name (Node lower-cases incoming header names, but callers may pass either).
 */
export function isWebSocketUpgrade(headers: Readonly<Record<string, string | string[] | undefined>>): boolean {
  const raw = headers['upgrade'] ?? headers['Upgrade'];
  const value = Array.isArray(raw) ? raw[0] : raw;
  return typeof value === 'string' && value.toLowerCase() === 'websocket';
}

/**
 * Full front-door decision: checks the WS special case FIRST (protocol-level,
 * ignores the path entirely — matching the real `ws` library's behavior), then
 * falls through to the ordinary (method, path-template) matcher for everything else.
 */
export function matchSurfaceForRequest(
  method: string,
  path: string,
  headers: Readonly<Record<string, string | string[] | undefined>>,
  templates: readonly RouteTemplate[],
): MatchResult {
  if (isWebSocketUpgrade(headers)) {
    return {
      surface: 'S6',
      matched: true,
      reason:
        'HTTP Upgrade: websocket — matched by protocol (Upgrade header), not path. ' +
        'ws.WebSocketServer has no `path` filter, so ANY path upgrades into S6; a ' +
        'path-template rule for "/ws" would be a phantom precision the real server ' +
        'does not enforce.',
    };
  }
  return matchSurface(method, path, templates);
}
