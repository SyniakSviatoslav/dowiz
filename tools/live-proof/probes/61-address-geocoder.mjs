// Row 61: one reverse lookup at Nominatim, the way store/address.js makes it,
// within the OSMF usage policy (identifying UA, one request per run), and the
// live CSP lets the storefront call it.
export default async function ({ lib, check, must, note }) {
  const r = await fetch('https://nominatim.openstreetmap.org/reverse?format=jsonv2&lat=41.3231&lon=19.4414&zoom=18&accept-language=sq',
    { headers: { 'user-agent': 'dowiz-live-proof/0.1 (+https://dowiz.org; one request per run)' } });
  must(r.ok, `nominatim ${r.status}`);
  const b = check('response_schema', await r.json());
  const page = await fetch(`${lib.HOST}/`, { headers: { 'user-agent': lib.UA } });
  const csp = page.headers.get('content-security-policy') || '';
  must(/connect-src[^;]*nominatim\.openstreetmap\.org/.test(csp), 'live CSP connect-src does not name nominatim.openstreetmap.org');
  note(`place_id ${b.place_id}; ${b.display_name.slice(0, 60)}; CSP connect-src names nominatim`);
}
