const KEY = 'dowiz-markov';

export function createMarkov() {
  let transitions = {};
  let currentState = null;
  let stateStart = 0;

  function load() {
    try { const raw = localStorage.getItem(KEY); if (raw) transitions = JSON.parse(raw); } catch {}
  }
  function save() {
    try { localStorage.setItem(KEY, JSON.stringify(transitions)); } catch {}
  }

  function states() { return Object.keys(transitions); }

  function observe(state) {
    const now = performance.now();
    if (currentState && currentState !== state) {
      const elapsed = now - stateStart;
      const key = `${currentState}→${state}`;
      transitions[key] = transitions[key] || { from: currentState, to: state, count: 0, totalMs: 0, avgMs: 0 };
      transitions[key].count++;
      transitions[key].totalMs += elapsed;
      transitions[key].avgMs = transitions[key].totalMs / transitions[key].count;
    }
    currentState = state;
    stateStart = now;
    save();
  }

  function getFriction(state) {
    const entries = Object.entries(transitions).filter(([k]) => k.startsWith(state + '→'));
    if (entries.length === 0) return null;
    const totalCount = entries.reduce((s, [, v]) => s + v.count, 0);
    const probs = entries.map(([k, v]) => ({ to: k.split('→')[1], prob: v.count / totalCount, avgMs: v.avgMs }));
    probs.sort((a, b) => b.prob - a.prob);
    return { state, transitions: probs, totalObservations: totalCount };
  }

  function detectFreeze(state, threshold = 3) {
    const f = getFriction(state);
    if (!f) return null;
    const expected = f.transitions[0]?.avgMs || 3000;
    const actual = performance.now() - stateStart;
    return actual > expected * threshold ? { state, expected, actual, ratio: actual / expected } : null;
  }

  load();
  return { observe, getFriction, detectFreeze, states };
}
