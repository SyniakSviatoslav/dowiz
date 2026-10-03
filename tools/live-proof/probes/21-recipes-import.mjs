// Row 21: the owner's recipe import (CSV, services/catalogue/import/bulk).
// A dry run previews and writes nothing; ?apply=1 rewrites QA Veg roll's line
// (150 g rice -> 140 g); the dish's recipe reads 140; the line is put back to
// what was found by the same import, whatever happens.
const DISH = 'qa-veg-roll';
export default async function ({ lib, check, must, note }) {
  const bom = async () => ((await lib.own(`/api/owner/products?id=${DISH}&location_id=${lib.LOC}`)).body?.products || [])[0]?.bom || [];
  const before = await bom();
  const rice = before.find(l => l.supply === 'qa-rice');
  must(before.length === 1 && rice, `QA Veg roll's recipe is not the one qa-setup made: ${JSON.stringify(before).slice(0, 120)}`);
  const csv = g => `dish,ingredient,qty,unit,batch\nQA Veg roll,QA Sushi rice,${g},g,\n`;
  const imp = async (g, q = '') => {
    const r = await fetch(`${lib.HOST}/api/owner/recipes/import${q}${q ? '&' : '?'}location_id=${lib.LOC}`, { method: 'POST',
      headers: { authorization: `Bearer ${await lib.owner()}`, 'content-type': 'text/csv', 'user-agent': lib.UA }, body: csv(g) });
    let b; const t = await r.text(); try { b = JSON.parse(t); } catch { b = t; }
    return { status: r.status, body: b, text: t.slice(0, 200) };
  };
  const want = rice.qty === 140 ? 130 : 140;
  try {
    const dry = await imp(want);
    must(dry.status === 200 && !(dry.body?.rows || []).some(r => r.error), `dry run ${dry.status} ${dry.text}`);
    check('response_schema', dry.body);
    must((await bom())[0].qty === rice.qty, 'the dry run changed the recipe');
    const ap = await imp(want, '?apply=1');
    must(ap.status === 200 && ap.body?.written >= 1, `apply ${ap.status} ${ap.text}`);
    check('response_schema', ap.body);
    const after = await bom();
    must(after.length === 1 && after[0].supply === 'qa-rice' && after[0].qty === want, `after apply the recipe reads ${JSON.stringify(after.map(l => [l.supply, l.qty]))}`);
    note(`dry run wrote nothing; apply set rice ${rice.qty} -> ${want} g; read back ${after[0].qty}`);
  } finally {
    const back = await imp(rice.qty, '?apply=1');
    const now = await bom();
    must(back.status === 200 && now[0]?.qty === rice.qty, `RESTORE FAILED: QA Veg roll rice is ${now[0]?.qty}, was ${rice.qty}`);
  }
}
