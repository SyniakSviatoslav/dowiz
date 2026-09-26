// "AI agent (MCP)": the one panel every app shows -- the owner's console, the
// room (waiter, kitchen, counter-manager) and the courier. PURE render + a
// small binder, so each app's own `mcp.js` only fetches and passes words.
//
// What it shows: the URL, how this person gets a key, the tools THEIR key
// gets (from `GET /api/mcp` -> roles), and ready setup for Claude Code,
// Claude Desktop, OpenAI Codex, Gemini CLI and any other MCP client.
//
// THE KEY IS NEVER IN A SNIPPET unless the person minted it on this screen a
// moment ago (the hub keeps only a hash and cannot show it again). Otherwise
// the snippet carries PLACEHOLDER, which no client will mistake for a key.
//
// Client setup, as documented (checked 2026-09-25):
//   Claude Code: `claude mcp add --transport http <name> <url> --header "..."`
//   Claude Desktop: its config file runs local (stdio) servers, so a remote
//     server with a header goes through `mcp-remote` (npm), whose README passes
//     the header via an env var to keep the space out of the args.
//   Codex: ~/.codex/config.toml `[mcp_servers.<name>]` with `url` and
//     `bearer_token_env_var` (learn.chatgpt.com/docs/extend/mcp, the page
//     developers.openai.com/codex/mcp redirects to).
//   Gemini CLI: `gemini mcp add --transport http <name> <url> -H "..."`, or
//     ~/.gemini/settings.json `mcpServers.<name>` with `httpUrl` (Streamable
//     HTTP; `url` there means SSE) and `headers` (geminicli.com/docs/tools/mcp-server).
import * as ui from './ui/index.js';

export const PLACEHOLDER = '<YOUR_KEY>';
export const CLIENTS = ['claudeCode', 'claudeDesktop', 'codex', 'gemini', 'generic'];

/// The five setups for one URL. `key` only when it was just minted.
export function snippets(url, key){
  const k = key || PLACEHOLDER;
  const desktop = { mcpServers: { dowiz: { command: 'npx',
    args: ['-y', 'mcp-remote', url, '--header', 'Authorization:${DOWIZ_AUTH}'], env: { DOWIZ_AUTH: `Bearer ${k}` } } } };
  const gemini = { mcpServers: { dowiz: { httpUrl: url, headers: { Authorization: `Bearer ${k}` } } } };
  return {
    claudeCode: `claude mcp add --transport http dowiz ${url} --header "Authorization: Bearer ${k}"`,
    claudeDesktop: JSON.stringify(desktop, null, 2),
    codex: `# ~/.codex/config.toml\n[mcp_servers.dowiz]\nurl = "${url}"\nbearer_token_env_var = "DOWIZ_MCP_KEY"\n\n`
      + `# in the shell that starts codex\nexport DOWIZ_MCP_KEY="${k}"\n\n`
      + `# or: codex mcp add dowiz --url ${url} --bearer-token-env-var DOWIZ_MCP_KEY`,
    gemini: `gemini mcp add --transport http dowiz ${url} -H "Authorization: Bearer ${k}"\n\n`
      + `# or in ~/.gemini/settings.json\n${JSON.stringify(gemini, null, 2)}`,
    generic: `URL: ${url}\nTransport: Streamable HTTP (JSON-RPC over POST)\nHeader: Authorization: Bearer ${k}`,
  };
}

/// `tours` maps a control to `{ tour: '<id>' }` (the app's own anchor names);
/// a missing one draws no anchor.
const dt = (tours, k) => (tours && tours[k] ? { data: { tour: tours[k].tour } } : {});

const code = (id, text, tours, k) => `<pre class="mcp-code" id="${ui.esc(id)}"${tours && tours[k] ? ` data-tour="${ui.esc(tours[k].tour)}"` : ''}>${ui.esc(text)}</pre>`;

/// The tool list of one role: names in mono, what each does beneath.
export function toolList(tools, w, tours){
  if (!tools || !tools.length) return ui.emptyState({ icon: 'cube-3d-sphere', title: w.noTools });
  const rows = tools.map(x => ui.row({ title: x.name, sub: x.description || '', cls: 'mcp-tool' }));
  return `<p class="eyebrow mt-3">${ui.esc(String(tools.length))} ${ui.esc(w.tools)}</p>${ui.list(rows, { label: w.tools, attrs: dt(tours, 'tools') })}`;
}

