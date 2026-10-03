// Row 39: the bootstrap door (bootstrap.rs seed) opens only to the box's
// secret. BOX ONLY: BOOTSTRAP_SECRET is never in GitHub, so the runner says
// NEEDS-KEY there. The correct secret with a body that is not a bundle must
// get past the secret and be refused as "bad bundle" (400) -- proving the
// secret matched WITHOUT writing the QA catalogue; a wrong secret and no
// secret both answer the same 404 as a route that does not exist.
export default async function ({ lib, check, must, note, NeedsKey }) {
  const secret = lib.creds.BOOTSTRAP_SECRET;
  if (!secret) throw new NeedsKey('BOOTSTRAP_SECRET is not on this machine (box-only)');
  const post = headers => lib.api('/api/bootstrap', { method: 'POST', headers, body: { not_a_bundle: true } });
  const none = await post({});
  must(none.status === 404, `no secret: ${none.status}`);
  const wrong = await post({ 'x-dowiz-bootstrap': secret.slice(0, -1) + (secret.endsWith('A') ? 'B' : 'A') });
  must(wrong.status === 404, `a wrong secret: ${wrong.status}`);
  must(wrong.text === none.text, 'a wrong secret is told apart from no route');
  const right = await post({ 'x-dowiz-bootstrap': secret });
  check('response_schema', right.body);
  must(right.status === 400 && /bad bundle/.test(right.text), `the right secret with a non-bundle: ${right.status} ${right.text.slice(0, 100)}`);
  note('no secret 404, wrong secret 404 (same body), right secret reaches the bundle parser (400 bad bundle); nothing written');
}
