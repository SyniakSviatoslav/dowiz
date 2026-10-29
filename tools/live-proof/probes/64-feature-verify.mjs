// Row 64 (LV-PCD-C): the published price root, proved from the outside. Reads only public objects:
// the venue's manifest and fragment on the CDN, the storefront's verify page and module.
import { fileURLToPath } from 'node:url';
const store = (p) => fileURLToPath(new URL(`../../../workers/api/public/store/${p}`, import.meta.url));
export default async function ({ check, must, note }) {
  const { pricesOf, certFor, verifyWith, readCert } = await import(store('verify.js'));
  const host = new URL(process.env.FLOWS_HOST);
  const slug = host.hostname.split('.')[0];
  const cdn = `https://cdn.${host.hostname.split('.').slice(1).join('.')}`; // store/shell.js cdnOrigin's rule
  const m = await (await fetch(`${cdn}/v/${slug}/manifest.json`)).json();
  check('response_schema', m);
  const frag = await (await fetch(`${cdn}/v/${slug}/${m.fragment}`)).json();
  const rows = pricesOf(frag);
  must(rows && rows.length === m.n, `the fragment has ${rows && rows.length} priced dishes, the manifest says n=${m.n}`);
  const page = await fetch(`${host.origin}/store/verify.html`);
  must(page.status === 200, `verify.html ${page.status}`);
  const wasm = await fetch(`${host.origin}/store/verify.wasm`);
  must(wasm.status === 200, `verify.wasm ${wasm.status}`);
  const { instance } = await WebAssembly.instantiate(await wasm.arrayBuffer(), {});
  let ok = 0;
  for (const [id, price] of rows) {
    const c = await certFor(frag, id, price);
    const v = verifyWith(instance.exports, c);
    must(v.status === 0 && v.root === m.root && readCert(c).root === m.root, `${id}: status ${v.status} root ${v.root} vs manifest ${m.root}`);
    ok++;
  }
  if (rows.length) {
    const c = await certFor(frag, rows[0][0], rows[0][1]);
    c[c.length - 1] ^= 1;
    must(verifyWith(instance.exports, c).status === 51, 'a moved path byte was not refused');
  }
  note(`${ok}/${rows.length} published prices climb to root ${m.root.slice(0, 16)}; ${m.sig ? `signed pk_id ${m.pk_id}` : 'UNSIGNED (VENUE_SIGN_SK not installed)'}`);
}