/// The client tabs and their snippets.
export function clientTabs(url, key, w, tours){
  const s = snippets(url, key);
  const hint = { claudeCode: w.hintClaudeCode, claudeDesktop: w.hintClaudeDesktop, codex: w.hintCodex, gemini: w.hintGemini, generic: w.hintGeneric };
  const tabs = ui.tabs({ id: 'mcpClients', label: w.clients, items: CLIENTS.map(c => ({ id: `mcp-${c}`, label: w[c] })) });
  const panels = CLIENTS.map((c, i) => `<div id="mcp-${c}-panel" role="tabpanel" aria-labelledby="tab-mcp-${c}"${i ? ' hidden' : ''}>
      <p class="ui-hint">${ui.esc(hint[c])}</p>${code(`mcpSnip-${c}`, s[c], i ? null : tours, 'snippet')}
      ${ui.button({ icon: 'copy', label: w.copy, cls: 'mcp-copy', attrs: { data: { copy: `mcpSnip-${c}`, ...(i ? {} : dt(tours, 'copySnippet').data) } } })}</div>`).join('');
  const keyNote = key ? ui.alert({ tone: 'warning', label: w.shownOnce }) : `<p class="ui-hint">${ui.esc(w.placeholderNote.replace('{k}', PLACEHOLDER))}</p>`;
  return `<div class="mcp-clients"${tours && tours.clients ? ` data-tour="${ui.esc(tours.clients.tour)}"` : ''}>${tabs}</div>${keyNote}${panels}`;
}

/// One key row: label, when it ends, revoke.
function keyRow(k, w, tours, showWho){
  const who = showWho ? `${k.role || k.holder} · ${k.name || k.person || ''} · ` : '';
  return ui.row({ title: k.label, sub: `${who}${w.until} ${new Date(k.expiresMs).toISOString().slice(0, 10)}`, attrs: dt(tours, 'keyRow'),
    trailing: ui.button({ variant: 'ghost', icon: 'trash', label: w.revoke, attrs: { data: { revoke: k.id, holder: k.holder || '', ...dt(tours, 'revoke').data } } }) });
}

export function keyList(keys, w, tours, showWho){
  if (!keys || !keys.length) return `<p class="ui-hint">${ui.esc(w.noKeys)}</p>`;
  return ui.list(keys.map(k => keyRow(k, w, tours, showWho)), { label: w.yourKeys });
}

/// The whole panel for a person who mints their own key.
/// o { w, url, tools, key, keys, tours, mint (bool) }
export function renderPanel(o){
  const { w, url, tours } = o;
  const mint = o.mint ? `<p class="eyebrow mt-3">${ui.esc(w.keyHow)}</p>
    ${ui.inputRow({ id: 'mcpLabel', label: w.label, placeholder: w.label, attrs: dt(tours, 'label'),
      action: ui.button({ id: 'mcpMint', variant: 'primary', icon: 'key', label: w.mint, attrs: dt(tours, 'mint') }) })}` : '';
  const fresh = o.key ? `${code('mcpKey', o.key, tours, 'key')}${ui.button({ icon: 'copy', label: w.copyKey, cls: 'mcp-copy', attrs: { data: { copy: 'mcpKey' } } })}` : '';
  return `<div class="mcp">
    <p class="ui-hint">${ui.esc(w.hint)}</p>
    <p class="eyebrow mt-3">URL</p>${code('mcpUrl', url, tours, 'url')}
    ${ui.button({ icon: 'copy', label: w.copy, cls: 'mcp-copy', attrs: { data: { copy: 'mcpUrl', ...dt(tours, 'copyUrl').data } } })}
    ${mint}${fresh}
    ${o.keys ? `<p class="eyebrow mt-3">${ui.esc(w.yourKeys)}</p><div id="mcpKeys">${keyList(o.keys, w, tours, false)}</div>` : ''}
    ${toolList(o.tools, w, tours)}
    <p class="eyebrow mt-3">${ui.esc(w.clients)}</p>${clientTabs(url, o.key, w, tours)}
  </div>`;
}

/// The tools of one role from `GET /api/mcp`'s `roles`, or none.
export function toolsOf(describe, role){
  const r = describe && describe.roles;
  return (r && Array.isArray(r[role]) && r[role]) || [];
}

/// Wire a rendered panel: tabs, copy buttons, mint, revoke. `copy(text)` is
/// the clipboard (injected, so a test needs no browser).
export function bindPanel(root, { copy, onMint, onRevoke } = {}){
  if (!root) return;
  ui.bindTabs(root.querySelector('#mcpClients'), null, root.ownerDocument);
  root.addEventListener('click', e => {
    const b = e.target.closest && e.target.closest('button');
    if (!b || !root.contains(b)) return;
    if (b.dataset.copy) {
      const src = root.querySelector('#' + b.dataset.copy);
      if (src && copy) copy(src.textContent);
    } else if (b.dataset.revoke && onRevoke) onRevoke(b.dataset.revoke, b.dataset.holder || '');
    else if (b.id === 'mcpMint' && onMint) {
      const inp = root.querySelector('#mcpLabel');
      onMint(inp ? String(inp.value || '').trim() : '');
    }
  });
}
