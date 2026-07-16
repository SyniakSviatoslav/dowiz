/* tslint:disable */
/* eslint-disable */
/**
 * Related docs: every doc sharing ≥1 tag with `id` (sorted, excludes self).
 * Browser renders the returned id list as a field.
 * @param {string} id
 * @param {string} docs_json
 * @returns {(string)[]}
 */
export function related_docs(id: string, docs_json: string): (string)[];
/**
 * @param {number} count
 * @param {Float64Array} edges
 * @returns {Float32Array}
 */
export function vertex_field(count: number, edges: Float64Array): Float32Array;
/**
 * @param {Float64Array} circles
 * @param {number} w
 * @param {number} h
 * @param {number} steps
 * @returns {Uint8Array}
 */
export function compose_field(circles: Float64Array, w: number, h: number, steps: number): Uint8Array;
/**
 * Tag lookup (case-insensitive). Returns the sorted bucket of doc ids tagged
 * with `tag` — a field the browser lists, not a DOM tree.
 * @param {string} tag
 * @param {string} docs_json
 * @returns {(string)[]}
 */
export function lookup_tag(tag: string, docs_json: string): (string)[];
/**
 * Knowledge Map: grouped `## <tag>` sections over the corpus. Returns the
 * kernel's deterministic MAP markdown; the browser renders it as a field.
 * @param {string} docs_json
 * @returns {string}
 */
export function knowledge_map(docs_json: string): string;
/**
 * Stateful field-frame integrator exposed to JS for a live rAF loop. The
 * browser calls `step()` once per animation frame to advance the physics and
 * `frame()` to blit the returned RGBA — ALL math stays in the kernel/engine.
 */
export class FieldSim {
  free(): void;
  /**
   * Build a sim from a flat circle list `[cx,cy,r, ...]`, rasterize the SDF
   * source `S`, and allocate a zeroed field `U`. The browser only blits.
   * @param {Float64Array} circles
   * @param {number} w
   * @param {number} h
   */
  constructor(circles: Float64Array, w: number, h: number);
  /**
   * Advance one physics timestep. The rAF loop calls this per frame.
   */
  step(): void;
  /**
   * RGBA8 frame the canvas paints (`len == w*h*4`, never NaN bytes).
   * @returns {Uint8Array}
   */
  frame(): Uint8Array;
  /**
   * Frame width (JS sizes the `ImageData`).
   * @returns {number}
   */
  width(): number;
  /**
   * Frame height (JS sizes the `ImageData`).
   * @returns {number}
   */
  height(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_fieldsim_free: (a: number, b: number) => void;
  readonly compose_field: (a: number, b: number, c: number, d: number, e: number) => Array;
  readonly fieldsim_frame: (a: number) => Array;
  readonly fieldsim_height: (a: number) => number;
  readonly fieldsim_new: (a: number, b: number, c: number, d: number) => number;
  readonly fieldsim_step: (a: number) => void;
  readonly fieldsim_width: (a: number) => number;
  readonly knowledge_map: (a: number, b: number) => Array;
  readonly lookup_tag: (a: number, b: number, c: number, d: number) => Array;
  readonly related_docs: (a: number, b: number, c: number, d: number) => Array;
  readonly vertex_field: (a: number, b: number, c: number) => Array;
  readonly __wbindgen_export_0: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_drop_slice: (a: number, b: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
