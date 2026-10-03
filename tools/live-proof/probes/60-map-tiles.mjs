// Row 60: the storefront's map style, its TileJSON and one Durres tile are
// served by OpenFreeMap, and the live CSP lets the page load them.
const UA = { 'user-agent': 'dowiz-live-proof/0.1 (+https://dowiz.org)' };
export default async function ({ lib, check, must, note }) {
  const s = await fetch('https://tiles.openfreemap.org/styles/liberty', { headers: UA });
  must(s.ok, `style ${s.status}`);
  const style = check('response_schema', await s.json());
  const src = Object.values(style.sources).find(x => x.type === 'vector' && x.url);
  must(src, 'the style names no vector source url');
  const tj = await (await fetch(src.url, { headers: UA })).json();
  must(tj.tilejson === '3.0.0', `TileJSON ${tj.tilejson}`);
  const url = tj.tiles[0].replace('{z}', '14').replace('{x}', '9076').replace('{y}', '6122');
  const t = await fetch(url, { headers: UA });
  const bytes = (await t.arrayBuffer()).byteLength;
  must(t.status === 200 && /vnd\.mapbox-vector-tile|x-protobuf/.test(t.headers.get('content-type') || '') && bytes > 1024,
    `tile ${t.status} ${t.headers.get('content-type')} ${bytes} B`);
  const page = await fetch(`${lib.HOST}/`, { headers: { 'user-agent': lib.UA } });
  const csp = page.headers.get('content-security-policy') || '';
  must(csp.includes('tiles.openfreemap.org'), `live CSP does not name tiles.openfreemap.org (${csp.length} B policy)`);
  note(`style v${style.version}; tilejson 3.0.0; tile z14/9076/6122 ${bytes} B; CSP names the host`);
}
