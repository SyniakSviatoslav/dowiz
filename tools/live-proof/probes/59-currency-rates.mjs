// Row 59: the Worker's rates equal ExchangeRate-API's open v6 answer, rounded
// to parts per million (services/ordering/rates.rs), and are fresh.
export default async function ({ lib, check, must, note }) {
  const w = await lib.api('/api/public/rates?base=ALL');
  must(w.status === 200, `worker rates ${w.status}`);
  check('response_schema', w.body);
  const age = Date.now() - Date.parse(w.body.asOf);
  must(age >= 0 && age < 48 * 3600e3, `asOf ${w.body.asOf} is ${Math.round(age / 3600e3)} h old (limit 48)`);
  const er = await fetch('https://open.er-api.com/v6/latest/ALL', { headers: { 'user-agent': 'dowiz-live-proof/0.1 (+https://dowiz.org)' } });
  must(er.ok, `open.er-api.com ${er.status}`);
  const e = await er.json();
  must(e.result === 'success', `er-api result ${e.result}`);
  for (const k of ['EUR', 'USD']) {
    const want = Math.round(e.rates[k] * 1e6);
    must(w.body.ppm[k] === want, `${k}: worker ${w.body.ppm[k]} ppm, er-api ${want}`);
  }
  note(`EUR ${w.body.ppm.EUR} USD ${w.body.ppm.USD} ppm == er-api; asOf ${Math.round(age / 3600e3)} h old; stale false`);
}
