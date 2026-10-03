// Row 30: the hub as an MCP server (protocol 2025-06-18). An owner key minted
// in the console opens initialize, tools/list and the `orders` tool, whose
// answer equals GET /api/owner/orders; once revoked the key is refused.
export default async function ({ lib, run, check, must, note }) {
  const mint = await lib.own(`/api/owner/apikeys?location_id=${lib.LOC}`, { label: `${run} mcp probe` });
  must(mint.status === 200 && /^dowiz_/.test(mint.body?.key || ''), `minting a key: ${mint.status} ${mint.text.slice(0, 100)}`);
  const key = mint.body.key;
  let revoked = false;
  const rpc = (id, method, params = {}) => lib.api('/api/mcp', { method: 'POST', token: key, body: { jsonrpc: '2.0', id, method, params } });
  try {
    const init = await rpc(1, 'initialize', { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'dowiz-live-proof', version: '0.1' } });
    must(init.status === 200, `initialize ${init.status} ${init.text.slice(0, 100)}`);
    check('response_schema', init.body);
    must(init.body.result?.protocolVersion, `initialize answered no protocolVersion: ${init.text.slice(0, 120)}`);
    const list = await rpc(2, 'tools/list');
    check('response_schema', list.body);
    const names = (list.body.result?.tools || []).map(t => t.name);
    must(names.includes('orders'), `tools/list has no orders tool (${names.length} tools)`);
    const call = await rpc(3, 'tools/call', { name: 'orders', arguments: {} });
    check('response_schema', call.body);
    must(!call.body.error && !call.body.result?.isError, `tools/call orders: ${call.text.slice(0, 160)}`);
    const text = (call.body.result?.content || []).map(c => c.text || '').join('');
    const viaTool = JSON.parse(text);
    const direct = await lib.ownerOrders();
    const ids = x => (Array.isArray(x) ? x : x.orders || []).map(o => o.id).sort().join(',');
    must(ids(viaTool) === ids(direct), `the orders tool lists ${ids(viaTool).split(',').length} ids, the owner route ${direct.length}`);
    const rv = await lib.own(`/api/owner/apikeys/revoke?location_id=${lib.LOC}`, { id: mint.body.id });
    must(rv.status === 200, `revoke ${rv.status}`);
    revoked = true;
    const after = await rpc(4, 'tools/list');
    must(after.status === 401, `the revoked key answered ${after.status}, not 401`);
    note(`initialize ${init.body.result.protocolVersion}; ${names.length} tools; orders tool == owner route (${direct.length} orders); revoked key 401`);
  } finally {
    if (!revoked) await lib.own(`/api/owner/apikeys/revoke?location_id=${lib.LOC}`, { id: mint.body.id });
  }
}
