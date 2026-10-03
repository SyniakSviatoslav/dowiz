// Row 58: the scheduled watchers on GitHub actually run and pass. Latest run
// of each watched workflow: completed, success, and younger than its schedule
// allows. A red here is real (health-cron was red 8/8 behind Bot Fight Mode).
const WATCHED = { 'health-cron.yml': 26, 'heartbeat-monitor.yml': 2 }; // hours
export default async function ({ check, must, note }) {
  const bad = [];
  for (const [file, hours] of Object.entries(WATCHED)) {
    const r = await fetch(`https://api.github.com/repos/SyniakSviatoslav/dowiz/actions/workflows/${file}/runs?per_page=5`,
      { headers: { accept: 'application/vnd.github+json', 'x-github-api-version': '2022-11-28', 'user-agent': 'dowiz-live-proof' } });
    must(r.ok, `GitHub ${file}: ${r.status}`);
    const b = check('response_schema', await r.json());
    const done = b.workflow_runs.find(x => x.status === 'completed');
    if (!done) { bad.push(`${file}: no completed run`); continue; }
    const age = (Date.now() - Date.parse(done.created_at)) / 3600e3;
    const line = `${file} ${done.conclusion} ${age.toFixed(1)} h ago`;
    if (done.conclusion !== 'success' || age > hours) bad.push(line); else note(line);
  }
  must(!bad.length, bad.join('; '));
}
