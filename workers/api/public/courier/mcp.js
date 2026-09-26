// The courier's "AI agent (MCP)" panel: the courier connects their own agent
// with a key minted HERE for them (`POST /api/courier/mcp/keys`). The agent
// sees what the courier's app sees -- their runs and the open pool -- and
// nothing of another courier's. The panel is /lib/mcp.js; this file fetches,
// mints, revokes and redraws, inside the app's own sheet (`panel`).
import * as ui from '../lib/ui/index.js';
import { renderPanel, bindPanel, toolsOf } from '../lib/mcp.js';
import { mcpWords } from '../lib/mcp-words.js';

/// The anchors lesson C7 names, spelled `tour:` so tools/gates/learn.sh finds
/// each one in this file.
export const TOURS = {
  url: { tour: 'agent.url' }, copyUrl: { tour: 'agent.copyUrl' }, label: { tour: 'agent.label' },
  mint: { tour: 'agent.mint' }, key: { tour: 'agent.key' }, keyRow: { tour: 'agent.keyRow' },
  revoke: { tour: 'agent.revoke' }, tools: { tour: 'agent.tools' }, clients: { tour: 'agent.clients' },
  snippet: { tour: 'agent.snippet' }, copySnippet: { tour: 'agent.copySnippet' },
};

/// Where the courier stands still (offline, waiting), beside the lessons.
export const mcpButton = () => ui.button({ id: 'mcpOpen', variant: 'ghost', icon: 'cube-3d-sphere', label: { t: 'agent' }, block: true, attrs: { data: { tour: 'panel.agent' } } });

/// The panel for one state. PURE. s { url, describe, keys, key }
export function body(lang, s){
  const w = mcpWords(lang, 'courier');
  return renderPanel({ w, url: s.url, tools: toolsOf(s.describe, 'courier'), keys: s.keys || [], key: s.key || null, tours: TOURS, mint: true });
}

/// c { api, lang(), toast, panel(title, html), root() -> the element the panel
///     is drawn in, origin?, copy? }
export async function openMcp(c){
  const w = mcpWords(c.lang(), 'courier');
  const s = { url: `${c.origin || location.origin}/api/mcp`, describe: null, keys: [], key: null };
  const draw = async () => {
    await c.panel(w.title, `<div>${body(c.lang(), s)}</div>`);
    const host = c.root() && c.root().querySelector('.mcp');
    bindPanel(host, {
      copy: text => (c.copy || (x => navigator.clipboard.writeText(x)))(text).then(() => c.toast(w.copied), () => {}),
      onMint: async label => {
        try { s.key = (await c.api('/courier/mcp/keys', { method: 'POST', body: JSON.stringify({ label }) })).key; await load(); }
        catch (e) { c.toast(e.message || w.error); }
      },
      onRevoke: async id => {
        try { await c.api('/courier/mcp/keys/revoke', { method: 'POST', body: JSON.stringify({ id }) }); s.key = null; c.toast(w.revoked); await load(); }
        catch (e) { c.toast(e.message || w.error); }
      },
    });
  };
  const load = async () => {
    const [d, k] = await Promise.all([c.api('/mcp').catch(() => null), c.api('/courier/mcp/keys').catch(() => null)]);
    s.describe = d; s.keys = (k && k.keys) || [];
    await draw();
  };
  await load();
}
