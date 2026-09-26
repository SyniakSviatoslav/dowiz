// The console's "Agents (MCP)" sheet, per role: the URL, the owner's own key
// (API keys), the tools EACH role's key gets -- owner, waiter, kitchen,
// counter-manager, courier -- the setup for Claude Code, Claude Desktop, Codex, Gemini CLI
// and any MCP client, and every key the venue's people minted in their own
// apps, each revocable here. The pieces are /lib/mcp.js; this file composes
// them and does the owner's I/O through the functions `more.js` hands it, so
// it renders in node without the console's session.
import * as ui from '../lib/ui/index.js';
import { clientTabs, toolList, keyList, bindPanel, toolsOf } from '../lib/mcp.js';
import { mcpWords } from '../lib/mcp-words.js';

/// The anchors lesson O16j names, spelled `tour:` so tools/gates/learn.sh
/// finds each one in this file.
export const TOURS = {
  url: { tour: 'mcp.url' }, copyUrl: { tour: 'mcp.copy' }, role: { tour: 'mcp.role' }, tools: { tour: 'mcp.tools' },
  clients: { tour: 'mcp.clients' }, snippet: { tour: 'mcp.snippet' }, copySnippet: { tour: 'mcp.copySnippet' },
  keyRow: { tour: 'mcp.personKey' }, revoke: { tour: 'mcp.personRevoke' }, keys: { tour: 'mcp.keys' },
};
export const ROLES = ['owner', 'waiter', 'kitchen', 'counter-manager', 'courier'];

/// The tools of the picked role. PURE.
export function roleTools(lang, s){
  return toolList(toolsOf(s.describe, s.role), mcpWords(lang, 'owner'), TOURS);
}

/// The whole sheet body after `head`. PURE. s { url, describe, role, keys }
export function body(lang, s){
  const w = mcpWords(lang, 'owner');
  const role = ROLES.includes(s.role) ? s.role : 'owner';
  return `<div class="mcp">
    <p class="ui-hint">${ui.esc(w.hint)}</p>
    <p class="eyebrow mt-3">URL</p><pre class="mcp-code" id="mcpUrl" data-tour="${TOURS.url.tour}">${ui.esc(s.url)}</pre>
    <div class="btn-row">
      ${ui.button({ icon: 'copy', label: w.copy, cls: 'mcp-copy', attrs: { data: { copy: 'mcpUrl', tour: TOURS.copyUrl.tour } } })}
      ${ui.button({ id: 'mcpKeys', variant: 'primary', icon: 'key', label: w.ownerKey, attrs: { data: { tour: TOURS.keys.tour } } })}
    </div>
    <p class="eyebrow mt-3">${ui.esc(w.roleOf)}</p>
    ${ui.segmented({ id: 'mcpRole', label: w.roleOf, value: role, options: ROLES.map(r => ({ value: r, label: w.roleNames[r] })), attrs: { data: { tour: TOURS.role.tour } } })}
    <div id="mcpRoleTools">${roleTools(lang, { ...s, role })}</div>
    <p class="eyebrow mt-3">${ui.esc(w.clients)}</p>${clientTabs(s.url, null, w, TOURS)}
    <p class="eyebrow mt-3">${ui.esc(w.personKeys)}</p>
    <div id="mcpPeople">${keyList(s.keys || [], w, TOURS, true)}</div>
  </div>`;
}

/// Open it. d { lang(), sheet(html,{name}), root() -> the sheet element, head,
///             api(path), post(path, body), toast, openKeys, origin?, copy? }
export async function openMcp(d){
  const w = mcpWords(d.lang(), 'owner');
  const s = { url: `${d.origin || location.origin}/api/mcp`, describe: null, role: 'owner', keys: [] };
  const load = async () => {
    const [desc, k] = await Promise.all([d.api('/mcp').catch(() => null), d.api('/owner/mcp/keys').catch(() => null)]);
    s.describe = desc; s.keys = (k && k.keys) || [];
  };
  const draw = () => {
    d.sheet(`${d.head}${body(d.lang(), s)}`, { name: 'mcp' });
    const host = d.root().querySelector('.mcp');
    bindPanel(host, {
      copy: text => (d.copy || (x => navigator.clipboard.writeText(x)))(text).then(() => d.toast(w.copied), () => {}),
      onRevoke: async (id, holder) => {
        try { await d.post('/owner/mcp/keys/revoke', { id, holder }); d.toast(w.revoked); await load(); draw(); }
        catch (e) { d.toast(e.message || w.error); }
      },
    });
    ui.bindSegmented(host.querySelector('#mcpRole'), role => { s.role = role; host.querySelector('#mcpRoleTools').innerHTML = roleTools(d.lang(), s); });
    host.querySelector('#mcpKeys').addEventListener('click', () => d.openKeys());
  };
  await load();
  draw();
}
