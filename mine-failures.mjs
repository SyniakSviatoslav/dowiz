#!/usr/bin/env node
// analytics/mine-failures.mjs
// Clusters failed runs by failure signature, then (1) writes one reflection per cluster for MemPalace
// (your Reflexion memory) and (2) records held-out candidates so the system gets tested against real
// past failures (Phase C/E). This is the feedback that makes the loop self-improving.
//
// Usage: node analytics/mine-failures.mjs [run-history.jsonl]

import { readFileSync, writeFileSync, appendFileSync } from "node:fs";

const runs = readFileSync(process.argv[2] || "run-history.jsonl", "utf8")
  .trim().split("\n").filter(Boolean).map((l) => JSON.parse(l));

const failed = runs.filter((r) => r.core?.passed === false);

// Signature = which hard gates failed (stable, deterministic). "unknown" if none recorded.
const sig = (r) => (r.core?.gating_failed?.length ? [...r.core.gating_failed].sort().join("+") : "unknown");

const clustersMap = failed.reduce((m, r) => ((m[sig(r)] ??= []).push(r), m), {});

const clusters = Object.entries(clustersMap).map(([signature, rs]) => ({
  signature,
  count: rs.length,
  example_run_ids: rs.slice(0, 3).map((r) => r.run_id),
  affected_files: [...new Set(rs.flatMap((r) => r.touched_files || []))],
  by_model: [...new Set(rs.map((r) => r.model).filter(Boolean))],
  by_category: [...new Set(rs.map((r) => r.category).filter(Boolean))],
})).sort((a, b) => b.count - a.count);

writeFileSync("clusters.json", JSON.stringify(clusters, null, 2));

// (1) Reflections for MemPalace. We WRITE A FILE in a clear shape; ingest it via MemPalace's MCP
//     write/diary tool (it auto-saves) — see README. We don't hardcode MemPalace's API here.
const reflections = clusters.map((c) => ({
  ts: new Date().toISOString(),
  type: "failure-cluster",
  signature: c.signature,
  count: c.count,
  note: `Recurring failure on gate(s) [${c.signature}] — ${c.count} run(s). Affected: ${c.affected_files.join(", ") || "n/a"}. ` +
        `Seen with model(s): ${c.by_model.join(", ") || "n/a"}. Before next attempt, check this gate first.`,
  affected_files: c.affected_files,
  example_run_ids: c.example_run_ids,
}));
writeFileSync("mempalace-reflections.jsonl", reflections.map((r) => JSON.stringify(r)).join("\n") + "\n");

// (2) Held-out candidates: representative failing run_ids per cluster. Join with your runtime's full
//     run dump (input/expected) by run_id to turn these into real eval cases for Phase C/E.
const heldOutCandidates = clusters.map((c) => ({ signature: c.signature, run_ids: c.example_run_ids }));
appendFileSync("held-out-candidates.json", JSON.stringify(heldOutCandidates, null, 2) + "\n");

// (3) Blast-radius (optional enrichment): for each cluster's affected_files, query the Graphify MCP
//     graph to find the impacted communities/modules, and annotate the reflection. We leave this as a
//     hook — call your Graphify MCP tool with c.affected_files and merge the result. See README.

console.log(`failed=${failed.length}  clusters=${clusters.length}`);
clusters.slice(0, 5).forEach((c) => console.log(`  [${c.signature}] x${c.count}  files: ${c.affected_files.slice(0, 3).join(", ")}`));
console.log("→ wrote clusters.json, mempalace-reflections.jsonl, held-out-candidates.json");
