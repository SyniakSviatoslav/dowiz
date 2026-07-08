// mergeCaptureOutput — combines a failed child-process's stdout/stderr into one string for
// downstream JSON.parse without corrupting valid JSON that a command still printed on stdout
// despite exiting nonzero (e.g. plane-guard.mjs --json on a hard FAIL).
//
// execSync's caught error exposes stdout/stderr as '' (not undefined) when a stream produced no
// bytes, so `stderr || message` treats a genuinely-empty stderr as "no stderr" and falls back to
// the generic "Command failed: …" message — appending it straight onto valid stdout JSON breaks
// JSON.parse. Only fall back to `message` when the process produced NO output on either stream
// (e.g. ENOENT before the child ever ran).
export function mergeCaptureOutput(stdout, stderr, message) {
  const so = stdout || '';
  const se = stderr || '';
  return so + se + (so || se ? '' : (message || ''));
}
