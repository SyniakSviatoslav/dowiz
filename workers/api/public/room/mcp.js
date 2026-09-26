// The room's "AI agent (MCP)" sheet: a waiter, the kitchen or a
// counter-manager connects their own agent, with a key minted HERE for this
// person (`POST /api/staff/mcp/keys`). The panel itself is /lib/mcp.js; this
// file only fetches, mints, revokes and redraws.
import * as ui from '../lib/ui/index.js';
import { renderPanel, bindPanel, toolsOf } from '../lib/mcp.js';
import { mcpWords } from '../lib/mcp-words.js';

/// The anchors lesson W11 names, spelled `tour:` so tools/gates/learn.sh finds
/// each one in this file.
export const TOURS = {
  url: { tour: 'agent.url' }, copyUrl: { tour: 'agent.copyUrl' }, label: { tour: 'agent.label' },
  mint: { tour: 'agent.mint' }, key: { tour: 'agent.key' }, keyRow: { tour: 'agent.keyRow' },
  revoke: { tour: 'agent.revoke' }, tools: { tour: 'agent.tools' }, clients: { tour: 'agent.clients' },
  snippet: { tour: 'agent.snippet' }, copySnippet: { tour: 'agent.copySnippet' },
};

/// The sheet's body for one state. PURE (app state in, markup out).
/// s { url, role, describe, keys, key }
export function body(lang, s){
  const w = mcpWords(lang, 'staff');
  return renderPanel({ w, url: s.url, tools: toolsOf(s.describe, s.role || 'waiter'), keys: s.keys || [], key: s.key || null, tours: TOURS, mint: true });
}

/// Open the sheet. `c` { api, lang, toast, role, closeLabel, copy?, doc?, origin? }
/// (`doc` and `origin` default to the page's; a test passes its own).
export async function openMcp(c){
  const w = mcpWords(c.lang(), 'staff');
  const s = { url: `${c.origin || location.origin}/api/mcp`, role: c.role, describe: null, keys: [], key: null };
  const sheet = ui.openSheet({ title: w.title, body: `<div id="mcpHost">${ui.skeleton({ shapes: ['block', 'block'] })}</div>`, closeLabel: c.closeLabel }, c.doc || document);
  const host = () => sheet.el.querySelector('#mcpHost');
  const draw = () => {
    const h = host();
    if (!h) return;
    // A FRESH element per draw, so a redraw never stacks a second listener.
    h.innerHTML = `<div>${body(c.lang(), s)}</div>`;
    bindPanel(h.children[0], {
      copy: text => (c.copy || (x => navigator.clipboard.writeText(x)))(text).then(() => c.toast(w.copied), () => {}),
      onMint: async label => {
        try { s.key = (await c.api('/staff/mcp/keys', { method: 'POST', body: { label } })).key; await load(); }
        catch (e) { c.toast(e.message || w.error); }
      },
      onRevoke: async id => {
        try { await c.api('/staff/mcp/keys/revoke', { method: 'POST', body: { id } }); s.key = null; c.toast(w.revoked); await load(); }
        catch (e) { c.toast(e.message || w.error); }
      },
    });
  };
  const load = async () => {
    const [d, k] = await Promise.all([c.api('/mcp').catch(() => null), c.api('/staff/mcp/keys').catch(() => null)]);
    s.describe = d; s.keys = (k && k.keys) || [];
    draw();
  };
  await load();
  return sheet;
}
