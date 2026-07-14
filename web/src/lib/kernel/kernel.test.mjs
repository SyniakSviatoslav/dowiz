// kernel.test.mjs — VbM (Verified-by-Math) contract tests for the kernel wasm surface.
//
// These assert the EXACT flat-bridge layout the Rust kernel emits and the engine
// mirror-pins. If the kernel side changes the layout, this test fails loudly —
// that is the point: the web/ frontend must not silently desync from Rust math.
//
// Run:  node web/src/lib/kernel/kernel.test.mjs
import * as K from "../../../../kernel/pkg/dowiz_kernel.js";
import { routeProgress, spectralReport, fsmReport } from "./kernel_client.mjs";

let passed = 0, failed = 0;
function ok(name, cond, extra = "") {
  if (cond) { passed++; console.log(`  ok  ${name}`); }
  else { failed++; console.log(`FAIL  ${name} ${extra}`); }
}

// ── geo flat bridge ──
{
  const g = routeProgress(JSON.stringify([[0, 0], [10, 0]]), 5, 0);
  ok("geo: ok", g.ok === true, JSON.stringify(g));
  ok("geo: remainingM finite", Number.isFinite(g.remainingM));
  ok("geo: segment index int", Number.isInteger(g.segmentIndex), `seg=${g.segmentIndex}`);
  ok("geo: snapped lat finite", Number.isFinite(g.snappedLat));
  ok("geo: snapped lng finite", Number.isFinite(g.snappedLng));
}

// ── spectral flat bridge (2-cycle: eigenvalues ±1) ──
{
  const s = spectralReport(JSON.stringify([[0, 1], [1, 0]]));
  ok("spectral: not null", s !== null, "null");
  ok("spectral: rho == 1", s && Math.abs(s.rho - 1) < 1e-6, `rho=${s && s.rho}`);
  ok("spectral: gap == 0", s && Math.abs(s.gap) < 1e-6, `gap=${s && s.gap}`);
  ok("spectral: fiedler == 2", s && Math.abs(s.fiedler - 2) < 1e-6, `f=${s && s.fiedler}`);
  ok("spectral: drift Resonant", s && s.drift === "Resonant", `drift=${s && s.drift}`);
  ok("spectral: n == 2", s && s.n === 2, `n=${s && s.n}`);
  ok("spectral: e1 == +1", s && Math.abs(s.eigenvalues[0].re - 1) < 1e-6 && Math.abs(s.eigenvalues[0].im) < 1e-6);
  ok("spectral: e2 == -1", s && Math.abs(s.eigenvalues[1].re + 1) < 1e-6 && Math.abs(s.eigenvalues[1].im) < 1e-6);
}

// ── fail-closed: malformed matrices never return a phantom result ──
{
  ok("spectral: empty rejected", spectralReport("[]") === null);
  ok("spectral: non-square rejected", spectralReport("[[1,2,3],[4,5,6]]") === null);
  ok("spectral: bad JSON rejected", spectralReport("not json") === null);
  ok("geo: bad JSON rejected", routeProgress("not json", 1, 0).ok === false);
}

// ── FSM signature present + structured ──
{
  const r = fsmReport();
  ok("fsm: report object", r && typeof r === "object");
  ok("fsm: vertices >= 10", r && r.vertices >= 10, `n=${r && r.vertices}`);
  ok("fsm: is_acyclic", r && r.is_acyclic === true);
}

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
