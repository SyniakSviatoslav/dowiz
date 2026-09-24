// Skeletons: "something is coming; this is its shape".
//
// Not decoration: the alternative on the courier's first paint was asserting
// "you are offline" before the hub had answered. It carries aria-busy and a
// label so a screen reader hears "Loading", not silence. The shimmer stops
// under reduced motion (ui.css); the shape stays, which is the meaning.
import { cx, attrs, merge, text } from './core.js';

const SHAPES = ['line', 'title', 'block', 'button', 'row', 'card'];

/// @param o { shapes: ['title','block','block','button'], label, id, cls }
///   or { shape:'row', count:3 }. Unknown shapes throw.
export function skeleton(o = {}){
  const shapes = o.shapes || Array.from({ length: o.count || 1 }, () => o.shape || 'line');
  for (const s of shapes) if (!SHAPES.includes(s)) throw new Error(`ui.skeleton: unknown shape ${s}`);
  const a = merge({ id: o.id, class: cx('ui-skel-wrap', o.cls), 'aria-busy': 'true',
                    role: 'status', 'aria-label': text(o.label) || null }, o.attrs);
  return `<div${attrs(a)}>${shapes.map(s => `<div class="ui-skel ui-skel--${s}"></div>`).join('')}</div>`;
}
